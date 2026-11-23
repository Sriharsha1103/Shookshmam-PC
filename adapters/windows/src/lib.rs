//! Windows read-only sensor adapter (P1-04).
//!
//! Status: **documented stub, not yet wired to a backend.** PHASE-1-SPEC §9
//! requires evaluating LibreHardwareMonitor (or an equivalent) together with its
//! licensing/redistribution constraints *before* bundling it, and keeping any
//! privileged helper process narrow and separately permissioned. Until that
//! review is recorded, this provider honestly reports `NotImplemented` rather
//! than fabricating readings (ROADMAP release gate 3).
//!
//! The intended design once a backend is approved:
//! - Run the sensor backend in a narrow helper process, elevated only if a
//!   specific provider requires it, never the whole UI.
//! - Map backend sensor nodes to the normalized `SensorReading` contract,
//!   preserving unsupported/permission-denied states.

use shookshmam_hardware_core::{HardwareError, SensorProvider};
use shookshmam_sensor_model::{DeviceDescriptor, SensorReading};

pub const SOURCE: &str = "windows-sensors";

pub struct WindowsSensorProvider;

impl WindowsSensorProvider {
    pub fn new() -> Self {
        WindowsSensorProvider
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
        // No backend approved/integrated yet — see module docs.
        Err(HardwareError::NotImplemented)
    }

    fn read_snapshot(&self, _device_id: &str) -> Result<Vec<SensorReading>, HardwareError> {
        Err(HardwareError::NotImplemented)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reports_not_implemented_until_backend_review() {
        let p = WindowsSensorProvider::new();
        assert!(matches!(p.discover(), Err(HardwareError::NotImplemented)));
    }
}
