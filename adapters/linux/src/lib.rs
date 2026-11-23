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
    use super::{parse_milli_celsius, parse_rpm, SOURCE};
    use shookshmam_hardware_core::{now_unix_ms, HardwareError};
    use shookshmam_sensor_model::{
        DeviceDescriptor, DeviceKind, SensorCategory, SensorReading,
    };
    use std::fs;
    use std::path::{Path, PathBuf};

    const HWMON_ROOT: &str = "/sys/class/hwmon";

    fn read_trimmed(path: &Path) -> Option<String> {
        fs::read_to_string(path).ok().map(|s| s.trim().to_string())
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
            out.push(DeviceDescriptor {
                id,
                name: name.clone(),
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
                            shookshmam_sensor_model::SensorStatus::Error,
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

        readings.sort_by(|a, b| a.id.cmp(&b.id));
        Ok(readings)
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
    #[cfg(not(target_os = "linux"))]
    fn not_implemented_off_linux() {
        let p = HwmonProvider::new();
        assert!(matches!(p.discover(), Err(HardwareError::NotImplemented)));
    }
}
