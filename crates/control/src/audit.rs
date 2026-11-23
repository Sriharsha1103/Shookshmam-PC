//! Bounded control-event audit log (PHASE-3-SPEC §7): record every control
//! transition, command, rejection and fallback — without unrelated personal data.

use serde::Serialize;
use std::collections::VecDeque;

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ControlEventKind {
    Applied,
    Rejected,
    Fallback,
    Released,
    ModeChange,
    WatchdogTrip,
}

#[derive(Debug, Clone, Serialize)]
pub struct ControlEvent {
    pub ts_ms: i64,
    pub channel: String,
    pub kind: ControlEventKind,
    pub detail: String,
}

/// Fixed-capacity audit log (oldest entries drop out).
#[derive(Debug, Clone)]
pub struct AuditLog {
    capacity: usize,
    events: VecDeque<ControlEvent>,
}

impl AuditLog {
    pub fn new(capacity: usize) -> Self {
        AuditLog { capacity: capacity.max(1), events: VecDeque::new() }
    }

    pub fn record(
        &mut self,
        ts_ms: i64,
        channel: impl Into<String>,
        kind: ControlEventKind,
        detail: impl Into<String>,
    ) {
        if self.events.len() == self.capacity {
            self.events.pop_front();
        }
        self.events.push_back(ControlEvent {
            ts_ms,
            channel: channel.into(),
            kind,
            detail: detail.into(),
        });
    }

    pub fn len(&self) -> usize {
        self.events.len()
    }

    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
    }

    /// Events oldest-to-newest.
    pub fn events(&self) -> impl Iterator<Item = &ControlEvent> {
        self.events.iter()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bounded_and_ordered() {
        let mut log = AuditLog::new(2);
        log.record(1, "a", ControlEventKind::Applied, "50%");
        log.record(2, "a", ControlEventKind::Rejected, "stale");
        log.record(3, "a", ControlEventKind::Released, "shutdown");
        assert_eq!(log.len(), 2);
        let kinds: Vec<_> = log.events().map(|e| e.kind).collect();
        assert_eq!(kinds, vec![ControlEventKind::Rejected, ControlEventKind::Released]);
    }
}
