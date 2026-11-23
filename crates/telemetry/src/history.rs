use serde::{Deserialize, Serialize};
use shookshmam_sensor_model::{SensorReading, SensorStatus, Snapshot};

/// One persisted time-series point. Mirrors the normalized reading but flattened
/// for storage/charting (PHASE-1-SPEC §7).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Sample {
    pub sensor_id: String,
    pub device_id: String,
    pub value: Option<f64>,
    pub unit: String,
    pub status: SensorStatus,
    pub source: String,
    pub timestamp_unix_ms: i64,
}

impl Sample {
    pub fn from_reading(r: &SensorReading) -> Self {
        Sample {
            sensor_id: r.id.clone(),
            device_id: r.device_id.clone(),
            value: r.value,
            unit: r.unit.clone(),
            status: r.status,
            source: r.source.clone(),
            timestamp_unix_ms: r.timestamp_unix_ms,
        }
    }
}

/// Durable history of samples with bounded-batch pruning. Implemented in-memory
/// (tests) and over SQLite (`SqliteHistory`).
pub trait HistoryStore {
    /// Persist every reading in a snapshot. Returns the number of samples stored.
    fn insert_snapshot(&mut self, snapshot: &Snapshot) -> Result<usize, String>;

    /// Samples for one sensor within `[from_ms, to_ms]`, oldest-first.
    fn query(&self, sensor_id: &str, from_ms: i64, to_ms: i64) -> Result<Vec<Sample>, String>;

    /// Delete samples strictly older than `cutoff_ms`. Returns rows removed.
    fn prune(&mut self, cutoff_ms: i64) -> Result<usize, String>;

    /// When pruning last completed (UTC Unix ms), if ever.
    fn last_prune_ms(&self) -> Option<i64>;

    /// Total samples currently stored.
    fn count(&self) -> Result<usize, String>;
}

/// Compute the retention cutoff for `retention_days` relative to `now_ms`.
pub fn retention_cutoff_ms(now_ms: i64, retention_days: u32) -> i64 {
    now_ms.saturating_sub(retention_days as i64 * 24 * 60 * 60 * 1000)
}

/// In-memory history, used by tests and as a fallback when SQLite is disabled.
#[derive(Debug, Default)]
pub struct MemoryHistory {
    samples: Vec<Sample>,
    last_prune_ms: Option<i64>,
}

impl MemoryHistory {
    pub fn new() -> Self {
        Self::default()
    }
}

impl HistoryStore for MemoryHistory {
    fn insert_snapshot(&mut self, snapshot: &Snapshot) -> Result<usize, String> {
        let before = self.samples.len();
        self.samples.extend(snapshot.readings.iter().map(Sample::from_reading));
        Ok(self.samples.len() - before)
    }

    fn query(&self, sensor_id: &str, from_ms: i64, to_ms: i64) -> Result<Vec<Sample>, String> {
        let mut out: Vec<Sample> = self
            .samples
            .iter()
            .filter(|s| {
                s.sensor_id == sensor_id
                    && s.timestamp_unix_ms >= from_ms
                    && s.timestamp_unix_ms <= to_ms
            })
            .cloned()
            .collect();
        out.sort_by_key(|s| s.timestamp_unix_ms);
        Ok(out)
    }

    fn prune(&mut self, cutoff_ms: i64) -> Result<usize, String> {
        let before = self.samples.len();
        self.samples.retain(|s| s.timestamp_unix_ms >= cutoff_ms);
        self.last_prune_ms = Some(cutoff_ms);
        Ok(before - self.samples.len())
    }

    fn last_prune_ms(&self) -> Option<i64> {
        self.last_prune_ms
    }

    fn count(&self) -> Result<usize, String> {
        Ok(self.samples.len())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use shookshmam_sensor_model::SensorCategory;

    fn snap(ts: i64, val: f64) -> Snapshot {
        Snapshot::new(
            ts,
            vec![SensorReading::available(
                "cpu.temp",
                "cpu0",
                "CPU",
                SensorCategory::Temperature,
                val,
                "sim",
                ts,
            )],
        )
    }

    #[test]
    fn insert_query_roundtrip() {
        let mut h = MemoryHistory::new();
        h.insert_snapshot(&snap(1000, 40.0)).unwrap();
        h.insert_snapshot(&snap(2000, 41.0)).unwrap();
        let got = h.query("cpu.temp", 0, 5000).unwrap();
        assert_eq!(got.len(), 2);
        assert_eq!(got[0].timestamp_unix_ms, 1000);
        assert_eq!(got[1].value, Some(41.0));
    }

    #[test]
    fn query_window_filters() {
        let mut h = MemoryHistory::new();
        h.insert_snapshot(&snap(1000, 40.0)).unwrap();
        h.insert_snapshot(&snap(9000, 41.0)).unwrap();
        assert_eq!(h.query("cpu.temp", 0, 2000).unwrap().len(), 1);
        assert_eq!(h.query("other", 0, 9000).unwrap().len(), 0);
    }

    #[test]
    fn prune_removes_only_expired() {
        let mut h = MemoryHistory::new();
        h.insert_snapshot(&snap(1000, 40.0)).unwrap();
        h.insert_snapshot(&snap(100_000, 41.0)).unwrap();
        let removed = h.prune(50_000).unwrap();
        assert_eq!(removed, 1);
        assert_eq!(h.count().unwrap(), 1);
        assert_eq!(h.last_prune_ms(), Some(50_000));
    }

    #[test]
    fn retention_cutoff_is_30_days() {
        let day = 24 * 60 * 60 * 1000i64;
        assert_eq!(retention_cutoff_ms(100 * day, 30), 70 * day);
    }
}
