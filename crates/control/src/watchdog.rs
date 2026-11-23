//! Watchdog heartbeat (PHASE-3-SPEC §7). While control is active a heartbeat
//! must keep arriving; if it stops (UI/service hang, comms loss) the caller must
//! execute the adapter's tested safe-release behavior.

#[derive(Debug, Clone)]
pub struct Watchdog {
    timeout_ms: i64,
    last_beat_ms: i64,
}

impl Watchdog {
    pub fn new(timeout_ms: i64, now_ms: i64) -> Self {
        Watchdog { timeout_ms: timeout_ms.max(1), last_beat_ms: now_ms }
    }

    pub fn heartbeat(&mut self, now_ms: i64) {
        self.last_beat_ms = now_ms;
    }

    /// True if no heartbeat has arrived within the timeout — control must be
    /// released.
    pub fn is_tripped(&self, now_ms: i64) -> bool {
        now_ms.saturating_sub(self.last_beat_ms) >= self.timeout_ms
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trips_after_timeout() {
        let mut w = Watchdog::new(5000, 1000);
        assert!(!w.is_tripped(5000));
        assert!(w.is_tripped(6000));
        w.heartbeat(6000);
        assert!(!w.is_tripped(10_000));
        assert!(w.is_tripped(11_000));
    }
}
