//! Central control-policy engine (PHASE-3-SPEC §7). Every control command passes
//! through here before it can reach an adapter. Control is **disabled by default**
//! per channel and must be explicitly enabled; channels must be validated; manual
//! commands must respect bounds exactly; fan stop requires explicit support.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Validated duty bounds for one channel, supplied by the adapter after
/// hardware verification.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct ChannelBounds {
    pub min_duty: f64,
    pub max_duty: f64,
    /// Whether the hardware + cooling design explicitly support stopping the fan.
    pub allow_stop: bool,
    /// Whether this channel has passed hardware validation.
    pub validated: bool,
}

impl ChannelBounds {
    pub fn new(min_duty: f64, max_duty: f64) -> Self {
        ChannelBounds { min_duty, max_duty, allow_stop: false, validated: false }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandSource {
    Automatic,
    Manual,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ControlCommand {
    pub channel: String,
    pub duty: f64,
    pub source: CommandSource,
}

/// The policy's ruling on a command.
#[derive(Debug, Clone, PartialEq)]
pub enum Decision {
    /// Allowed; apply this (possibly defensively-clamped) duty.
    Apply(f64),
    /// Refused, with a human-readable reason.
    Reject(String),
}

#[derive(Debug, Clone)]
struct ChannelConfig {
    enabled: bool,
    bounds: ChannelBounds,
}

/// Holds per-channel enablement + bounds and rules on each command.
#[derive(Debug, Clone, Default)]
pub struct SafetyPolicy {
    channels: HashMap<String, ChannelConfig>,
}

impl SafetyPolicy {
    pub fn new() -> Self {
        SafetyPolicy { channels: HashMap::new() }
    }

    /// Register a channel with its validated bounds. Control stays disabled
    /// until `enable` is called (spec: disabled by default, including after an
    /// unclean shutdown).
    pub fn register_channel(&mut self, channel: impl Into<String>, bounds: ChannelBounds) {
        self.channels
            .insert(channel.into(), ChannelConfig { enabled: false, bounds });
    }

    /// Explicitly enable control for a channel (requires the user's action).
    pub fn enable(&mut self, channel: &str) -> bool {
        match self.channels.get_mut(channel) {
            Some(c) if c.bounds.validated => {
                c.enabled = true;
                true
            }
            _ => false, // cannot enable an unknown or unvalidated channel
        }
    }

    pub fn disable(&mut self, channel: &str) {
        if let Some(c) = self.channels.get_mut(channel) {
            c.enabled = false;
        }
    }

    pub fn is_enabled(&self, channel: &str) -> bool {
        self.channels.get(channel).map(|c| c.enabled).unwrap_or(false)
    }

    /// Rule on a command.
    pub fn evaluate(&self, cmd: &ControlCommand) -> Decision {
        let cfg = match self.channels.get(&cmd.channel) {
            Some(c) => c,
            None => return Decision::Reject(format!("unknown channel `{}`", cmd.channel)),
        };
        if !cfg.bounds.validated {
            return Decision::Reject(format!("channel `{}` is not validated", cmd.channel));
        }
        if !cfg.enabled {
            return Decision::Reject(format!("control not enabled for `{}`", cmd.channel));
        }
        if !cmd.duty.is_finite() {
            return Decision::Reject("duty must be finite".to_string());
        }

        let (lo, hi) = (cfg.bounds.min_duty, cfg.bounds.max_duty);

        // Fan stop is only allowed when the hardware explicitly supports it.
        if cmd.duty <= 0.0 && !cfg.bounds.allow_stop {
            return match cmd.source {
                CommandSource::Manual => {
                    Decision::Reject("fan stop is not supported on this channel".to_string())
                }
                // Automatic must never actually stop the fan: floor at min.
                CommandSource::Automatic => Decision::Apply(lo),
            };
        }

        if cmd.duty < lo {
            return match cmd.source {
                CommandSource::Manual => {
                    Decision::Reject(format!("duty {:.0}% below channel minimum {:.0}%", cmd.duty, lo))
                }
                CommandSource::Automatic => Decision::Apply(lo),
            };
        }
        if cmd.duty > hi {
            return match cmd.source {
                CommandSource::Manual => {
                    Decision::Reject(format!("duty {:.0}% above channel maximum {:.0}%", cmd.duty, hi))
                }
                CommandSource::Automatic => Decision::Apply(hi),
            };
        }
        Decision::Apply(cmd.duty)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn validated_bounds() -> ChannelBounds {
        ChannelBounds { min_duty: 20.0, max_duty: 100.0, allow_stop: false, validated: true }
    }

    fn policy_with_enabled_channel() -> SafetyPolicy {
        let mut p = SafetyPolicy::new();
        p.register_channel("cpu_fan", validated_bounds());
        assert!(p.enable("cpu_fan"));
        p
    }

    #[test]
    fn unknown_channel_rejected() {
        let p = SafetyPolicy::new();
        let cmd = ControlCommand { channel: "x".into(), duty: 50.0, source: CommandSource::Manual };
        assert!(matches!(p.evaluate(&cmd), Decision::Reject(_)));
    }

    #[test]
    fn disabled_by_default() {
        let mut p = SafetyPolicy::new();
        p.register_channel("cpu_fan", validated_bounds());
        let cmd =
            ControlCommand { channel: "cpu_fan".into(), duty: 50.0, source: CommandSource::Manual };
        assert!(matches!(p.evaluate(&cmd), Decision::Reject(_)));
    }

    #[test]
    fn cannot_enable_unvalidated_channel() {
        let mut p = SafetyPolicy::new();
        p.register_channel("cpu_fan", ChannelBounds::new(20.0, 100.0)); // validated=false
        assert!(!p.enable("cpu_fan"));
        let cmd =
            ControlCommand { channel: "cpu_fan".into(), duty: 50.0, source: CommandSource::Manual };
        assert!(matches!(p.evaluate(&cmd), Decision::Reject(_)));
    }

    #[test]
    fn manual_respects_bounds_strictly() {
        let p = policy_with_enabled_channel();
        let too_low =
            ControlCommand { channel: "cpu_fan".into(), duty: 5.0, source: CommandSource::Manual };
        let too_high =
            ControlCommand { channel: "cpu_fan".into(), duty: 150.0, source: CommandSource::Manual };
        assert!(matches!(p.evaluate(&too_low), Decision::Reject(_)));
        assert!(matches!(p.evaluate(&too_high), Decision::Reject(_)));
        let ok =
            ControlCommand { channel: "cpu_fan".into(), duty: 50.0, source: CommandSource::Manual };
        assert_eq!(p.evaluate(&ok), Decision::Apply(50.0));
    }

    #[test]
    fn automatic_is_defensively_clamped() {
        let p = policy_with_enabled_channel();
        let low =
            ControlCommand { channel: "cpu_fan".into(), duty: 0.0, source: CommandSource::Automatic };
        // Stop not supported -> floored at min, never stopped.
        assert_eq!(p.evaluate(&low), Decision::Apply(20.0));
    }

    #[test]
    fn fan_stop_requires_support() {
        let mut p = SafetyPolicy::new();
        let mut bounds = validated_bounds();
        bounds.allow_stop = true;
        bounds.min_duty = 0.0;
        p.register_channel("case_fan", bounds);
        p.enable("case_fan");
        let stop =
            ControlCommand { channel: "case_fan".into(), duty: 0.0, source: CommandSource::Manual };
        assert_eq!(p.evaluate(&stop), Decision::Apply(0.0));
    }
}
