//! Display-independent dashboard layout and RGBA frame rendering (P1-09/P1-10).
//!
//! Device transport (USB wire format) lives in the Thermalright adapter and is
//! implemented only after hardware-in-the-loop validation.

mod frame;
mod layout;

pub use frame::{Frame, Rgba};
pub use layout::{render, test_pattern, Layout, LayoutError, Rect, Widget};
