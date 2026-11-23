//! Automatic curve evaluation with hysteresis and ramp limiting (PHASE-3-SPEC §5).

use crate::curve::{FanCurve, SensorLossBehavior};

/// A temperature input for the evaluator. `fresh` must be true only for a
/// currently-available, finite reading (stale/missing inputs are never used to
/// produce a duty — spec §4).
#[derive(Debug, Clone)]
pub struct TempSample {
    pub id: String,
    pub value: Option<f64>,
    pub fresh: bool,
}

impl TempSample {
    pub fn fresh(id: impl Into<String>, value: f64) -> Self {
        TempSample { id: id.into(), value: Some(value), fresh: true }
    }
    pub fn stale(id: impl Into<String>) -> Self {
        TempSample { id: id.into(), value: None, fresh: false }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AutoOutcome {
    /// Apply this duty (already ramp-limited and within curve bounds).
    Apply(f64),
    /// No valid sensor: drive the configured safe fallback duty.
    Fallback(f64),
    /// No valid sensor: relinquish control to firmware.
    Release,
}

/// Stateful automatic controller for one curve. Holds the last controlling
/// temperature (for hysteresis) and the current duty (for ramp limiting).
#[derive(Debug, Clone)]
pub struct AutoController {
    last_temp: Option<f64>,
    duty: f64,
}

impl AutoController {
    pub fn new(initial_duty: f64) -> Self {
        AutoController { last_temp: None, duty: initial_duty }
    }

    pub fn current_duty(&self) -> f64 {
        self.duty
    }

    /// Advance the controller by `dt_ms` given the latest samples.
    pub fn step(&mut self, curve: &FanCurve, samples: &[TempSample], dt_ms: u64) -> AutoOutcome {
        // Only fresh, finite readings from this curve's sources count.
        let usable: Vec<f64> = samples
            .iter()
            .filter(|s| curve.temp_source_ids.iter().any(|id| id == &s.id))
            .filter(|s| s.fresh)
            .filter_map(|s| s.value)
            .filter(|v| v.is_finite())
            .collect();

        let combined = match curve.combine(&usable) {
            Some(v) => v,
            None => {
                return match curve.sensor_loss {
                    SensorLossBehavior::SafeFallbackDuty(d) => {
                        self.duty = d.clamp(curve.min_duty, curve.max_duty);
                        AutoOutcome::Fallback(self.duty)
                    }
                    SensorLossBehavior::ReleaseToFirmware => AutoOutcome::Release,
                };
            }
        };

        // Hysteresis: keep the previous controlling temperature until the new one
        // moves beyond the band.
        let effective = match self.last_temp {
            Some(prev) if (combined - prev).abs() < curve.hysteresis_c => prev,
            _ => combined,
        };
        self.last_temp = Some(effective);

        let target = curve.duty_for_temperature(effective);

        // Ramp limiting: cap how fast duty may change this tick.
        let dt_s = dt_ms as f64 / 1000.0;
        let delta = target - self.duty;
        let limited = if delta >= 0.0 {
            delta.min(curve.ramp_up_pct_per_s * dt_s)
        } else {
            delta.max(-curve.ramp_down_pct_per_s * dt_s)
        };
        self.duty = (self.duty + limited).clamp(curve.min_duty, curve.max_duty);
        AutoOutcome::Apply(self.duty)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::curve::preset;

    fn approx(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-6
    }

    #[test]
    fn releases_or_falls_back_when_no_sensor() {
        let mut curve = preset("balanced").unwrap();

        curve.sensor_loss = SensorLossBehavior::SafeFallbackDuty(80.0);
        let mut c = AutoController::new(30.0);
        assert_eq!(
            c.step(&curve, &[TempSample::stale("cpu.temperature.tctl")], 1000),
            AutoOutcome::Fallback(80.0)
        );

        curve.sensor_loss = SensorLossBehavior::ReleaseToFirmware;
        let mut c = AutoController::new(30.0);
        assert_eq!(c.step(&curve, &[], 1000), AutoOutcome::Release);
    }

    #[test]
    fn ramp_up_is_limited() {
        let mut curve = preset("balanced").unwrap();
        curve.ramp_up_pct_per_s = 10.0; // max +10%/s
        let mut c = AutoController::new(30.0);
        // Hot sensor wants ~100%, but one 1s tick can only add 10%.
        let out = c.step(&curve, &[TempSample::fresh("cpu.temperature.tctl", 90.0)], 1000);
        assert_eq!(out, AutoOutcome::Apply(40.0));
    }

    #[test]
    fn ramp_down_is_slower_than_up() {
        let mut curve = preset("balanced").unwrap();
        curve.ramp_down_pct_per_s = 5.0;
        let mut c = AutoController::new(90.0);
        // Cold sensor wants low duty; 1s tick can only drop 5%.
        let out = c.step(&curve, &[TempSample::fresh("cpu.temperature.tctl", 25.0)], 1000);
        assert_eq!(out, AutoOutcome::Apply(85.0));
    }

    #[test]
    fn hysteresis_damps_small_temperature_moves() {
        let mut curve = preset("balanced").unwrap();
        curve.hysteresis_c = 5.0;
        curve.ramp_up_pct_per_s = 1000.0;
        curve.ramp_down_pct_per_s = 1000.0;
        let mut c = AutoController::new(30.0);

        let AutoOutcome::Apply(d1) =
            c.step(&curve, &[TempSample::fresh("cpu.temperature.tctl", 50.0)], 1000)
        else {
            panic!("expected apply");
        };
        // A +3°C move is within the 5°C band -> duty unchanged.
        let AutoOutcome::Apply(d2) =
            c.step(&curve, &[TempSample::fresh("cpu.temperature.tctl", 53.0)], 1000)
        else {
            panic!("expected apply");
        };
        assert!(approx(d1, d2), "duty should not change within hysteresis band");
    }
}
