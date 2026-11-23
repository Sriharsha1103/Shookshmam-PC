use crate::provider::SensorProvider;
use serde::Serialize;
use shookshmam_sensor_model::{SensorReading, Snapshot};
use std::panic::{catch_unwind, AssertUnwindSafe};

/// Per-provider outcome of a collection pass, surfaced to diagnostics/UI.
#[derive(Debug, Clone, Serialize)]
pub struct ProviderStatus {
    pub provider_id: String,
    pub devices: usize,
    pub readings: usize,
    pub ok: bool,
    pub last_error: Option<String>,
}

/// Result of collecting from every registered provider.
#[derive(Debug, Clone, Serialize)]
pub struct CollectResult {
    pub snapshot: Snapshot,
    pub providers: Vec<ProviderStatus>,
}

/// Owns the set of providers and merges their readings into one snapshot.
///
/// A single failing (or panicking) provider marks itself not-ok and records the
/// error, but never aborts collection of the others (ARCHITECTURE.md §9;
/// PHASE-1-SPEC §10). Adapters are responsible for emitting explicit
/// `Unsupported`/`Unavailable` readings for individual sensors they cannot read.
#[derive(Default)]
pub struct Registry {
    providers: Vec<Box<dyn SensorProvider>>,
}

impl Registry {
    pub fn new() -> Self {
        Registry { providers: Vec::new() }
    }

    pub fn register(&mut self, provider: Box<dyn SensorProvider>) {
        self.providers.push(provider);
    }

    pub fn provider_ids(&self) -> Vec<&'static str> {
        self.providers.iter().map(|p| p.provider_id()).collect()
    }

    /// The fastest safe polling interval: the max of all providers' minimums and
    /// the requested global target.
    pub fn effective_interval_ms(&self, target_ms: u64) -> u64 {
        self.providers
            .iter()
            .map(|p| p.min_interval_ms())
            .chain(std::iter::once(target_ms))
            .max()
            .unwrap_or(target_ms)
    }

    /// Collect one snapshot from all providers at `now_ms`.
    pub fn collect(&self, now_ms: i64) -> CollectResult {
        let mut all_readings: Vec<SensorReading> = Vec::new();
        let mut statuses: Vec<ProviderStatus> = Vec::with_capacity(self.providers.len());

        for provider in &self.providers {
            let pid = provider.provider_id();
            // Isolate panics so one buggy provider cannot crash the core.
            let outcome =
                catch_unwind(AssertUnwindSafe(|| collect_one(provider.as_ref(), now_ms)));

            let status = match outcome {
                Ok(Ok(collected)) => {
                    let readings_len = collected.readings.len();
                    all_readings.extend(collected.readings);
                    ProviderStatus {
                        provider_id: pid.to_string(),
                        devices: collected.devices,
                        readings: readings_len,
                        ok: collected.device_errors.is_empty(),
                        last_error: collected.device_errors.first().cloned(),
                    }
                }
                Ok(Err(e)) => ProviderStatus {
                    provider_id: pid.to_string(),
                    devices: 0,
                    readings: 0,
                    ok: false,
                    last_error: Some(e),
                },
                Err(_) => ProviderStatus {
                    provider_id: pid.to_string(),
                    devices: 0,
                    readings: 0,
                    ok: false,
                    last_error: Some("provider panicked".to_string()),
                },
            };
            statuses.push(status);
        }

        CollectResult { snapshot: Snapshot::new(now_ms, all_readings), providers: statuses }
    }
}

struct Collected {
    readings: Vec<SensorReading>,
    devices: usize,
    device_errors: Vec<String>,
}

/// Discover devices for one provider and read each. A device-level read error
/// is recorded but does not abort the provider's other devices.
fn collect_one(provider: &dyn SensorProvider, _now_ms: i64) -> Result<Collected, String> {
    let devices = provider.discover().map_err(|e| e.to_string())?;
    let mut readings = Vec::new();
    let mut device_errors = Vec::new();

    for device in &devices {
        match provider.read_snapshot(&device.id) {
            Ok(mut rs) => readings.append(&mut rs),
            Err(e) => device_errors.push(format!("{}: {e}", device.id)),
        }
    }

    Ok(Collected { readings, devices: devices.len(), device_errors })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::provider::HardwareError;
    use shookshmam_sensor_model::{DeviceDescriptor, DeviceKind, SensorCategory};

    struct GoodProvider;
    impl SensorProvider for GoodProvider {
        fn provider_id(&self) -> &'static str {
            "good"
        }
        fn discover(&self) -> Result<Vec<DeviceDescriptor>, HardwareError> {
            Ok(vec![DeviceDescriptor {
                id: "cpu0".into(),
                name: "CPU".into(),
                kind: DeviceKind::Cpu,
                source: "good".into(),
            }])
        }
        fn read_snapshot(&self, device_id: &str) -> Result<Vec<SensorReading>, HardwareError> {
            Ok(vec![SensorReading::available(
                format!("{device_id}.temp"),
                device_id,
                "CPU Temp",
                SensorCategory::Temperature,
                50.0,
                "good",
                100,
            )])
        }
    }

    struct FailingProvider;
    impl SensorProvider for FailingProvider {
        fn provider_id(&self) -> &'static str {
            "failing"
        }
        fn discover(&self) -> Result<Vec<DeviceDescriptor>, HardwareError> {
            Err(HardwareError::PermissionDenied)
        }
        fn read_snapshot(&self, _: &str) -> Result<Vec<SensorReading>, HardwareError> {
            unreachable!()
        }
    }

    struct PanickingProvider;
    impl SensorProvider for PanickingProvider {
        fn provider_id(&self) -> &'static str {
            "panicky"
        }
        fn discover(&self) -> Result<Vec<DeviceDescriptor>, HardwareError> {
            panic!("boom");
        }
        fn read_snapshot(&self, _: &str) -> Result<Vec<SensorReading>, HardwareError> {
            unreachable!()
        }
    }

    #[test]
    fn collects_from_good_provider() {
        let mut reg = Registry::new();
        reg.register(Box::new(GoodProvider));
        let result = reg.collect(100);
        assert_eq!(result.snapshot.readings.len(), 1);
        assert!(result.providers[0].ok);
        assert!(result.snapshot.validate().is_ok());
    }

    #[test]
    fn failing_provider_is_isolated() {
        let mut reg = Registry::new();
        reg.register(Box::new(GoodProvider));
        reg.register(Box::new(FailingProvider));
        let result = reg.collect(100);
        assert_eq!(result.snapshot.readings.len(), 1);
        let failing = result.providers.iter().find(|p| p.provider_id == "failing").unwrap();
        assert!(!failing.ok);
        assert!(failing.last_error.as_deref().unwrap().contains("permission"));
    }

    #[test]
    fn panicking_provider_does_not_crash_core() {
        let mut reg = Registry::new();
        reg.register(Box::new(PanickingProvider));
        reg.register(Box::new(GoodProvider));
        let result = reg.collect(100);
        assert_eq!(result.snapshot.readings.len(), 1);
        let p = result.providers.iter().find(|p| p.provider_id == "panicky").unwrap();
        assert!(!p.ok);
        assert_eq!(p.last_error.as_deref(), Some("provider panicked"));
    }

    #[test]
    fn effective_interval_respects_slowest_provider() {
        struct Slow;
        impl SensorProvider for Slow {
            fn provider_id(&self) -> &'static str {
                "slow"
            }
            fn min_interval_ms(&self) -> u64 {
                5000
            }
            fn discover(&self) -> Result<Vec<DeviceDescriptor>, HardwareError> {
                Ok(vec![])
            }
            fn read_snapshot(&self, _: &str) -> Result<Vec<SensorReading>, HardwareError> {
                Ok(vec![])
            }
        }
        let mut reg = Registry::new();
        reg.register(Box::new(Slow));
        assert_eq!(reg.effective_interval_ms(1000), 5000);
    }
}
