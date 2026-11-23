//! ARGB and lighting control (Phase 4): zones, effects, a deterministic effect
//! engine, the write-policy guard and a gated controller interface.
//!
//! Safety stance (ADR-0002, PHASE-4-SPEC §6): read-only discovery precedes all
//! writes; an unknown electrical type or topology disables writes; zones must be
//! validated; the first write to a zone requires explicit confirmation; software
//! names are never used to infer header voltage.

mod controller;
mod effect;
mod engine;
mod policy;
mod zone;

pub use controller::{GatedLightingController, LightingController, LightingError, ZoneState};
pub use effect::{Effect, EffectError, MAX_GRADIENT_COLORS, MAX_SPEED, MIN_SPEED};
pub use engine::render;
pub use policy::{
    check_lighting_ownership, evaluate_write, LightingWriteDecision, OwnershipState,
};
pub use zone::{EffectClass, ElectricalType, LightingZone, Rgb};
