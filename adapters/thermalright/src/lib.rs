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
//! - A `DisplayTransport` trait and frame-dimension validation, so a malformed
//!   frame is rejected at the transport boundary (TEST-PLAN "Safety regression").
//! - A connection/health state machine with no real I/O; `send_frame` returns
//!   `NotValidated` until the protocol is confirmed.

use shookshmam_lcd_renderer::Frame;

/// Target panel resolution (confirm against the delivered unit — §13).
pub const TARGET_WIDTH: u32 = 1280;
pub const TARGET_HEIGHT: u32 = 480;

/// Candidate USB identity — **unverified**. Do not ship a udev rule or driver
/// binding based on these until confirmed on the real device.
pub const UNVERIFIED_USB_VID: u16 = 0x0000;
pub const UNVERIFIED_USB_PID: u16 = 0x0000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransportHealth {
    Disconnected,
    Connected,
    /// Device present but protocol not yet validated (Phase 1 default).
    AwaitingValidation,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TransportError {
    /// Frame size does not match the display's expected resolution.
    FrameDimensionMismatch { expected: (u32, u32), got: (u32, u32) },
    /// The device/protocol has not been validated; writes are disabled.
    NotValidated,
    /// The display is not connected.
    Disconnected,
    Io(String),
}

impl std::fmt::Display for TransportError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TransportError::FrameDimensionMismatch { expected, got } => write!(
                f,
                "frame {}x{} does not match display {}x{}",
                got.0, got.1, expected.0, expected.1
            ),
            TransportError::NotValidated => {
                write!(f, "device protocol not validated; writes disabled")
            }
            TransportError::Disconnected => write!(f, "display disconnected"),
            TransportError::Io(e) => write!(f, "transport I/O error: {e}"),
        }
    }
}

impl std::error::Error for TransportError {}

/// A target interface (ARCHITECTURE.md §5): enumerate displays, write frames,
/// report health. No fan/RGB/control writes — this is image output only.
pub trait DisplayTransport {
    fn target_resolution(&self) -> (u32, u32);
    fn health(&self) -> TransportHealth;
    /// Validate and send a frame. Must reject a frame whose dimensions do not
    /// match the display before any bytes are written.
    fn send_frame(&mut self, frame: &Frame) -> Result<(), TransportError>;
}

/// Stub transport for the Thermalright panel. Performs full frame validation but
/// never touches hardware until the protocol is confirmed.
pub struct ThermalrightTransport {
    health: TransportHealth,
}

impl ThermalrightTransport {
    pub fn new() -> Self {
        ThermalrightTransport { health: TransportHealth::AwaitingValidation }
    }

    /// Validate that a frame matches the panel geometry. Exposed so the renderer
    /// side can check before attempting a send.
    pub fn validate_frame(&self, frame: &Frame) -> Result<(), TransportError> {
        if frame.width() != TARGET_WIDTH || frame.height() != TARGET_HEIGHT {
            return Err(TransportError::FrameDimensionMismatch {
                expected: (TARGET_WIDTH, TARGET_HEIGHT),
                got: (frame.width(), frame.height()),
            });
        }
        Ok(())
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
        self.validate_frame(frame)?;
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
        assert!(t.validate_frame(&frame).is_ok());
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
