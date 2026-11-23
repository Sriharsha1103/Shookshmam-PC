use shookshmam_sensor_model::{SensorReading, SensorStatus, Snapshot};
use std::collections::HashMap;

/// A fixed-capacity ring buffer for recent snapshots (bounded in-memory history,
/// PHASE-1-SPEC §5). Oldest entries are overwritten once full.
#[derive(Debug, Clone)]
pub struct RingBuffer<T> {
    buf: Vec<T>,
    capacity: usize,
    next: usize,
}

impl<T: Clone> RingBuffer<T> {
    pub fn new(capacity: usize) -> Self {
        RingBuffer { buf: Vec::with_capacity(capacity.max(1)), capacity: capacity.max(1), next: 0 }
    }

    pub fn push(&mut self, item: T) {
        if self.buf.len() < self.capacity {
            self.buf.push(item);
        } else {
            self.buf[self.next] = item;
            self.next = (self.next + 1) % self.capacity;
        }
    }

    pub fn len(&self) -> usize {
        self.buf.len()
    }

    pub fn is_empty(&self) -> bool {
        self.buf.is_empty()
    }

    pub fn capacity(&self) -> usize {
        self.capacity
    }

    /// Entries oldest-to-newest.
    pub fn iter_chrono(&self) -> impl Iterator<Item = &T> {
        let (a, b) = if self.buf.len() < self.capacity {
            (&self.buf[..], &self.buf[0..0])
        } else {
            (&self.buf[self.next..], &self.buf[..self.next])
        };
        a.iter().chain(b.iter())
    }
}

/// Tracks the latest reading per sensor and ages out readings whose provider has
/// stopped updating them. This is what turns "the provider skipped a cycle" into
/// an explicit `Stale` then `Unavailable` status, rather than a frozen value or
/// a fabricated zero (ARCHITECTURE.md §9; PHASE-1-SPEC §4, §10).
#[derive(Debug, Clone)]
pub struct TelemetryTracker {
    staleness_ms: i64,
    drop_ms: i64,
    latest: HashMap<String, SensorReading>,
}

impl TelemetryTracker {
    /// `staleness_ms`: age past which a retained reading becomes `Stale`.
    /// `drop_ms`: age past which it becomes `Unavailable` and loses its value.
    pub fn new(staleness_ms: i64, drop_ms: i64) -> Self {
        TelemetryTracker { staleness_ms, drop_ms: drop_ms.max(staleness_ms), latest: HashMap::new() }
    }

    /// Merge a freshly collected snapshot into the tracked state and return the
    /// current continuous view (every known sensor, with aged statuses applied).
    pub fn ingest(&mut self, snapshot: Snapshot, now_ms: i64) -> Snapshot {
        let fresh_ids: std::collections::HashSet<String> =
            snapshot.readings.iter().map(|r| r.id.clone()).collect();

        // 1. Fresh readings replace tracked state.
        for r in snapshot.readings {
            self.latest.insert(r.id.clone(), r);
        }

        // 2. Age out readings that were NOT in this snapshot.
        for r in self.latest.values_mut() {
            if fresh_ids.contains(&r.id) {
                continue;
            }
            let age = now_ms.saturating_sub(r.timestamp_unix_ms);
            if age >= self.drop_ms {
                if r.status != SensorStatus::Unavailable {
                    r.status = SensorStatus::Unavailable;
                    r.value = None; // never keep a value we can no longer trust
                    r.quality_note = Some("provider stopped updating; value dropped".into());
                }
            } else if age >= self.staleness_ms && r.status.carries_value() {
                r.status = SensorStatus::Stale;
                r.quality_note = Some("last-known value; provider is late".into());
            }
        }

        let mut readings: Vec<SensorReading> = self.latest.values().cloned().collect();
        readings.sort_by(|a, b| a.id.cmp(&b.id));
        Snapshot::new(now_ms, readings)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use shookshmam_sensor_model::SensorCategory;

    fn temp(ts: i64) -> SensorReading {
        SensorReading::available(
            "cpu.temp",
            "cpu0",
            "CPU",
            SensorCategory::Temperature,
            50.0,
            "sim",
            ts,
        )
    }

    #[test]
    fn ring_buffer_bounds_and_order() {
        let mut rb = RingBuffer::new(3);
        for i in 0..5 {
            rb.push(i);
        }
        assert_eq!(rb.len(), 3);
        assert_eq!(rb.capacity(), 3);
        let items: Vec<_> = rb.iter_chrono().copied().collect();
        assert_eq!(items, vec![2, 3, 4]); // oldest two overwritten
    }

    #[test]
    fn fresh_reading_stays_available() {
        let mut t = TelemetryTracker::new(3000, 9000);
        let out = t.ingest(Snapshot::new(1000, vec![temp(1000)]), 1000);
        assert_eq!(out.get("cpu.temp").unwrap().status, SensorStatus::Available);
    }

    #[test]
    fn missing_reading_goes_stale_then_unavailable() {
        let mut t = TelemetryTracker::new(3000, 9000);
        t.ingest(Snapshot::new(1000, vec![temp(1000)]), 1000);

        // Next collect omits the sensor; 4s later -> stale, value retained.
        let out = t.ingest(Snapshot::new(5000, vec![]), 5000);
        let r = out.get("cpu.temp").unwrap();
        assert_eq!(r.status, SensorStatus::Stale);
        assert_eq!(r.value, Some(50.0));

        // 10s after the original reading -> unavailable, value dropped.
        let out = t.ingest(Snapshot::new(11_000, vec![]), 11_000);
        let r = out.get("cpu.temp").unwrap();
        assert_eq!(r.status, SensorStatus::Unavailable);
        assert_eq!(r.value, None);
    }

    #[test]
    fn stale_recovers_when_reading_returns() {
        let mut t = TelemetryTracker::new(3000, 9000);
        t.ingest(Snapshot::new(1000, vec![temp(1000)]), 1000);
        t.ingest(Snapshot::new(5000, vec![]), 5000); // stale
        let out = t.ingest(Snapshot::new(6000, vec![temp(6000)]), 6000);
        assert_eq!(out.get("cpu.temp").unwrap().status, SensorStatus::Available);
    }
}
