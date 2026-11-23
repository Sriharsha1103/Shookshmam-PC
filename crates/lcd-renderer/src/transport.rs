//! Shared display-transport interface (ARCHITECTURE.md §5). Each device adapter
//! (Thermalright, Lian Li TL) implements this to send validated frames; image
//! output only — never fan/RGB/control writes.

use crate::Frame;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransportHealth {
    Disconnected,
    Connected,
    /// Device present but protocol not yet validated (read-only default).
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

/// Enumerate displays, write frames, report health.
pub trait DisplayTransport {
    fn target_resolution(&self) -> (u32, u32);
    fn health(&self) -> TransportHealth;
    /// Validate and send a frame. Must reject a frame whose dimensions do not
    /// match the display before any bytes are written.
    fn send_frame(&mut self, frame: &Frame) -> Result<(), TransportError>;
}

/// Shared helper: confirm a frame matches a target resolution.
pub fn check_frame_dimensions(
    frame: &Frame,
    target: (u32, u32),
) -> Result<(), TransportError> {
    if frame.width() != target.0 || frame.height() != target.1 {
        return Err(TransportError::FrameDimensionMismatch {
            expected: target,
            got: (frame.width(), frame.height()),
        });
    }
    Ok(())
}
