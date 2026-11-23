//! Lighting controller interface and a gated no-write implementation.
//!
//! Concrete adapters (ASUS ARGB header, validated Lian Li RGB) implement
//! [`LightingController`]. Until a zone is writable and validated, `apply`/`off`
//! refuse. On adapter failure the caller must stop sending commands (no rapid
//! retry loop — spec §6).

use crate::effect::Effect;
use crate::policy::{evaluate_write, LightingWriteDecision};
use crate::zone::LightingZone;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq)]
pub enum LightingError {
    /// Zone cannot be written (electrical type/topology unknown, unvalidated,
    /// or disconnected).
    ZoneUnwritable(String),
    /// First-write confirmation was not given.
    NotConfirmed,
    /// The effect is invalid or unsupported by the zone.
    InvalidEffect(String),
    /// No backend wired for a validated zone yet.
    Unsupported,
    Io(String),
}

impl std::fmt::Display for LightingError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LightingError::ZoneUnwritable(z) => write!(f, "zone `{z}` is not writable"),
            LightingError::NotConfirmed => write!(f, "first-write confirmation required"),
            LightingError::InvalidEffect(e) => write!(f, "invalid/unsupported effect: {e}"),
            LightingError::Unsupported => write!(f, "no validated backend for this zone"),
            LightingError::Io(e) => write!(f, "lighting I/O error: {e}"),
        }
    }
}

impl std::error::Error for LightingError {}

/// A saved zone state for restore-to-known-state (spec §6).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ZoneState {
    pub zone_id: String,
    pub effect: Effect,
}

pub trait LightingController {
    fn zones(&self) -> Vec<LightingZone>;
    /// Read back current state where supported. `Ok(None)` means state is not
    /// readable (caller must disclose this before applying changes).
    fn read_state(&self, zone_id: &str) -> Result<Option<ZoneState>, LightingError>;
    fn apply(
        &mut self,
        zone_id: &str,
        effect: &Effect,
        confirmed_first_write: bool,
    ) -> Result<(), LightingError>;
    fn off(&mut self, zone_id: &str) -> Result<(), LightingError>;
}

/// Knows its zones but refuses writes until a zone is writable + validated and
/// a real backend exists.
#[derive(Debug, Clone, Default)]
pub struct GatedLightingController {
    zones: Vec<LightingZone>,
}

impl GatedLightingController {
    pub fn new(zones: Vec<LightingZone>) -> Self {
        GatedLightingController { zones }
    }

    fn zone(&self, zone_id: &str) -> Option<&LightingZone> {
        self.zones.iter().find(|z| z.id == zone_id)
    }
}

impl LightingController for GatedLightingController {
    fn zones(&self) -> Vec<LightingZone> {
        self.zones.clone()
    }

    fn read_state(&self, zone_id: &str) -> Result<Option<ZoneState>, LightingError> {
        match self.zone(zone_id) {
            Some(_) => Ok(None), // no backend/readback yet; never fabricate state
            None => Err(LightingError::ZoneUnwritable(zone_id.to_string())),
        }
    }

    fn apply(
        &mut self,
        zone_id: &str,
        effect: &Effect,
        confirmed_first_write: bool,
    ) -> Result<(), LightingError> {
        let zone = self
            .zone(zone_id)
            .ok_or_else(|| LightingError::ZoneUnwritable(zone_id.to_string()))?;
        match evaluate_write(zone, effect, confirmed_first_write) {
            LightingWriteDecision::Refuse(reason) => {
                if reason.contains("confirmation") {
                    Err(LightingError::NotConfirmed)
                } else if reason.contains("effect") {
                    Err(LightingError::InvalidEffect(reason))
                } else {
                    Err(LightingError::ZoneUnwritable(zone_id.to_string()))
                }
            }
            // Policy allowed it, but no hardware backend is wired yet.
            LightingWriteDecision::Allow => Err(LightingError::Unsupported),
        }
    }

    fn off(&mut self, zone_id: &str) -> Result<(), LightingError> {
        // "Off" is still a write; route through the same gate (confirmed).
        self.apply(zone_id, &Effect::Off, true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::zone::{EffectClass, ElectricalType, LightingZone, Rgb};
    use shookshmam_device_model::{ConnectionState, ValidationStatus};

    fn unvalidated_zone() -> LightingZone {
        let mut z = LightingZone::new("z", "Zone", "ctl", "hdr1");
        z.supported_effects = vec![EffectClass::Static, EffectClass::Off];
        z
    }

    #[test]
    fn refuses_apply_on_unvalidated_zone() {
        let mut c = GatedLightingController::new(vec![unvalidated_zone()]);
        let err = c.apply("z", &Effect::Static { color: Rgb::new(255, 0, 0) }, true).unwrap_err();
        assert!(matches!(err, LightingError::ZoneUnwritable(_)));
    }

    #[test]
    fn validated_zone_without_backend_reports_unsupported() {
        let mut z = unvalidated_zone();
        z.electrical_type = ElectricalType::Argb5v3pin;
        z.led_count = Some(8);
        z.validation = ValidationStatus::Validated;
        z.connection = ConnectionState::Connected;
        let mut c = GatedLightingController::new(vec![z]);
        let err = c.apply("z", &Effect::Static { color: Rgb::new(0, 255, 0) }, true).unwrap_err();
        assert_eq!(err, LightingError::Unsupported);
    }

    #[test]
    fn read_state_never_fabricates() {
        let c = GatedLightingController::new(vec![unvalidated_zone()]);
        assert_eq!(c.read_state("z"), Ok(None));
        assert!(c.read_state("missing").is_err());
    }
}
