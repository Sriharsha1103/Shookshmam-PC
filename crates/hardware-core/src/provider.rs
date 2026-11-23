use shookshmam_sensor_model::{DeviceDescriptor, SensorReading};
use std::fmt;

/// Typed error returned across the adapter boundary.
#[derive(Debug, Clone)]
pub enum HardwareError {
    /// Feature/provider not implemented on this platform or build.
    NotImplemented,
    /// The process lacks permission to read the device/sensor.
    PermissionDenied,
    /// The device is not present or disappeared.
    DeviceUnavailable,
    /// An I/O error occurred while talking to the OS/device.
    Io(String),
    /// Data could not be parsed or failed validation.
    InvalidData(String),
    /// The operation exceeded its deadline.
    Timeout,
}

impl fmt::Display for HardwareError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            HardwareError::NotImplemented => write!(f, "not implemented on this platform"),
            HardwareError::PermissionDenied => write!(f, "permission denied"),
            HardwareError::DeviceUnavailable => write!(f, "device unavailable"),
            HardwareError::Io(e) => write!(f, "I/O error: {e}"),
            HardwareError::InvalidData(e) => write!(f, "invalid data: {e}"),
            HardwareError::Timeout => write!(f, "operation timed out"),
        }
    }
}

impl std::error::Error for HardwareError {}

/// A read-only source of sensor telemetry (CPU, GPU, hwmon, etc.).
///
/// Implementations must be `Send + Sync` so the registry can poll them, and must
/// never panic for ordinary error conditions — return a [`HardwareError`] instead.
/// (The registry still guards against panics, but that is a safety net, not a
/// contract the provider may rely on.)
pub trait SensorProvider: Send + Sync {
    /// Short stable identifier, e.g. `"linux-hwmon"`, `"simulation"`.
    fn provider_id(&self) -> &'static str;

    /// Minimum polling interval this provider tolerates, in milliseconds. The
    /// scheduler will not poll faster than this even if the global target is
    /// lower (PHASE-1-SPEC §5).
    fn min_interval_ms(&self) -> u64 {
        1000
    }

    /// Enumerate the devices this provider currently exposes.
    fn discover(&self) -> Result<Vec<DeviceDescriptor>, HardwareError>;

    /// Read a fresh set of readings for one device.
    fn read_snapshot(&self, device_id: &str) -> Result<Vec<SensorReading>, HardwareError>;
}
