//! Lian Li Uni Fan TL LCD controller adapter (Phase 2).
//!
//! Status: **device layer, protocol stub pending HIL validation + licence
//! review (P2-01).** The real USB protocol for the TL controller is not yet
//! validated, so fan telemetry reads and all LCD/RGB writes are refused. What is
//! implemented and fully testable without the device:
//!
//! - Stable controller+port → logical device ID mapping (P2-03).
//! - Capability probing that keeps unknown revisions read-only (P2-02).
//! - Single-owner / ownership-conflict detection vs. L-Connect / OpenRGB (§7).
//! - A gated `DisplayTransport` for TL screens and a gated fan telemetry
//!   provider, both refusing until validated.

use shookshmam_device_model::{
    logical_device_id, Capability, DeviceRecord, DeviceType, ValidationStatus,
};
use shookshmam_hardware_core::{HardwareError, SensorProvider};
use shookshmam_lcd_renderer::{DisplayTransport, Frame, TransportError, TransportHealth};
use shookshmam_sensor_model::{DeviceDescriptor, SensorReading};

pub const SOURCE: &str = "lianli";

/// Known-validated controller revisions. Empty until a real unit is tested, so
/// every device currently resolves to the safe read-only path.
const VALIDATED_REVISIONS: &[&str] = &[];

/// A detected (or modeled) TL controller and the devices behind it.
pub struct LianLiController {
    controller_id: String,
    revision: Option<String>,
}

impl LianLiController {
    pub fn new(controller_id: impl Into<String>, revision: Option<String>) -> Self {
        LianLiController { controller_id: controller_id.into(), revision }
    }

    fn revision_is_validated(&self) -> bool {
        self.revision
            .as_deref()
            .map(|r| VALIDATED_REVISIONS.contains(&r))
            .unwrap_or(false)
    }

    /// Validation to assign to a write capability on this controller: `Validated`
    /// only for a known-good revision, otherwise the safe `Unvalidated` default.
    fn write_validation(&self) -> ValidationStatus {
        if self.revision_is_validated() {
            ValidationStatus::Validated
        } else {
            ValidationStatus::Unvalidated
        }
    }

    /// Probe the controller topology (P2-02/P2-03). Produces the controller plus
    /// `fan_count` fan devices with stable logical IDs. Connection defaults to
    /// `Unknown` because this is a model, not a live read.
    pub fn probe(&self, fan_count: u8, now_ms: i64) -> Vec<DeviceRecord> {
        let mut out = Vec::new();

        out.push(
            DeviceRecord::new(
                logical_device_id(&self.controller_id, 0),
                "Lian Li TL Controller",
                DeviceType::Controller,
                SOURCE,
                now_ms,
            )
            .with_capability(Capability::LcdFrameWrite, self.write_validation())
            .with_capability(Capability::RgbEffectWrite, self.write_validation()),
        );

        for port in 1..=fan_count {
            out.push(
                DeviceRecord::new(
                    logical_device_id(&self.controller_id, port),
                    format!("TL LCD Fan (port {port})"),
                    DeviceType::Fan,
                    SOURCE,
                    now_ms,
                )
                // Fan RPM read is a read capability; still unvalidated until a
                // real tachometer source is confirmed, but never fabricated.
                .with_capability(Capability::FanRpmRead, ValidationStatus::Unvalidated)
                .with_capability(Capability::RgbEffectWrite, self.write_validation()),
            );
        }

        out
    }
}

// ---------------------------------------------------------------------------
// Ownership / single-owner policy (§7)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OwnershipState {
    /// No competing controller software detected; safe to own the device.
    Clear,
    /// Another process may be driving the controller; do not take ownership.
    Conflict { competitor: String },
}

/// Processes known to drive the Lian Li controller and conflict with us.
const COMPETITORS: &[&str] = &["L-Connect", "LConnect", "OpenRGB", "lianli", "lian-li-listener"];

/// Decide whether we can safely own the controller given the running processes.
pub fn check_ownership<I, S>(running_processes: I) -> OwnershipState
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    for proc in running_processes {
        let name = proc.as_ref();
        if let Some(hit) = COMPETITORS
            .iter()
            .find(|c| name.to_ascii_lowercase().contains(&c.to_ascii_lowercase()))
        {
            return OwnershipState::Conflict { competitor: (*hit).to_string() };
        }
    }
    OwnershipState::Clear
}

// ---------------------------------------------------------------------------
// Fan telemetry provider (P2-04) — gated until protocol validated
// ---------------------------------------------------------------------------

pub struct LianLiFanProvider;

impl LianLiFanProvider {
    pub fn new() -> Self {
        LianLiFanProvider
    }
}

impl Default for LianLiFanProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl SensorProvider for LianLiFanProvider {
    fn provider_id(&self) -> &'static str {
        SOURCE
    }

    fn discover(&self) -> Result<Vec<DeviceDescriptor>, HardwareError> {
        // Protocol not validated: report no devices rather than fabricate any.
        Err(HardwareError::NotImplemented)
    }

    fn read_snapshot(&self, _device_id: &str) -> Result<Vec<SensorReading>, HardwareError> {
        Err(HardwareError::NotImplemented)
    }
}

// ---------------------------------------------------------------------------
// TL LCD transport (P2-05/P2-06) — gated until protocol validated
// ---------------------------------------------------------------------------

/// Candidate TL LCD geometry — **unconfirmed**. The TL screens are small; the
/// exact pixel format/resolution must be read from the device (§4).
pub struct LianLiLcdTransport {
    resolution: Option<(u32, u32)>,
}

impl LianLiLcdTransport {
    pub fn new() -> Self {
        LianLiLcdTransport { resolution: None }
    }
}

impl Default for LianLiLcdTransport {
    fn default() -> Self {
        Self::new()
    }
}

impl DisplayTransport for LianLiLcdTransport {
    fn target_resolution(&self) -> (u32, u32) {
        self.resolution.unwrap_or((0, 0))
    }

    fn health(&self) -> TransportHealth {
        TransportHealth::AwaitingValidation
    }

    fn send_frame(&mut self, _frame: &Frame) -> Result<(), TransportError> {
        // Resolution/protocol unknown; refuse before any bytes are written.
        Err(TransportError::NotValidated)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn probe_builds_controller_and_fans_with_stable_ids() {
        let ctl = LianLiController::new("TL-SERIAL-1", None);
        let devices = ctl.probe(3, 1000);
        assert_eq!(devices.len(), 4); // 1 controller + 3 fans
        assert_eq!(devices[0].device_type, DeviceType::Controller);
        assert_eq!(devices[1].id, "lianli:TL-SERIAL-1:port1");
        assert_eq!(devices[3].id, "lianli:TL-SERIAL-1:port3");
    }

    #[test]
    fn unknown_revision_keeps_writes_unvalidated() {
        let ctl = LianLiController::new("TL-SERIAL-1", Some("rev-unknown".into()));
        let devices = ctl.probe(1, 1000);
        let controller = &devices[0];
        // LCD/RGB writes must not be usable on an unvalidated revision.
        assert!(!controller.can_write(Capability::LcdFrameWrite));
        assert!(!controller.can_write(Capability::RgbEffectWrite));
    }

    #[test]
    fn ownership_detects_competitors() {
        assert_eq!(check_ownership(["explorer.exe", "steam"]), OwnershipState::Clear);
        assert_eq!(
            check_ownership(["L-Connect 3.exe"]),
            OwnershipState::Conflict { competitor: "L-Connect".into() }
        );
        assert!(matches!(
            check_ownership(["openrgb"]),
            OwnershipState::Conflict { .. }
        ));
    }

    #[test]
    fn fan_provider_refuses_until_validated() {
        let p = LianLiFanProvider::new();
        assert!(matches!(p.discover(), Err(HardwareError::NotImplemented)));
    }

    #[test]
    fn lcd_transport_refuses_writes() {
        let mut t = LianLiLcdTransport::new();
        let frame = Frame::new(100, 100, shookshmam_lcd_renderer::Rgba::BLACK).unwrap();
        assert_eq!(t.send_frame(&frame), Err(TransportError::NotValidated));
        assert_eq!(t.health(), TransportHealth::AwaitingValidation);
    }
}
