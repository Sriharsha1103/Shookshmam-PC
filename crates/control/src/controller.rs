//! Fan-controller hardware interface and a gated no-write implementation.
//!
//! Concrete adapters (ASUS motherboard headers, validated Lian Li fan channels)
//! implement [`FanController`]. Until a channel passes hardware validation,
//! `apply_duty` must refuse with [`ControlError::NotValidated`] (ADR-0002;
//! PHASE-3-SPEC §9 criterion 2). `release` is always safe (hands control back to
//! firmware) and defaults to a no-op when we never took control.

use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ControlError {
    /// The channel has not passed hardware validation; writes are refused.
    NotValidated,
    /// The channel/operation is not supported on this hardware.
    Unsupported,
    /// Requested duty is outside the channel's validated bounds.
    OutOfBounds,
    Io(String),
}

impl std::fmt::Display for ControlError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ControlError::NotValidated => write!(f, "channel not validated; write refused"),
            ControlError::Unsupported => write!(f, "operation not supported"),
            ControlError::OutOfBounds => write!(f, "duty out of validated bounds"),
            ControlError::Io(e) => write!(f, "control I/O error: {e}"),
        }
    }
}

impl std::error::Error for ControlError {}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct ChannelInfo {
    pub id: String,
    pub label: String,
    pub min_duty: f64,
    pub max_duty: f64,
    pub allow_stop: bool,
    pub validated: bool,
}

/// Read fan RPM and request bounded duty on controllable channels.
pub trait FanController {
    fn channels(&self) -> Vec<ChannelInfo>;
    /// Read tachometer RPM where available. `Ok(None)` means "no tach source",
    /// never a fabricated value.
    fn read_rpm(&self, channel: &str) -> Result<Option<f64>, ControlError>;
    fn apply_duty(&mut self, channel: &str, duty: f64) -> Result<(), ControlError>;
    fn release(&mut self, channel: &str) -> Result<(), ControlError>;
}

/// A controller that knows its channels but refuses all duty writes until the
/// channel is validated. Used to model ASUS/Lian Li channels before HIL sign-off.
#[derive(Debug, Clone, Default)]
pub struct GatedFanController {
    channels: Vec<ChannelInfo>,
}

impl GatedFanController {
    pub fn new(channels: Vec<ChannelInfo>) -> Self {
        GatedFanController { channels }
    }
}

impl FanController for GatedFanController {
    fn channels(&self) -> Vec<ChannelInfo> {
        self.channels.clone()
    }

    fn read_rpm(&self, _channel: &str) -> Result<Option<f64>, ControlError> {
        // No validated tachometer source yet; report "no reading" honestly.
        Ok(None)
    }

    fn apply_duty(&mut self, channel: &str, _duty: f64) -> Result<(), ControlError> {
        match self.channels.iter().find(|c| c.id == channel) {
            Some(c) if c.validated => Err(ControlError::Unsupported), // validated but no real backend wired
            Some(_) => Err(ControlError::NotValidated),
            None => Err(ControlError::Unsupported),
        }
    }

    fn release(&mut self, _channel: &str) -> Result<(), ControlError> {
        // We never took control, so releasing to firmware is a safe no-op.
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctl() -> GatedFanController {
        GatedFanController::new(vec![ChannelInfo {
            id: "cpu_fan".into(),
            label: "CPU Fan".into(),
            min_duty: 20.0,
            max_duty: 100.0,
            allow_stop: false,
            validated: false,
        }])
    }

    #[test]
    fn refuses_writes_until_validated() {
        let mut c = ctl();
        assert_eq!(c.apply_duty("cpu_fan", 50.0), Err(ControlError::NotValidated));
        assert_eq!(c.apply_duty("unknown", 50.0), Err(ControlError::Unsupported));
    }

    #[test]
    fn release_is_safe_noop() {
        let mut c = ctl();
        assert_eq!(c.release("cpu_fan"), Ok(()));
    }

    #[test]
    fn read_rpm_never_fabricates() {
        let c = ctl();
        assert_eq!(c.read_rpm("cpu_fan"), Ok(None));
    }
}
