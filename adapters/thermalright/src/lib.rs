//! Thermalright Trofeo Vision LCD transport (P1-09/P1-10).
//!
//! Status: **transport stub pending hardware-in-the-loop validation.** The
//! physical unit is ordered but not yet validated (config/hardware-profile),
//! so the real USB VID/PID, endpoints and wire format are unknown. Per
//! PHASE-1-SPEC §8 and §13, we must confirm the actual USB identity and protocol
//! against the delivered device before enabling any writes.
//!
//! What is implemented now and testable without the device:
//! - The target panel geometry (1280x480).
//! - Frame-dimension validation via the shared `DisplayTransport` interface, so a
//!   malformed frame is rejected at the transport boundary (TEST-PLAN "Safety
//!   regression").
//! - A connection/health state machine with no real I/O; `send_frame` returns
//!   `NotValidated` until the protocol is confirmed.

use shookshmam_lcd_renderer::{
    check_frame_dimensions, DisplayTransport, Frame, TransportError, TransportHealth,
};

/// Target panel resolution (confirm against the delivered unit — §13).
pub const TARGET_WIDTH: u32 = 1280;
pub const TARGET_HEIGHT: u32 = 480;

/// Candidate USB identity — **unverified**. Do not ship a udev rule or driver
/// binding based on these until confirmed on the real device.
pub const UNVERIFIED_USB_VID: u16 = 0x0000;
pub const UNVERIFIED_USB_PID: u16 = 0x0000;

/// Stub transport for the Thermalright panel. Performs full frame validation but
/// never touches hardware until the protocol is confirmed.
pub struct ThermalrightTransport {
    health: TransportHealth,
}

impl ThermalrightTransport {
    pub fn new() -> Self {
        ThermalrightTransport { health: TransportHealth::AwaitingValidation }
    }
}

impl Default for ThermalrightTransport {
    fn default() -> Self {
        Self::new()
    }
}

impl DisplayTransport for ThermalrightTransport {
    fn target_resolution(&self) -> (u32, u32) {
        (TARGET_WIDTH, TARGET_HEIGHT)
    }

    fn health(&self) -> TransportHealth {
        self.health
    }

    fn send_frame(&mut self, frame: &Frame) -> Result<(), TransportError> {
        // Dimension check happens first, regardless of connection state, so the
        // validation is enforced and unit-testable now.
        check_frame_dimensions(frame, self.target_resolution())?;
        // Protocol unverified: refuse to write until HIL validation (ADR-0002).
        Err(TransportError::NotValidated)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use shookshmam_lcd_renderer::{test_pattern, Frame, Rgba};

    #[test]
    fn rejects_wrong_frame_dimensions() {
        let mut t = ThermalrightTransport::new();
        let wrong = Frame::new(640, 480, Rgba::BLACK).unwrap();
        assert!(matches!(
            t.send_frame(&wrong),
            Err(TransportError::FrameDimensionMismatch { .. })
        ));
    }

    #[test]
    fn correct_dimensions_pass_validation_but_writes_are_disabled() {
        let mut t = ThermalrightTransport::new();
        let frame = test_pattern(TARGET_WIDTH, TARGET_HEIGHT).unwrap();
        // Validation passes, but Phase 1 refuses the actual write.
        assert_eq!(t.send_frame(&frame), Err(TransportError::NotValidated));
    }

    #[test]
    fn starts_awaiting_validation() {
        let t = ThermalrightTransport::new();
        assert_eq!(t.health(), TransportHealth::AwaitingValidation);
        assert_eq!(t.target_resolution(), (1280, 480));
    }
}
