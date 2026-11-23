//! Profiles, automation and release hardening (Phase 5): a declarative profile
//! schema with versioned migration, a transactional apply/rollback orchestrator,
//! built-in presets and a sanitized support-bundle builder.
//!
//! Safety stance (PHASE-5-SPEC §4, §9): profiles are data only (no scripts);
//! applying to hardware requires confirmation; a partial application is never
//! reported as fully successful; and no control operation bypasses the Phase 3/4
//! safety policies.

mod apply;
mod migrate;
mod model;
mod presets;
mod support;

pub use apply::{
    apply, plan, ApplyPlan, ApplyReport, ChangeKind, DeviceEnvironment, PlannedChange,
};
pub use migrate::migrate;
pub use model::{
    ControlModePref, Platform, Profile, ProfileError, CURRENT_SCHEMA_VERSION,
};
pub use presets::preset;
pub use support::{sanitize_line, SupportBundle};
