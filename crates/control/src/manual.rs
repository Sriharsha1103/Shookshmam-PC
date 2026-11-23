//! Manual override with expiry (PHASE-3-SPEC §6).

use serde::{Deserialize, Serialize};

/// Default manual-override lifetime: 30 minutes.
pub const DEFAULT_MANUAL_DURATION_MS: i64 = 30 * 60 * 1000;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ManualOverride {
    pub channel: String,
    pub duty: f64,
    /// Expiry time (UTC Unix ms). `None` means persistent.
    pub expires_at_ms: Option<i64>,
    /// Persistent mode requires an explicit confirmation + visible warning.
    pub persistent: bool,
}

impl ManualOverride {
    /// A temporary override expiring `duration_ms` after `now_ms`.
    pub fn temporary(channel: impl Into<String>, duty: f64, now_ms: i64, duration_ms: i64) -> Self {
        ManualOverride {
            channel: channel.into(),
            duty,
            expires_at_ms: Some(now_ms + duration_ms.max(0)),
            persistent: false,
        }
    }

    /// A persistent override (no expiry). Caller must have obtained confirmation.
    pub fn persistent(channel: impl Into<String>, duty: f64) -> Self {
        ManualOverride { channel: channel.into(), duty, expires_at_ms: None, persistent: true }
    }

    pub fn is_active(&self, now_ms: i64) -> bool {
        match self.expires_at_ms {
            None => true,
            Some(exp) => now_ms < exp,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn temporary_expires() {
        let m = ManualOverride::temporary("cpu_fan", 60.0, 1000, DEFAULT_MANUAL_DURATION_MS);
        assert!(m.is_active(1000));
        assert!(m.is_active(1000 + DEFAULT_MANUAL_DURATION_MS - 1));
        assert!(!m.is_active(1000 + DEFAULT_MANUAL_DURATION_MS));
    }

    #[test]
    fn persistent_never_expires() {
        let m = ManualOverride::persistent("cpu_fan", 60.0);
        assert!(m.is_active(i64::MAX));
        assert!(m.persistent);
    }
}
