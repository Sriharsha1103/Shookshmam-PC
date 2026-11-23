//! Built-in profile templates (PHASE-5-SPEC §2). These are editable starting
//! points, not fixed hardware assumptions; the fan-curve IDs line up with the
//! control crate's presets.

use crate::model::{ControlModePref, Profile};
use shookshmam_device_model::Capability;

/// Returns a named preset profile stamped at `now_ms`, or `None`.
pub fn preset(name: &str, now_ms: i64) -> Option<Profile> {
    let (curve_id, display, mode) = match name {
        "quiet" => ("quiet", "Quiet", ControlModePref::Automatic),
        "balanced" => ("balanced", "Balanced", ControlModePref::Automatic),
        "performance" => ("performance", "Performance", ControlModePref::Automatic),
        _ => return None,
    };
    let mut p = Profile::new(curve_id, display, now_ms);
    p.description = format!("{display} preset: automatic cooling curve and default dashboard.");
    p.preferred_control_mode = mode;
    p.required_capabilities = vec![Capability::FanDutyWrite];
    p.fan_curve_assignments.insert("cpu_fan".into(), curve_id.into());
    Some(p)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn presets_exist_and_validate() {
        for name in ["quiet", "balanced", "performance"] {
            let p = preset(name, 1000).unwrap();
            assert!(p.validate().is_ok());
            assert_eq!(p.fan_curve_assignments.get("cpu_fan").map(String::as_str), Some(name));
        }
        assert!(preset("nope", 1000).is_none());
    }
}
