//! Lighting zone model (PHASE-4-SPEC §4). Electrical type must be known before
//! any write: sending a 12V RGB command to a 5V ARGB header (or vice versa) can
//! damage hardware, and software device names must never be used to infer it.

use serde::{Deserialize, Serialize};
use shookshmam_device_model::{ConnectionState, ValidationStatus};

/// A plain 8-bit-per-channel color (no alpha).
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub struct Rgb {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl Rgb {
    pub const BLACK: Rgb = Rgb { r: 0, g: 0, b: 0 };

    pub const fn new(r: u8, g: u8, b: u8) -> Self {
        Rgb { r, g, b }
    }

    /// Scale brightness by `factor` in `[0,1]`.
    pub fn scaled(self, factor: f64) -> Rgb {
        let f = factor.clamp(0.0, 1.0);
        Rgb {
            r: (self.r as f64 * f).round() as u8,
            g: (self.g as f64 * f).round() as u8,
            b: (self.b as f64 * f).round() as u8,
        }
    }

    /// Linear interpolation between two colors, `t` clamped to `[0,1]`.
    pub fn lerp(a: Rgb, b: Rgb, t: f64) -> Rgb {
        let t = t.clamp(0.0, 1.0);
        let m = |x: u8, y: u8| (x as f64 + (y as f64 - x as f64) * t).round() as u8;
        Rgb { r: m(a.r, b.r), g: m(a.g, b.g), b: m(a.b, b.b) }
    }

    /// HSV → RGB. `h` in degrees, `s`/`v` in `[0,1]`.
    pub fn from_hsv(h: f64, s: f64, v: f64) -> Rgb {
        let h = h.rem_euclid(360.0);
        let s = s.clamp(0.0, 1.0);
        let v = v.clamp(0.0, 1.0);
        let c = v * s;
        let x = c * (1.0 - ((h / 60.0) % 2.0 - 1.0).abs());
        let m = v - c;
        let (r, g, b) = match h as u32 / 60 {
            0 => (c, x, 0.0),
            1 => (x, c, 0.0),
            2 => (0.0, c, x),
            3 => (0.0, x, c),
            4 => (x, 0.0, c),
            _ => (c, 0.0, x),
        };
        let to = |n: f64| ((n + m) * 255.0).round() as u8;
        Rgb { r: to(r), g: to(g), b: to(b) }
    }
}

/// Physical interface type. `Unknown` disables all writes (spec §4).
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ElectricalType {
    Argb5v3pin,
    Rgb12v4pin,
    Unknown,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum EffectClass {
    Static,
    Breathing,
    ColorCycle,
    Gradient,
    Off,
}

/// One addressable lighting zone.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LightingZone {
    pub id: String,
    pub name: String,
    pub controller: String,
    pub connector: String,
    pub electrical_type: ElectricalType,
    /// LED count/topology where known; `None` means unknown topology (writes off).
    pub led_count: Option<u32>,
    pub supported_effects: Vec<EffectClass>,
    /// Whether the controller can read back current state for save/restore.
    pub readback_available: bool,
    pub validation: ValidationStatus,
    pub connection: ConnectionState,
}

impl LightingZone {
    pub fn new(
        id: impl Into<String>,
        name: impl Into<String>,
        controller: impl Into<String>,
        connector: impl Into<String>,
    ) -> Self {
        LightingZone {
            id: id.into(),
            name: name.into(),
            controller: controller.into(),
            connector: connector.into(),
            electrical_type: ElectricalType::Unknown,
            led_count: None,
            supported_effects: Vec::new(),
            readback_available: false,
            validation: ValidationStatus::Unvalidated,
            connection: ConnectionState::Unknown,
        }
    }

    /// Whether this zone may receive writes at all: known electrical type, known
    /// topology, validated, and connected (spec §4, §6).
    pub fn writable(&self) -> bool {
        self.electrical_type != ElectricalType::Unknown
            && self.led_count.is_some()
            && self.validation == ValidationStatus::Validated
            && self.connection == ConnectionState::Connected
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hsv_primaries() {
        assert_eq!(Rgb::from_hsv(0.0, 1.0, 1.0), Rgb::new(255, 0, 0));
        assert_eq!(Rgb::from_hsv(120.0, 1.0, 1.0), Rgb::new(0, 255, 0));
        assert_eq!(Rgb::from_hsv(240.0, 1.0, 1.0), Rgb::new(0, 0, 255));
        assert_eq!(Rgb::from_hsv(0.0, 0.0, 0.0), Rgb::BLACK);
    }

    #[test]
    fn scaling_and_lerp() {
        assert_eq!(Rgb::new(200, 100, 50).scaled(0.0), Rgb::BLACK);
        assert_eq!(Rgb::new(200, 100, 50).scaled(1.0), Rgb::new(200, 100, 50));
        assert_eq!(Rgb::lerp(Rgb::BLACK, Rgb::new(255, 255, 255), 0.5), Rgb::new(128, 128, 128));
    }

    #[test]
    fn unknown_electrical_type_blocks_writes() {
        let mut z = LightingZone::new("z", "Zone", "ctl", "hdr1");
        z.validation = ValidationStatus::Validated;
        z.connection = ConnectionState::Connected;
        z.led_count = Some(8);
        // Electrical type still Unknown -> not writable.
        assert!(!z.writable());
        z.electrical_type = ElectricalType::Argb5v3pin;
        assert!(z.writable());
    }

    #[test]
    fn unknown_topology_blocks_writes() {
        let mut z = LightingZone::new("z", "Zone", "ctl", "hdr1");
        z.electrical_type = ElectricalType::Argb5v3pin;
        z.validation = ValidationStatus::Validated;
        z.connection = ConnectionState::Connected;
        z.led_count = None; // unknown topology
        assert!(!z.writable());
    }
}
