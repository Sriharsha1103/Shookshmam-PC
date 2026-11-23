/// A minimal, clock-injected poll scheduler. The caller owns the real timing
/// loop (async task / thread); this just answers "is a poll due?" so the policy
/// is unit-testable without sleeping.
#[derive(Debug, Clone)]
pub struct Scheduler {
    interval_ms: u64,
    last_tick_ms: Option<i64>,
}

impl Scheduler {
    /// `interval_ms` should already be the effective interval (the max of the
    /// global target and every provider's minimum — see
    /// `Registry::effective_interval_ms`).
    pub fn new(interval_ms: u64) -> Self {
        Scheduler { interval_ms: interval_ms.max(1), last_tick_ms: None }
    }

    pub fn interval_ms(&self) -> u64 {
        self.interval_ms
    }

    /// Returns true if a poll should run at `now_ms`, recording the tick when it
    /// does. The first call always fires.
    pub fn due(&mut self, now_ms: i64) -> bool {
        let fire = match self.last_tick_ms {
            None => true,
            Some(last) => now_ms.saturating_sub(last) >= self.interval_ms as i64,
        };
        if fire {
            self.last_tick_ms = Some(now_ms);
        }
        fire
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn first_tick_always_fires() {
        let mut s = Scheduler::new(1000);
        assert!(s.due(0));
    }

    #[test]
    fn respects_interval() {
        let mut s = Scheduler::new(1000);
        assert!(s.due(10_000));
        assert!(!s.due(10_500)); // only 500ms later
        assert!(s.due(11_000)); // exactly one interval later
    }

    #[test]
    fn interval_never_zero() {
        let mut s = Scheduler::new(0);
        assert_eq!(s.interval_ms(), 1);
        assert!(s.due(0));
        assert!(s.due(1));
    }
}
