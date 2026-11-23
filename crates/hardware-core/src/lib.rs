//! Platform-neutral hardware access traits plus the discovery registry that
//! aggregates providers and isolates their failures.
//!
//! Phase 1 intentionally exposes **no** hardware-control trait or write command
//! (ADR-0002). Only read traits and a display-frame transport (later) exist.

mod diagnostics;
pub mod mock;
mod provider;
mod registry;

pub use diagnostics::Diagnostics;
pub use provider::{HardwareError, SensorProvider};
pub use registry::{CollectResult, ProviderStatus, Registry};

/// Current UTC time in Unix milliseconds. Centralized so tests can avoid it and
/// pass explicit timestamps instead.
pub fn now_unix_ms() -> i64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}
