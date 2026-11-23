//! Effect schema and validation (PHASE-4-SPEC §5).

use crate::zone::{EffectClass, Rgb};
use serde::{Deserialize, Serialize};

pub const MIN_SPEED: u8 = 1;
pub const MAX_SPEED: u8 = 10;
pub const MAX_GRADIENT_COLORS: usize = 16;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Effect {
    Static { color: Rgb },
    Breathing { color: Rgb, speed: u8 },
    ColorCycle { speed: u8, brightness: u8 },
    Gradient { colors: Vec<Rgb>, speed: u8 },
    Off,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EffectError {
    SpeedOutOfRange,
    BrightnessOutOfRange,
    GradientNeedsTwoColors,
    TooManyGradientColors,
}

impl std::fmt::Display for EffectError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            EffectError::SpeedOutOfRange => "speed must be within 1..=10",
            EffectError::BrightnessOutOfRange => "brightness must be within 0..=100",
            EffectError::GradientNeedsTwoColors => "a gradient needs at least two colors",
            EffectError::TooManyGradientColors => "too many gradient colors",
        };
        f.write_str(s)
    }
}

impl std::error::Error for EffectError {}

fn check_speed(speed: u8) -> Result<(), EffectError> {
    if (MIN_SPEED..=MAX_SPEED).contains(&speed) {
        Ok(())
    } else {
        Err(EffectError::SpeedOutOfRange)
    }
}

impl Effect {
    pub fn class(&self) -> EffectClass {
        match self {
            Effect::Static { .. } => EffectClass::Static,
            Effect::Breathing { .. } => EffectClass::Breathing,
            Effect::ColorCycle { .. } => EffectClass::ColorCycle,
            Effect::Gradient { .. } => EffectClass::Gradient,
            Effect::Off => EffectClass::Off,
        }
    }

    /// Validate parameter ranges. Does not check zone support (the policy layer
    /// does that against the zone's `supported_effects`).
    pub fn validate(&self) -> Result<(), EffectError> {
        match self {
            Effect::Static { .. } | Effect::Off => Ok(()),
            Effect::Breathing { speed, .. } => check_speed(*speed),
            Effect::ColorCycle { speed, brightness } => {
                check_speed(*speed)?;
                if *brightness > 100 {
                    return Err(EffectError::BrightnessOutOfRange);
                }
                Ok(())
            }
            Effect::Gradient { colors, speed } => {
                check_speed(*speed)?;
                if colors.len() < 2 {
                    return Err(EffectError::GradientNeedsTwoColors);
                }
                if colors.len() > MAX_GRADIENT_COLORS {
                    return Err(EffectError::TooManyGradientColors);
                }
                Ok(())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_effects_pass() {
        assert!(Effect::Static { color: Rgb::new(10, 20, 30) }.validate().is_ok());
        assert!(Effect::Off.validate().is_ok());
        assert!(Effect::Breathing { color: Rgb::BLACK, speed: 5 }.validate().is_ok());
        assert!(Effect::ColorCycle { speed: 3, brightness: 80 }.validate().is_ok());
        assert!(Effect::Gradient { colors: vec![Rgb::BLACK, Rgb::new(255, 0, 0)], speed: 2 }
            .validate()
            .is_ok());
    }

    #[test]
    fn invalid_params_rejected() {
        assert_eq!(
            Effect::Breathing { color: Rgb::BLACK, speed: 0 }.validate(),
            Err(EffectError::SpeedOutOfRange)
        );
        assert_eq!(
            Effect::ColorCycle { speed: 3, brightness: 200 }.validate(),
            Err(EffectError::BrightnessOutOfRange)
        );
        assert_eq!(
            Effect::Gradient { colors: vec![Rgb::BLACK], speed: 2 }.validate(),
            Err(EffectError::GradientNeedsTwoColors)
        );
    }
}
