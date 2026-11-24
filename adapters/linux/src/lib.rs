//! Linux read-only sensor adapter (P1-05).
//!
//! Reads the kernel `hwmon` sysfs interface (`/sys/class/hwmon/hwmonN/...`),
//! which covers CPU (k10temp), motherboard, and many fan tachometers, plus
//! AMDGPU sensors exposed through hwmon on Bazzite. It requires no root for the
//! standard sysfs reads (ARCHITECTURE.md §8; PHASE-1-SPEC §9).
//!
//! The sysfs traversal is gated to Linux, but all parsing is pure and tested on
//! every platform so the normalized contract is verified in CI on Windows too.

use shookshmam_hardware_core::{HardwareError, SensorProvider};
use shookshmam_sensor_model::{DeviceDescriptor, SensorReading};

pub const SOURCE: &str = "linux-hwmon";

pub struct HwmonProvider;

impl HwmonProvider {
    pub fn new() -> Self {
        HwmonProvider
    }
}

impl Default for HwmonProvider {
    fn default() -> Self {
        Self::new()
    }
}

/// Parse a hwmon `temp*_input` value (millidegrees Celsius) into Celsius.
pub fn parse_milli_celsius(raw: &str) -> Option<f64> {
    raw.trim().parse::<i64>().ok().map(|m| m as f64 / 1000.0)
}

/// Parse a hwmon `in*_input` value (millivolts) into Volts.
pub fn parse_milli_volts(raw: &str) -> Option<f64> {
    raw.trim().parse::<i64>().ok().map(|m| m as f64 / 1000.0)
}

/// Parse a hwmon `fan*_input` value (RPM, already in RPM).
pub fn parse_rpm(raw: &str) -> Option<f64> {
    raw.trim().parse::<i64>().ok().map(|v| v as f64)
}

/// Parse a hwmon `power*_input` value (microwatts) into Watts.
pub fn parse_micro_watts(raw: &str) -> Option<f64> {
    raw.trim().parse::<i64>().ok().map(|u| u as f64 / 1_000_000.0)
}

/// Parse a hwmon `mem_info_*` value (bytes) for GPU VRAM.
pub fn parse_vram_bytes(raw: &str) -> Option<f64> {
    raw.trim().parse::<i64>().ok().map(|b| b as f64)
}

/// Parse GPU utilization from `gpu_busy_percent` (0-100).
pub fn parse_gpu_busy_percent(raw: &str) -> Option<f64> {
    raw.trim().parse::<i64>().ok().map(|p| p as f64)
}

/// Parse a pp_dpm_* line and extract the active clock (marked with *) in MHz.
/// Format example: "0: 213Mhz *\n1: 500Mhz".
/// Returns the frequency value in MHz.
pub fn parse_active_dpm_clock(raw: &str) -> Option<f64> {
    for line in raw.lines() {
        if line.contains('*') {
            // Extract the frequency value before "Mhz" or "Mhz *"
            if let Some(freq_part) = line.split_whitespace().find(|s| s.contains("Mhz")) {
                let freq_str = freq_part.trim_end_matches("Mhz").trim_end_matches('*');
                if let Ok(mhz) = freq_str.parse::<f64>() {
                    return Some(mhz);
                }
            }
        }
    }
    None
}

impl SensorProvider for HwmonProvider {
    fn provider_id(&self) -> &'static str {
        SOURCE
    }

    #[cfg(target_os = "linux")]
    fn discover(&self) -> Result<Vec<DeviceDescriptor>, HardwareError> {
        sysfs::discover()
    }

    #[cfg(not(target_os = "linux"))]
    fn discover(&self) -> Result<Vec<DeviceDescriptor>, HardwareError> {
        Err(HardwareError::NotImplemented)
    }

    #[cfg(target_os = "linux")]
    fn read_snapshot(&self, device_id: &str) -> Result<Vec<SensorReading>, HardwareError> {
        sysfs::read_snapshot(device_id)
    }

    #[cfg(not(target_os = "linux"))]
    fn read_snapshot(&self, _device_id: &str) -> Result<Vec<SensorReading>, HardwareError> {
        Err(HardwareError::NotImplemented)
    }
}

#[cfg(target_os = "linux")]
mod sysfs {
    use super::{parse_milli_celsius, parse_rpm, parse_vram_bytes, parse_gpu_busy_percent, parse_active_dpm_clock, SOURCE};
    use shookshmam_hardware_core::{now_unix_ms, HardwareError};
    use shookshmam_sensor_model::{
        DeviceDescriptor, DeviceKind, SensorCategory, SensorReading, SensorStatus,
    };
    use std::fs;
    use std::path::{Path, PathBuf};

    const HWMON_ROOT: &str = "/sys/class/hwmon";

    fn read_trimmed(path: &Path) -> Option<String> {
        fs::read_to_string(path).ok().map(|s| s.trim().to_string())
    }

    fn is_amdgpu_hwmon(name: &str) -> bool {
        name == "amdgpu"
    }

    fn resolve_pci_device_name(vendor_id: u16, device_id: u16) -> Option<String> {
        let pci_db_path = Path::new("/usr/share/hwdata/pci.ids");
        if !pci_db_path.exists() {
            return None;
        }

        let content = std::fs::read_to_string(pci_db_path).ok()?;
        let vendor_hex = format!("{:04x}", vendor_id);
        let device_hex = format!("{:04x}", device_id);

        let mut in_vendor = false;
        for line in content.lines() {
            // Skip empty lines and comments
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            // Check if this is the vendor line we're looking for
            if !in_vendor && line.starts_with(&vendor_hex) && !line.starts_with('\t') {
                in_vendor = true;
                continue;
            }

            // If we were in a vendor section, check for device entries
            if in_vendor {
                // Exit vendor section when we hit another vendor (hex ID at start, no tab)
                if !line.starts_with('\t') && line.chars().next().map(|c| c.is_ascii_hexdigit()).unwrap_or(false) {
                    break;
                }

                // Check for device entry (single tab, not double)
                if line.starts_with('\t') && !line.starts_with("\t\t") {
                    let device_line = line.trim_start_matches('\t');
                    if device_line.starts_with(&device_hex) {
                        let full_name = device_line[device_hex.len()..].trim().to_string();
                        if !full_name.is_empty() {
                            return Some(full_name);
                        }
                    }
                }
            }
        }

        None
    }

    fn get_gpu_model_name(hwmon_path: &Path) -> String {
        // Try to read GPU model from sysfs (DRM device path parent)
        // The device symlink points to the PCI device (e.g., ../../../0000:03:00.0)
        if let Ok(device_path) = std::fs::read_link(hwmon_path.join("device")) {
            // Convert to absolute path by joining with hwmon_path's parent
            let absolute_device_path = if device_path.is_absolute() {
                device_path
            } else {
                hwmon_path.parent().unwrap_or(Path::new("/")).join(device_path)
            };

            // Try to read vendor and device IDs from the PCI device directory
            if let Some(vendor_str) = read_trimmed(&absolute_device_path.join("vendor")) {
                if let Some(device_str) = read_trimmed(&absolute_device_path.join("device")) {
                    // Parse hex IDs
                    if let (Ok(vendor_id), Ok(device_id)) = (
                        u16::from_str_radix(vendor_str.trim_start_matches("0x"), 16),
                        u16::from_str_radix(device_str.trim_start_matches("0x"), 16),
                    ) {
                        // Try to resolve from pci.ids database
                        if let Some(model_name) = resolve_pci_device_name(vendor_id, device_id) {
                            return model_name;
                        }
                    }
                }
            }

            // Fallback: check for product_name in device info
            let device_info_paths = [
                "product_name",
                "name",
            ];
            for path_suffix in &device_info_paths {
                if let Some(model) = read_trimmed(&absolute_device_path.join(path_suffix)) {
                    if !model.is_empty() && model.len() < 100 {
                        return model;
                    }
                }
            }

            // Final fallback for AMD Radeon GPUs
            if let Some(raw_name) = read_trimmed(&absolute_device_path.join("driver/module/drivers")) {
                if raw_name.contains("amdgpu") {
                    return "AMD Radeon GPU".to_string();
                }
            }
        }

        "GPU".to_string()
    }

    fn classify(name: &str) -> DeviceKind {
        match name {
            "k10temp" | "coretemp" | "zenpower" => DeviceKind::Cpu,
            "amdgpu" | "nouveau" | "nvidia" => DeviceKind::Gpu,
            "nvme" | "drivetemp" => DeviceKind::Storage,
            _ => DeviceKind::Motherboard,
        }
    }

    pub fn discover() -> Result<Vec<DeviceDescriptor>, HardwareError> {
        let root = Path::new(HWMON_ROOT);
        if !root.exists() {
            return Err(HardwareError::DeviceUnavailable);
        }
        let mut out = Vec::new();
        let entries = fs::read_dir(root).map_err(|e| HardwareError::Io(e.to_string()))?;
        for entry in entries.flatten() {
            let dir = entry.path();
            let name = read_trimmed(&dir.join("name")).unwrap_or_else(|| "hwmon".to_string());
            let id = entry.file_name().to_string_lossy().into_owned();

            // For AMDGPU, try to get the actual GPU model name
            let display_name = if is_amdgpu_hwmon(&name) {
                get_gpu_model_name(&dir)
            } else {
                name.clone()
            };

            out.push(DeviceDescriptor {
                id,
                name: display_name,
                kind: classify(&name),
                source: SOURCE.into(),
            });
        }
        Ok(out)
    }

    pub fn read_snapshot(device_id: &str) -> Result<Vec<SensorReading>, HardwareError> {
        let dir: PathBuf = Path::new(HWMON_ROOT).join(device_id);
        if !dir.exists() {
            return Err(HardwareError::DeviceUnavailable);
        }
        let ts = now_unix_ms();
        let chip = read_trimmed(&dir.join("name")).unwrap_or_else(|| device_id.to_string());
        let mut readings = Vec::new();

        let entries = fs::read_dir(&dir).map_err(|e| HardwareError::Io(e.to_string()))?;
        for entry in entries.flatten() {
            let fname = entry.file_name().to_string_lossy().into_owned();

            // Temperatures: tempN_input (millidegrees C).
            if let Some(n) = fname.strip_prefix("temp").and_then(|s| s.strip_suffix("_input")) {
                if let Some(raw) = read_trimmed(&entry.path()) {
                    let label = read_trimmed(&dir.join(format!("temp{n}_label")))
                        .unwrap_or_else(|| format!("{chip} temp{n}"));
                    match parse_milli_celsius(&raw) {
                        Some(v) => readings.push(SensorReading::available(
                            format!("{device_id}.temp{n}"),
                            device_id,
                            label,
                            SensorCategory::Temperature,
                            v,
                            SOURCE,
                            ts,
                        )),
                        None => readings.push(SensorReading::without_value(
                            format!("{device_id}.temp{n}"),
                            device_id,
                            label,
                            SensorCategory::Temperature,
                            SOURCE,
                            SensorStatus::Error,
                            ts,
                        )),
                    }
                }
            }

            // Fan tachometers: fanN_input (RPM). Read-only; no PWM writes (ADR-0002).
            if let Some(n) = fname.strip_prefix("fan").and_then(|s| s.strip_suffix("_input")) {
                if let Some(raw) = read_trimmed(&entry.path()) {
                    if let Some(v) = parse_rpm(&raw) {
                        readings.push(SensorReading::available(
                            format!("{device_id}.fan{n}"),
                            device_id,
                            format!("{chip} fan{n}"),
                            SensorCategory::FanSpeed,
                            v,
                            SOURCE,
                            ts,
                        ));
                    }
                }
            }
        }

        // Add GPU-specific telemetry for AMDGPU hwmon devices.
        if is_amdgpu_hwmon(&chip) {
            add_gpu_telemetry(&dir, device_id, ts, &mut readings);
        }

        readings.sort_by(|a, b| a.id.cmp(&b.id));
        Ok(readings)
    }

    fn add_gpu_telemetry(dir: &Path, device_id: &str, ts: i64, readings: &mut Vec<SensorReading>) {
        // GPU attributes are in the parent PCI device directory, accessed via the "device" symlink.
        let device_dir = dir.join("device");

        // GPU utilization from gpu_busy_percent.
        if let Some(raw) = read_trimmed(&device_dir.join("gpu_busy_percent")) {
            match parse_gpu_busy_percent(&raw) {
                Some(v) => readings.push(SensorReading::available(
                    "gpu.utilization.core",
                    device_id,
                    "GPU Utilization",
                    SensorCategory::Utilization,
                    v,
                    SOURCE,
                    ts,
                )),
                None => readings.push(SensorReading::without_value(
                    "gpu.utilization.core",
                    device_id,
                    "GPU Utilization",
                    SensorCategory::Utilization,
                    SOURCE,
                    SensorStatus::Error,
                    ts,
                )),
            }
        } else {
            readings.push(SensorReading::without_value(
                "gpu.utilization.core",
                device_id,
                "GPU Utilization",
                SensorCategory::Utilization,
                SOURCE,
                SensorStatus::Unsupported,
                ts,
            ));
        }

        // VRAM usage: mem_info_vram_used (bytes).
        if let Some(raw) = read_trimmed(&device_dir.join("mem_info_vram_used")) {
            match parse_vram_bytes(&raw) {
                Some(v) => readings.push(SensorReading::available(
                    "gpu.memory.used",
                    device_id,
                    "GPU VRAM Used",
                    SensorCategory::Memory,
                    v,
                    SOURCE,
                    ts,
                )),
                None => readings.push(SensorReading::without_value(
                    "gpu.memory.used",
                    device_id,
                    "GPU VRAM Used",
                    SensorCategory::Memory,
                    SOURCE,
                    SensorStatus::Error,
                    ts,
                )),
            }
        } else {
            readings.push(SensorReading::without_value(
                "gpu.memory.used",
                device_id,
                "GPU VRAM Used",
                SensorCategory::Memory,
                SOURCE,
                SensorStatus::Unsupported,
                ts,
            ));
        }

        // VRAM total: mem_info_vram_total (bytes).
        if let Some(raw) = read_trimmed(&device_dir.join("mem_info_vram_total")) {
            match parse_vram_bytes(&raw) {
                Some(v) => readings.push(SensorReading::available(
                    "gpu.memory.total",
                    device_id,
                    "GPU VRAM Total",
                    SensorCategory::Memory,
                    v,
                    SOURCE,
                    ts,
                )),
                None => readings.push(SensorReading::without_value(
                    "gpu.memory.total",
                    device_id,
                    "GPU VRAM Total",
                    SensorCategory::Memory,
                    SOURCE,
                    SensorStatus::Error,
                    ts,
                )),
            }
        } else {
            readings.push(SensorReading::without_value(
                "gpu.memory.total",
                device_id,
                "GPU VRAM Total",
                SensorCategory::Memory,
                SOURCE,
                SensorStatus::Unsupported,
                ts,
            ));
        }

        // GPU core clock: pp_dpm_sclk (parse active entry marked with *).
        if let Some(raw) = read_trimmed(&device_dir.join("pp_dpm_sclk")) {
            match parse_active_dpm_clock(&raw) {
                Some(v) => readings.push(SensorReading::available(
                    "gpu.frequency.core",
                    device_id,
                    "GPU Core Clock",
                    SensorCategory::Frequency,
                    v,
                    SOURCE,
                    ts,
                )),
                None => readings.push(SensorReading::without_value(
                    "gpu.frequency.core",
                    device_id,
                    "GPU Core Clock",
                    SensorCategory::Frequency,
                    SOURCE,
                    SensorStatus::Error,
                    ts,
                )),
            }
        } else {
            readings.push(SensorReading::without_value(
                "gpu.frequency.core",
                device_id,
                "GPU Core Clock",
                SensorCategory::Frequency,
                SOURCE,
                SensorStatus::Unsupported,
                ts,
            ));
        }

        // GPU memory clock: pp_dpm_mclk (parse active entry marked with *).
        if let Some(raw) = read_trimmed(&device_dir.join("pp_dpm_mclk")) {
            match parse_active_dpm_clock(&raw) {
                Some(v) => readings.push(SensorReading::available(
                    "gpu.frequency.memory",
                    device_id,
                    "GPU Memory Clock",
                    SensorCategory::Frequency,
                    v,
                    SOURCE,
                    ts,
                )),
                None => readings.push(SensorReading::without_value(
                    "gpu.frequency.memory",
                    device_id,
                    "GPU Memory Clock",
                    SensorCategory::Frequency,
                    SOURCE,
                    SensorStatus::Error,
                    ts,
                )),
            }
        } else {
            readings.push(SensorReading::without_value(
                "gpu.frequency.memory",
                device_id,
                "GPU Memory Clock",
                SensorCategory::Frequency,
                SOURCE,
                SensorStatus::Unsupported,
                ts,
            ));
        }

        // GPU temperatures: map temp labels to standardized IDs.
        // temp1 (edge), temp2 (junction), temp3 (memory).
        for temp_num in 1..=3 {
            let temp_label_path = dir.join(format!("temp{}_label", temp_num));
            if let Some(label) = read_trimmed(&temp_label_path) {
                let standardized_id = match label.to_lowercase().as_str() {
                    s if s.contains("edge") => "gpu.temp.edge",
                    s if s.contains("junction") => "gpu.temp.junction",
                    s if s.contains("mem") => "gpu.temp.memory",
                    _ => continue,
                };

                let temp_input_path = dir.join(format!("temp{}_input", temp_num));
                if let Some(raw) = read_trimmed(&temp_input_path) {
                    match parse_milli_celsius(&raw) {
                        Some(v) => readings.push(SensorReading::available(
                            standardized_id,
                            device_id,
                            &label,
                            SensorCategory::Temperature,
                            v,
                            SOURCE,
                            ts,
                        )),
                        None => readings.push(SensorReading::without_value(
                            standardized_id,
                            device_id,
                            &label,
                            SensorCategory::Temperature,
                            SOURCE,
                            SensorStatus::Error,
                            ts,
                        )),
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_hwmon_units() {
        assert_eq!(parse_milli_celsius("45000"), Some(45.0));
        assert_eq!(parse_milli_celsius(" 38500 \n"), Some(38.5));
        assert_eq!(parse_milli_celsius("bad"), None);
        assert_eq!(parse_milli_volts("1200"), Some(1.2));
        assert_eq!(parse_rpm("1450"), Some(1450.0));
        assert_eq!(parse_micro_watts("65000000"), Some(65.0));
    }

    #[test]
    fn parses_vram_bytes() {
        assert_eq!(parse_vram_bytes("2147483648"), Some(2147483648.0));
        assert_eq!(parse_vram_bytes(" 8589934592 \n"), Some(8589934592.0));
        assert_eq!(parse_vram_bytes("0"), Some(0.0));
        assert_eq!(parse_vram_bytes("not a number"), None);
    }

    #[test]
    fn parses_gpu_busy_percent() {
        assert_eq!(parse_gpu_busy_percent("0"), Some(0.0));
        assert_eq!(parse_gpu_busy_percent("50"), Some(50.0));
        assert_eq!(parse_gpu_busy_percent(" 100 \n"), Some(100.0));
        assert_eq!(parse_gpu_busy_percent("invalid"), None);
    }

    #[test]
    fn parses_active_dpm_clock() {
        let pp_dpm_sclk = "0: 213Mhz *\n1: 500Mhz\n2: 1000Mhz";
        assert_eq!(parse_active_dpm_clock(pp_dpm_sclk), Some(213.0));

        let pp_dpm_mclk = "0: 100Mhz\n1: 500Mhz *\n2: 850Mhz";
        assert_eq!(parse_active_dpm_clock(pp_dpm_mclk), Some(500.0));

        let no_active = "0: 213Mhz\n1: 500Mhz\n2: 1000Mhz";
        assert_eq!(parse_active_dpm_clock(no_active), None);

        let empty = "";
        assert_eq!(parse_active_dpm_clock(empty), None);
    }

    #[test]
    fn parses_dpm_clock_various_formats() {
        // Test with various whitespace and line variations
        let variant1 = "0: 213Mhz *";
        assert_eq!(parse_active_dpm_clock(variant1), Some(213.0));

        let variant2 = "0: 213Mhz * ";
        assert_eq!(parse_active_dpm_clock(variant2), Some(213.0));

        let variant3 = "0: 1850Mhz *\n1: 2500Mhz";
        assert_eq!(parse_active_dpm_clock(variant3), Some(1850.0));
    }

    #[test]
    #[cfg(not(target_os = "linux"))]
    fn not_implemented_off_linux() {
        let p = HwmonProvider::new();
        assert!(matches!(p.discover(), Err(HardwareError::NotImplemented)));
    }

    #[test]
    #[cfg(target_os = "linux")]
    fn hwmon_device_path_resolution() {
        // Verify that hwmon3 (amdgpu) has a device symlink that points to the PCI device
        // containing GPU-specific attributes like gpu_busy_percent, mem_info_vram_used, pp_dpm_sclk.
        use std::fs;
        use std::path::Path;

        let hwmon_dir = Path::new("/sys/class/hwmon/hwmon3");
        if !hwmon_dir.exists() {
            return; // Test skipped if hwmon3 doesn't exist (not on AMDGPU system)
        }

        let name_path = hwmon_dir.join("name");
        if let Ok(name) = fs::read_to_string(&name_path) {
            if name.trim() != "amdgpu" {
                return; // hwmon3 is not amdgpu, skip test
            }
        } else {
            return; // Can't read name, skip test
        }

        // Verify device symlink exists and points to the PCI device
        let device_symlink = hwmon_dir.join("device");
        assert!(device_symlink.exists(), "hwmon3/device symlink should exist");

        // Verify GPU attributes are readable from device directory
        let gpu_busy = device_symlink.join("gpu_busy_percent");
        assert!(gpu_busy.exists(), "gpu_busy_percent should exist in device directory");

        let vram_used = device_symlink.join("mem_info_vram_used");
        assert!(vram_used.exists(), "mem_info_vram_used should exist in device directory");

        let vram_total = device_symlink.join("mem_info_vram_total");
        assert!(vram_total.exists(), "mem_info_vram_total should exist in device directory");

        let pp_sclk = device_symlink.join("pp_dpm_sclk");
        assert!(pp_sclk.exists(), "pp_dpm_sclk should exist in device directory");

        let pp_mclk = device_symlink.join("pp_dpm_mclk");
        assert!(pp_mclk.exists(), "pp_dpm_mclk should exist in device directory");

        // Verify they're readable
        assert!(fs::read_to_string(&gpu_busy).is_ok(), "gpu_busy_percent should be readable");
        assert!(fs::read_to_string(&vram_used).is_ok(), "mem_info_vram_used should be readable");
        assert!(fs::read_to_string(&vram_total).is_ok(), "mem_info_vram_total should be readable");
        assert!(fs::read_to_string(&pp_sclk).is_ok(), "pp_dpm_sclk should be readable");
        assert!(fs::read_to_string(&pp_mclk).is_ok(), "pp_dpm_mclk should be readable");
    }
}
