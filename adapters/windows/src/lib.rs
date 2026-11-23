//! Windows read-only sensor adapter (P1-04).
//!
//! Provides CPU/motherboard/GPU temperatures and fan speeds on Windows via WMI
//! (Windows Management Instrumentation) queries. Queries are executed via PowerShell,
//! which is built into all modern Windows systems.
//!
//! **Design:** Sensors that the OS doesn't expose (e.g., GPU temps without vendor
//! drivers) are reported as `Unsupported`, never fabricated. Fan speeds are queried
//! where available; fans not exposed by the system are reported as unavailable.

use std::process::Command;
use std::sync::Mutex;

use shookshmam_hardware_core::{now_unix_ms, HardwareError, SensorProvider};
use shookshmam_sensor_model::{DeviceDescriptor, DeviceKind, SensorCategory, SensorReading, SensorStatus};

pub const SOURCE: &str = "windows-sensors";

pub struct WindowsSensorProvider {
    cache: Mutex<Option<CachedSensors>>,
}

#[derive(Clone)]
struct CachedSensors {
    timestamp: i64,
    motherboard_temps: Vec<(String, f64)>,
    fan_speeds: Vec<(String, f64)>,
}

impl WindowsSensorProvider {
    pub fn new() -> Self {
        WindowsSensorProvider {
            cache: Mutex::new(None),
        }
    }

    /// Query WMI for motherboard temperatures and fan speeds. Caches for 1 second.
    fn get_sensors(&self) -> Result<CachedSensors, HardwareError> {
        let now = now_unix_ms();

        // Return cached data if fresh (within 1 second)
        if let Ok(cache) = self.cache.lock() {
            if let Some(cached) = cache.as_ref() {
                if now - cached.timestamp < 1000 {
                    return Ok(cached.clone());
                }
            }
        }

        // Query fresh data
        let motherboard_temps = query_motherboard_temps()
            .unwrap_or_default();
        let fan_speeds = query_fan_speeds()
            .unwrap_or_default();

        let sensors = CachedSensors {
            timestamp: now,
            motherboard_temps,
            fan_speeds,
        };

        let _ = self.cache.lock().map(|mut c| *c = Some(sensors.clone()));
        Ok(sensors)
    }
}

impl Default for WindowsSensorProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl SensorProvider for WindowsSensorProvider {
    fn provider_id(&self) -> &'static str {
        SOURCE
    }

    fn discover(&self) -> Result<Vec<DeviceDescriptor>, HardwareError> {
        // Always advertise board0 (motherboard sensors); we'll report Unsupported
        // for sensors that don't exist rather than failing discovery.
        Ok(vec![
            DeviceDescriptor {
                id: "board0".into(),
                name: "System Sensors (WMI)".into(),
                kind: DeviceKind::Motherboard,
                source: SOURCE.into(),
            },
        ])
    }

    fn read_snapshot(&self, device_id: &str) -> Result<Vec<SensorReading>, HardwareError> {
        match device_id {
            "board0" => {
                let sensors = self.get_sensors()?;
                let ts = now_unix_ms();
                let mut readings = Vec::new();

                // Motherboard/system component temperatures
                if sensors.motherboard_temps.is_empty() {
                    readings.push(SensorReading::without_value(
                        "sys.temp.motherboard",
                        "board0",
                        "Motherboard".to_string(),
                        SensorCategory::Temperature,
                        SOURCE,
                        SensorStatus::Unsupported,
                        ts,
                    ).with_note("WMI query returned no results (may need drivers or permissions)"));
                } else {
                    for (name, temp) in sensors.motherboard_temps {
                        readings.push(SensorReading::available(
                            format!("sys.temp.{}", slug(&name)),
                            "board0",
                            name,
                            SensorCategory::Temperature,
                            temp,
                            SOURCE,
                            ts,
                        ));
                    }
                }

                // Fan speeds
                if sensors.fan_speeds.is_empty() {
                    readings.push(SensorReading::without_value(
                        "sys.fan.motherboard",
                        "board0",
                        "Fans".to_string(),
                        SensorCategory::FanSpeed,
                        SOURCE,
                        SensorStatus::Unsupported,
                        ts,
                    ).with_note("WMI query returned no results"));
                } else {
                    for (name, rpm) in sensors.fan_speeds {
                        readings.push(SensorReading::available(
                            format!("sys.fan.{}", slug(&name)),
                            "board0",
                            name,
                            SensorCategory::FanSpeed,
                            rpm,
                            SOURCE,
                            ts,
                        ));
                    }
                }

                Ok(readings)
            }
            _ => Err(HardwareError::DeviceUnavailable),
        }
    }
}

/// Query WMI for motherboard/CPU temperatures via PowerShell.
/// Returns list of (sensor_name, temperature_celsius).
fn query_motherboard_temps() -> Result<Vec<(String, f64)>, HardwareError> {
    let output = Command::new("powershell")
        .args(&[
            "-NoProfile",
            "-Command",
            "Get-WmiObject MSAcpi_ThermalZoneTemperature -Namespace \"root/wmi\" -ErrorAction SilentlyContinue | Select-Object InstanceName, @{Name='Temp';Expression={($_.CurrentTemperature-2732)/10}} | ConvertTo-Json",
        ])
        .output()
        .map_err(|e| HardwareError::Io(format!("PowerShell exec failed: {}", e)))?;

    if !output.status.success() {
        return Ok(Vec::new()); // No thermal zones found; return empty list (not an error)
    }

    let stdout = String::from_utf8(output.stdout)
        .map_err(|e| HardwareError::Io(format!("UTF-8 decode: {}", e)))?;

    // Simple JSON parsing (avoiding serde_json dependency)
    let mut results = Vec::new();
    if stdout.contains("InstanceName") {
        // Try to extract values from PowerShell JSON output
        // Format: [{"InstanceName":"CPU","Temp":51.2}, ...]
        let entries: Vec<&str> = stdout.split("},").collect();
        for entry in entries {
            if let (Some(name_start), Some(name_end)) =
                (entry.find("\"InstanceName\":\""), entry.find("\","))
            {
                if let (Some(temp_start), Some(temp_end)) =
                    (entry.find("\"Temp\":"), entry.find("}"))
                {
                    let name = entry[name_start + 16..name_end].to_string();
                    let temp_str = &entry[temp_start + 7..temp_end].trim();
                    if let Ok(temp) = temp_str.parse::<f64>() {
                        results.push((name, temp));
                    }
                }
            }
        }
    }
    Ok(results)
}

/// Query WMI for fan speeds via PowerShell.
/// Returns list of (fan_name, rpm).
fn query_fan_speeds() -> Result<Vec<(String, f64)>, HardwareError> {
    let output = Command::new("powershell")
        .args(&[
            "-NoProfile",
            "-Command",
            "Get-WmiObject Win32_Fan -ErrorAction SilentlyContinue | Select-Object Name, CurrentSpeed | ConvertTo-Json",
        ])
        .output()
        .map_err(|e| HardwareError::Io(format!("PowerShell exec failed: {}", e)))?;

    if !output.status.success() {
        return Ok(Vec::new());
    }

    let stdout = String::from_utf8(output.stdout)
        .map_err(|e| HardwareError::Io(format!("UTF-8 decode: {}", e)))?;

    let mut results = Vec::new();
    if stdout.contains("Name") {
        let entries: Vec<&str> = stdout.split("},").collect();
        for entry in entries {
            if let (Some(name_start), Some(name_end)) =
                (entry.find("\"Name\":\""), entry.find("\","))
            {
                if let (Some(speed_start), Some(speed_end)) =
                    (entry.find("\"CurrentSpeed\":"), entry.find("}"))
                {
                    let name = entry[name_start + 8..name_end].to_string();
                    let speed_str = &entry[speed_start + 15..speed_end].trim();
                    if let Ok(speed) = speed_str.parse::<f64>() {
                        if speed > 0.0 {  // Ignore zero/null values
                            results.push((name, speed));
                        }
                    }
                }
            }
        }
    }
    Ok(results)
}

/// Slugify a sensor name into an id fragment.
fn slug(label: &str) -> String {
    label
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c.to_ascii_lowercase() } else { '_' })
        .collect::<String>()
        .trim_matches('_')
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn discovers_motherboard_sensors() {
        let p = WindowsSensorProvider::new();
        let devices = p.discover().unwrap();
        assert!(devices.iter().any(|d| d.id == "board0"));
    }

    #[test]
    fn slug_sanitizes_names() {
        assert_eq!(slug("CPU Temperature"), "cpu_temperature");
        assert_eq!(slug("Fan #1 Speed"), "fan__1_speed");
        assert_eq!(slug("_Leading/Trailing_"), "leading_trailing");
    }
}
