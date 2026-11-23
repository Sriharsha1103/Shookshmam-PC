use shookshmam_sensor_model::SensorReading;

#[derive(Debug)]
pub enum HardwareError {
    NotImplemented,
    PermissionDenied,
    DeviceUnavailable,
    Io(String),
    InvalidData(String),
}

pub trait SensorProvider: Send + Sync {
    fn provider_id(&self) -> &'static str;
    fn discover(&self) -> Result<Vec<String>, HardwareError>;
    fn read_snapshot(&self, device_id: &str) -> Result<Vec<SensorReading>, HardwareError>;
}

/// Phase 1 intentionally exposes no hardware-control trait or write command.
