//! Fan-curve model, validation and interpolation (PHASE-3-SPEC §4).

use serde::{Deserialize, Serialize};

/// How to combine multiple temperature sources into one controlling value.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SourcePolicy {
    /// Use the hottest source (safest default).
    Max,
    /// Use the mean of the sources.
    Average,
}

/// What to do when no valid temperature input is available.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum SensorLossBehavior {
    /// Drive a known-safe duty (must be within channel bounds).
    SafeFallbackDuty(f64),
    /// Hand control back to BIOS/firmware.
    ReleaseToFirmware,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct CurvePoint {
    pub temperature_c: f64,
    pub duty_percent: f64,
}

impl CurvePoint {
    pub fn new(temperature_c: f64, duty_percent: f64) -> Self {
        CurvePoint { temperature_c, duty_percent }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FanCurve {
    pub id: String,
    pub name: String,
    pub temp_source_ids: Vec<String>,
    pub source_policy: SourcePolicy,
    pub points: Vec<CurvePoint>,
    pub min_duty: f64,
    pub max_duty: f64,
    /// Temperature band (°C) that must be exceeded before the controlling
    /// temperature moves, to damp oscillation.
    pub hysteresis_c: f64,
    pub ramp_up_pct_per_s: f64,
    pub ramp_down_pct_per_s: f64,
    pub sensor_loss: SensorLossBehavior,
    pub channels: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CurveError {
    TooFewPoints,
    NonFiniteValue,
    TemperaturesNotStrictlyIncreasing,
    DutyNotMonotonic,
    DutyOutOfRange,
    InvalidDutyBounds,
    NoTemperatureSources,
    NonPositiveRamp,
    FallbackOutOfBounds,
}

impl std::fmt::Display for CurveError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            CurveError::TooFewPoints => "a curve needs at least two points",
            CurveError::NonFiniteValue => "temperatures and duties must be finite",
            CurveError::TemperaturesNotStrictlyIncreasing => {
                "curve temperatures must be strictly increasing"
            }
            CurveError::DutyNotMonotonic => "curve duties must be non-decreasing",
            CurveError::DutyOutOfRange => "duties must be within [min_duty, max_duty] and 0..=100",
            CurveError::InvalidDutyBounds => "min_duty must be <= max_duty and within 0..=100",
            CurveError::NoTemperatureSources => "a curve needs at least one temperature source",
            CurveError::NonPositiveRamp => "ramp limits must be positive",
            CurveError::FallbackOutOfBounds => "sensor-loss fallback duty is outside channel bounds",
        };
        f.write_str(s)
    }
}

impl std::error::Error for CurveError {}

impl FanCurve {
    /// Validate all structural + safety rules. The caller must surface the error
    /// to the user and never silently "fix" an unsafe curve (spec §4).
    pub fn validate(&self) -> Result<(), CurveError> {
        if !(0.0..=100.0).contains(&self.min_duty)
            || !(0.0..=100.0).contains(&self.max_duty)
            || self.min_duty > self.max_duty
        {
            return Err(CurveError::InvalidDutyBounds);
        }
        if self.temp_source_ids.is_empty() {
            return Err(CurveError::NoTemperatureSources);
        }
        if self.points.len() < 2 {
            return Err(CurveError::TooFewPoints);
        }
        if self.ramp_up_pct_per_s <= 0.0 || self.ramp_down_pct_per_s <= 0.0 {
            return Err(CurveError::NonPositiveRamp);
        }
        for p in &self.points {
            if !p.temperature_c.is_finite() || !p.duty_percent.is_finite() {
                return Err(CurveError::NonFiniteValue);
            }
            if p.duty_percent < self.min_duty || p.duty_percent > self.max_duty {
                return Err(CurveError::DutyOutOfRange);
            }
        }
        for w in self.points.windows(2) {
            if w[1].temperature_c <= w[0].temperature_c {
                return Err(CurveError::TemperaturesNotStrictlyIncreasing);
            }
            if w[1].duty_percent < w[0].duty_percent {
                return Err(CurveError::DutyNotMonotonic);
            }
        }
        if let SensorLossBehavior::SafeFallbackDuty(d) = self.sensor_loss {
            if !d.is_finite() || d < self.min_duty || d > self.max_duty {
                return Err(CurveError::FallbackOutOfBounds);
            }
        }
        Ok(())
    }

    /// Duty for a temperature, piecewise-linear between points and clamped to the
    /// curve's duty bounds. Assumes a validated curve.
    pub fn duty_for_temperature(&self, temp_c: f64) -> f64 {
        let pts = &self.points;
        let raw = if temp_c <= pts[0].temperature_c {
            pts[0].duty_percent
        } else if temp_c >= pts[pts.len() - 1].temperature_c {
            pts[pts.len() - 1].duty_percent
        } else {
            let mut duty = pts[pts.len() - 1].duty_percent;
            for w in pts.windows(2) {
                let (a, b) = (w[0], w[1]);
                if temp_c >= a.temperature_c && temp_c <= b.temperature_c {
                    let span = b.temperature_c - a.temperature_c;
                    let t = (temp_c - a.temperature_c) / span;
                    duty = a.duty_percent + (b.duty_percent - a.duty_percent) * t;
                    break;
                }
            }
            duty
        };
        raw.clamp(self.min_duty, self.max_duty)
    }

    /// Combine fresh temperature values per the source policy.
    pub fn combine(&self, temps: &[f64]) -> Option<f64> {
        if temps.is_empty() {
            return None;
        }
        Some(match self.source_policy {
            SourcePolicy::Max => temps.iter().cloned().fold(f64::NEG_INFINITY, f64::max),
            SourcePolicy::Average => temps.iter().sum::<f64>() / temps.len() as f64,
        })
    }
}

/// Built-in starting presets (spec §2). Callers assign real temp-source and
/// channel IDs before use.
pub fn preset(name: &str) -> Option<FanCurve> {
    let base = |id: &str, display: &str, points: Vec<CurvePoint>, min: f64| FanCurve {
        id: id.to_string(),
        name: display.to_string(),
        temp_source_ids: vec!["cpu.temperature.tctl".to_string()],
        source_policy: SourcePolicy::Max,
        points,
        min_duty: min,
        max_duty: 100.0,
        hysteresis_c: 2.0,
        ramp_up_pct_per_s: 20.0,
        ramp_down_pct_per_s: 8.0,
        sensor_loss: SensorLossBehavior::SafeFallbackDuty(100.0),
        channels: Vec::new(),
    };
    match name {
        "quiet" => Some(base(
            "quiet",
            "Quiet",
            vec![
                CurvePoint::new(30.0, 20.0),
                CurvePoint::new(55.0, 35.0),
                CurvePoint::new(75.0, 65.0),
                CurvePoint::new(85.0, 100.0),
            ],
            20.0,
        )),
        "balanced" => Some(base(
            "balanced",
            "Balanced",
            vec![
                CurvePoint::new(30.0, 30.0),
                CurvePoint::new(50.0, 45.0),
                CurvePoint::new(70.0, 75.0),
                CurvePoint::new(82.0, 100.0),
            ],
            30.0,
        )),
        "performance" => Some(base(
            "performance",
            "Performance",
            vec![
                CurvePoint::new(30.0, 50.0),
                CurvePoint::new(50.0, 70.0),
                CurvePoint::new(65.0, 90.0),
                CurvePoint::new(75.0, 100.0),
            ],
            50.0,
        )),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn approx(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-9
    }

    fn ok_curve() -> FanCurve {
        preset("balanced").unwrap()
    }

    #[test]
    fn presets_validate() {
        for name in ["quiet", "balanced", "performance"] {
            assert!(preset(name).unwrap().validate().is_ok(), "{name} should be valid");
        }
        assert!(preset("nope").is_none());
    }

    #[test]
    fn rejects_non_increasing_temps() {
        let mut c = ok_curve();
        c.points[1].temperature_c = c.points[0].temperature_c; // not strictly increasing
        assert_eq!(c.validate(), Err(CurveError::TemperaturesNotStrictlyIncreasing));
    }

    #[test]
    fn rejects_non_monotonic_duty() {
        let mut c = ok_curve();
        c.points[2].duty_percent = 10.0; // drops below previous
        assert_eq!(c.validate(), Err(CurveError::DutyNotMonotonic));
    }

    #[test]
    fn rejects_duty_out_of_bounds_and_bad_bounds() {
        let mut c = ok_curve();
        c.points[0].duty_percent = c.min_duty - 1.0;
        assert_eq!(c.validate(), Err(CurveError::DutyOutOfRange));

        let mut c = ok_curve();
        c.min_duty = 80.0;
        c.max_duty = 20.0;
        assert_eq!(c.validate(), Err(CurveError::InvalidDutyBounds));
    }

    #[test]
    fn rejects_fallback_outside_bounds() {
        let mut c = ok_curve();
        c.sensor_loss = SensorLossBehavior::SafeFallbackDuty(10.0); // below min 30
        assert_eq!(c.validate(), Err(CurveError::FallbackOutOfBounds));
    }

    #[test]
    fn interpolation_is_piecewise_linear_and_clamped() {
        let c = ok_curve();
        // Below first point -> first duty (>= min).
        assert!(approx(c.duty_for_temperature(0.0), 30.0));
        // Above last point -> last duty (100).
        assert!(approx(c.duty_for_temperature(200.0), 100.0));
        // Midway between (30,30) and (50,45) at 40 -> 37.5
        assert!(approx(c.duty_for_temperature(40.0), 37.5));
    }

    #[test]
    fn combine_respects_policy() {
        let mut c = ok_curve();
        c.source_policy = SourcePolicy::Max;
        assert!(approx(c.combine(&[40.0, 70.0, 55.0]).unwrap(), 70.0));
        c.source_policy = SourcePolicy::Average;
        assert!(approx(c.combine(&[40.0, 60.0]).unwrap(), 50.0));
        assert_eq!(c.combine(&[]), None);
    }
}
