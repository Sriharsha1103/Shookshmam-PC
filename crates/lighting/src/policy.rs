//! Lighting write policy and ownership (PHASE-4-SPEC §6). Read-only discovery
//! precedes all writes; a zone must be writable (known electrical type + known
//! topology + validated + connected), the effect must be valid and supported by
//! the zone, and the first write to a zone requires explicit confirmation.

use crate::effect::Effect;
use crate::zone::LightingZone;

#[derive(Debug, Clone, PartialEq)]
pub enum LightingWriteDecision {
    Allow,
    Refuse(String),
}

/// Rule on whether an effect may be written to a zone.
pub fn evaluate_write(
    zone: &LightingZone,
    effect: &Effect,
    confirmed_first_write: bool,
) -> LightingWriteDecision {
    if !zone.writable() {
        return LightingWriteDecision::Refuse(format!(
            "zone `{}` is not writable (electrical type/topology unknown, unvalidated or disconnected)",
            zone.id
        ));
    }
    if let Err(e) = effect.validate() {
        return LightingWriteDecision::Refuse(format!("invalid effect: {e}"));
    }
    if !zone.supported_effects.contains(&effect.class()) {
        return LightingWriteDecision::Refuse(format!(
            "effect {:?} is not supported by zone `{}`",
            effect.class(),
            zone.id
        ));
    }
    if !confirmed_first_write {
        return LightingWriteDecision::Refuse(
            "explicit confirmation is required before the first write to this zone".to_string(),
        );
    }
    LightingWriteDecision::Allow
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OwnershipState {
    Clear,
    Conflict { competitor: String },
}

/// Other lighting software that must not write concurrently to the same
/// controller (spec §6).
const COMPETITORS: &[&str] =
    &["OpenRGB", "L-Connect", "LConnect", "Armoury Crate", "SignalRGB", "Aura", "iCUE"];

pub fn check_lighting_ownership<I, S>(running_processes: I) -> OwnershipState
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    for proc in running_processes {
        let lower = proc.as_ref().to_ascii_lowercase();
        if let Some(hit) = COMPETITORS.iter().find(|c| lower.contains(&c.to_ascii_lowercase())) {
            return OwnershipState::Conflict { competitor: (*hit).to_string() };
        }
    }
    OwnershipState::Clear
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::zone::{EffectClass, ElectricalType, LightingZone, Rgb};
    use shookshmam_device_model::{ConnectionState, ValidationStatus};

    fn writable_zone() -> LightingZone {
        let mut z = LightingZone::new("z", "Zone", "ctl", "hdr1");
        z.electrical_type = ElectricalType::Argb5v3pin;
        z.led_count = Some(8);
        z.validation = ValidationStatus::Validated;
        z.connection = ConnectionState::Connected;
        z.supported_effects = vec![EffectClass::Static, EffectClass::Off];
        z
    }

    #[test]
    fn refuses_unwritable_zone() {
        let z = LightingZone::new("z", "Zone", "ctl", "hdr1"); // unknown electrical type
        let d = evaluate_write(&z, &Effect::Static { color: Rgb::BLACK }, true);
        assert!(matches!(d, LightingWriteDecision::Refuse(_)));
    }

    #[test]
    fn refuses_unsupported_effect() {
        let z = writable_zone(); // supports Static/Off only
        let d = evaluate_write(&z, &Effect::ColorCycle { speed: 5, brightness: 100 }, true);
        assert!(matches!(d, LightingWriteDecision::Refuse(_)));
    }

    #[test]
    fn requires_confirmation() {
        let z = writable_zone();
        let effect = Effect::Static { color: Rgb::new(255, 0, 0) };
        assert!(matches!(
            evaluate_write(&z, &effect, false),
            LightingWriteDecision::Refuse(_)
        ));
        assert_eq!(evaluate_write(&z, &effect, true), LightingWriteDecision::Allow);
    }

    #[test]
    fn ownership_detects_conflicts() {
        assert_eq!(check_lighting_ownership(["explorer.exe"]), OwnershipState::Clear);
        assert!(matches!(
            check_lighting_ownership(["OpenRGB.exe"]),
            OwnershipState::Conflict { .. }
        ));
    }
}
