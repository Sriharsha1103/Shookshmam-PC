use crate::registry::{CollectResult, ProviderStatus};
use serde::Serialize;
use shookshmam_sensor_model::{SensorStatus, Snapshot};

/// A privacy-safe snapshot of system/provider health for the support bundle
/// (PHASE-1-SPEC §10, §11.4). Contains only sensor/provider status counts, never
/// user files or personal paths.
#[derive(Debug, Clone, Serialize)]
pub struct Diagnostics {
    pub collected_at_unix_ms: i64,
    pub providers: Vec<ProviderStatus>,
    pub total_readings: usize,
    pub available: usize,
    pub stale: usize,
    pub unavailable: usize,
    pub unsupported: usize,
    pub permission_denied: usize,
    pub errored: usize,
}

impl Diagnostics {
    pub fn from_collect(result: &CollectResult) -> Self {
        Self::count(result.snapshot.timestamp_unix_ms, &result.snapshot, result.providers.clone())
    }

    fn count(ts: i64, snapshot: &Snapshot, providers: Vec<ProviderStatus>) -> Self {
        let mut d = Diagnostics {
            collected_at_unix_ms: ts,
            providers,
            total_readings: snapshot.readings.len(),
            available: 0,
            stale: 0,
            unavailable: 0,
            unsupported: 0,
            permission_denied: 0,
            errored: 0,
        };
        for r in &snapshot.readings {
            match r.status {
                SensorStatus::Available => d.available += 1,
                SensorStatus::Stale => d.stale += 1,
                SensorStatus::Unavailable => d.unavailable += 1,
                SensorStatus::Unsupported => d.unsupported += 1,
                SensorStatus::PermissionDenied => d.permission_denied += 1,
                SensorStatus::Error => d.errored += 1,
            }
        }
        d
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use shookshmam_sensor_model::{SensorCategory, SensorReading};

    #[test]
    fn counts_statuses() {
        let readings = vec![
            SensorReading::available("a", "d", "A", SensorCategory::Power, 1.0, "s", 10),
            SensorReading::without_value(
                "b",
                "d",
                "B",
                SensorCategory::Temperature,
                "s",
                SensorStatus::Unsupported,
                10,
            ),
            SensorReading::without_value(
                "c",
                "d",
                "C",
                SensorCategory::Temperature,
                "s",
                SensorStatus::PermissionDenied,
                10,
            ),
        ];
        let result = CollectResult {
            snapshot: Snapshot::new(10, readings),
            providers: vec![],
        };
        let d = Diagnostics::from_collect(&result);
        assert_eq!(d.total_readings, 3);
        assert_eq!(d.available, 1);
        assert_eq!(d.unsupported, 1);
        assert_eq!(d.permission_denied, 1);
    }
}
