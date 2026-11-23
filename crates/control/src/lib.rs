//! Fan and cooling control (Phase 3): fan-curve model, automatic evaluation,
//! the central safety policy, manual override, watchdog, audit and the gated
//! fan-controller interface.
//!
//! Safety stance (ADR-0002, PHASE-3-SPEC §7): the application must fail safely
//! and must never be the only protection against overheating. Every write passes
//! the [`SafetyPolicy`]; control is disabled per channel by default; channels
//! refuse writes until validated; invalid/stale/missing sensors never produce a
//! low-duty command; BIOS/firmware remains the fallback.

mod audit;
mod auto;
mod controller;
mod curve;
mod manual;
mod policy;
mod watchdog;

pub use audit::{AuditLog, ControlEvent, ControlEventKind};
pub use auto::{AutoController, AutoOutcome, TempSample};
pub use controller::{ChannelInfo, ControlError, FanController, GatedFanController};
pub use curve::{preset, CurveError, CurvePoint, FanCurve, SensorLossBehavior, SourcePolicy};
pub use manual::{ManualOverride, DEFAULT_MANUAL_DURATION_MS};
pub use policy::{ChannelBounds, CommandSource, ControlCommand, Decision, SafetyPolicy};
pub use watchdog::Watchdog;
