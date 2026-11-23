//! Phase 4 safety-regression integration tests: unknown/unverified zones can
//! never receive writes, and the effect engine adapts to LED count.

use shookshmam_device_model::{ConnectionState, ValidationStatus};
use shookshmam_lighting::{
    check_lighting_ownership, evaluate_write, render, Effect, EffectClass, ElectricalType,
    GatedLightingController, LightingController, LightingError, LightingWriteDecision, LightingZone,
    OwnershipState, Rgb,
};

fn validated_zone() -> LightingZone {
    let mut z = LightingZone::new("mobo_argb", "Motherboard ARGB", "asus", "argb_header_1");
    z.electrical_type = ElectricalType::Argb5v3pin;
    z.led_count = Some(12);
    z.validation = ValidationStatus::Validated;
    z.connection = ConnectionState::Connected;
    z.supported_effects = vec![EffectClass::Static, EffectClass::ColorCycle, EffectClass::Off];
    z
}

#[test]
fn unknown_electrical_type_can_never_be_written() {
    // A real header whose electrical type has not been physically verified.
    let mut z = validated_zone();
    z.electrical_type = ElectricalType::Unknown;
    let decision = evaluate_write(&z, &Effect::Static { color: Rgb::new(255, 0, 0) }, true);
    assert!(matches!(decision, LightingWriteDecision::Refuse(_)));

    let mut ctl = GatedLightingController::new(vec![z]);
    let err = ctl.apply("mobo_argb", &Effect::Static { color: Rgb::new(255, 0, 0) }, true).unwrap_err();
    assert!(matches!(err, LightingError::ZoneUnwritable(_)));
}

#[test]
fn validated_zone_allowed_by_policy_but_no_backend_yet() {
    let z = validated_zone();
    let decision = evaluate_write(&z, &Effect::ColorCycle { speed: 5, brightness: 80 }, true);
    assert_eq!(decision, LightingWriteDecision::Allow);

    // Controller still refuses because no validated hardware backend exists.
    let mut ctl = GatedLightingController::new(vec![z]);
    assert_eq!(
        ctl.apply("mobo_argb", &Effect::ColorCycle { speed: 5, brightness: 80 }, true),
        Err(LightingError::Unsupported)
    );
}

#[test]
fn first_write_requires_confirmation() {
    let z = validated_zone();
    assert!(matches!(
        evaluate_write(&z, &Effect::Static { color: Rgb::BLACK }, false),
        LightingWriteDecision::Refuse(_)
    ));
}

#[test]
fn ownership_conflict_blocks_takeover() {
    assert!(matches!(
        check_lighting_ownership(["OpenRGB"]),
        OwnershipState::Conflict { .. }
    ));
    assert_eq!(check_lighting_ownership(["steam", "explorer"]), OwnershipState::Clear);
}

#[test]
fn effect_engine_matches_led_count() {
    let out = render(&Effect::Static { color: Rgb::new(1, 2, 3) }, 12, 0);
    assert_eq!(out.len(), 12);
    assert!(out.iter().all(|c| *c == Rgb::new(1, 2, 3)));
}
