use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SensorCategory {
    Temperature, Utilization, Frequency, Voltage, Power, Memory, FanSpeed, StorageHealth,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SensorStatus {
    Available, Unsupported, Unavailable, PermissionDenied, Stale, Error,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SensorReading {
    pub id: String,
    pub device_id: String,
    pub name: String,
    pub category: SensorCategory,
    pub value: Option<f64>,
    pub unit: String,
    pub source: String,
    pub status: SensorStatus,
    pub timestamp_unix_ms: i64,
    pub quality_note: Option<String>,
}

impl SensorReading {
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.id.trim().is_empty() || self.device_id.trim().is_empty() {
            return Err("sensor and device identifiers must not be empty");
        }
        if self.status == SensorStatus::Available {
            let value = self.value.ok_or("available sensor must have a value")?;
            if !value.is_finite() { return Err("sensor value must be finite"); }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn available_sensor_requires_finite_value() {
        let reading = SensorReading {
            id: "gpu.temperature.hotspot".into(), device_id: "gpu0".into(),
            name: "GPU Hotspot".into(), category: SensorCategory::Temperature,
            value: Some(f64::NAN), unit: "C".into(), source: "test".into(),
            status: SensorStatus::Available, timestamp_unix_ms: 1, quality_note: None,
        };
        assert!(reading.validate().is_err());
    }
}
