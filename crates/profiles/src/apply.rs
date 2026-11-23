//! Transactional profile application (PHASE-5-SPEC §4): validate → discover →
//! compare → preview → confirm → apply in deterministic order with per-change
//! acknowledgement → stop-and-rollback on failure → report. A partial
//! application is never reported as fully successful.

use crate::model::{Platform, Profile};
use serde::Serialize;
use shookshmam_device_model::Capability;

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ChangeKind {
    Layout,
    FanCurve,
    Lighting,
}

/// One intended change, with whether the current hardware supports it.
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct PlannedChange {
    pub kind: ChangeKind,
    pub target: String,
    pub value: String,
    pub supported: bool,
    pub reason: Option<String>,
}

/// The previewable plan: every change plus what is missing/unsupported.
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct ApplyPlan {
    pub platform_compatible: bool,
    pub missing_capabilities: Vec<Capability>,
    pub changes: Vec<PlannedChange>,
}

impl ApplyPlan {
    pub fn supported(&self) -> impl Iterator<Item = &PlannedChange> {
        self.changes.iter().filter(|c| c.supported)
    }

    pub fn has_unsupported(&self) -> bool {
        self.changes.iter().any(|c| !c.supported)
    }
}

/// Outcome of an apply transaction.
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct ApplyReport {
    pub applied: Vec<String>,
    pub rejected: Vec<(String, String)>,
    pub rolled_back: bool,
    pub fully_successful: bool,
    pub note: String,
}

/// The hardware environment a profile is applied against. Implemented by the app
/// over the live registry/controllers; a mock implements it in tests.
pub trait DeviceEnvironment {
    fn platform(&self) -> Platform;
    fn has_capability(&self, cap: Capability) -> bool;
    fn has_display(&self, id: &str) -> bool;
    fn has_channel(&self, id: &str) -> bool;
    fn has_zone(&self, id: &str) -> bool;
    /// Apply one change to hardware. `Err(reason)` triggers rollback.
    fn apply_change(&mut self, change: &PlannedChange) -> Result<(), String>;
    /// Undo already-applied changes (release to firmware / restore known state).
    fn rollback(&mut self, applied: &[PlannedChange]);
}

/// Build the previewable plan by comparing the profile with the environment.
pub fn plan(profile: &Profile, env: &dyn DeviceEnvironment) -> ApplyPlan {
    let platform_compatible = profile.supports_platform(env.platform());

    let missing_capabilities: Vec<Capability> = profile
        .required_capabilities
        .iter()
        .copied()
        .filter(|c| !env.has_capability(*c))
        .collect();

    let mut changes = Vec::new();

    // Deterministic order: layouts, then fan curves, then lighting (BTreeMap
    // iteration is already sorted by key).
    for (display, layout) in &profile.layout_assignments {
        let supported = env.has_display(display);
        changes.push(PlannedChange {
            kind: ChangeKind::Layout,
            target: display.clone(),
            value: layout.clone(),
            supported,
            reason: (!supported).then(|| format!("display `{display}` not present")),
        });
    }
    for (channel, curve) in &profile.fan_curve_assignments {
        let supported = env.has_channel(channel) && env.has_capability(Capability::FanDutyWrite);
        changes.push(PlannedChange {
            kind: ChangeKind::FanCurve,
            target: channel.clone(),
            value: curve.clone(),
            supported,
            reason: (!supported)
                .then(|| format!("channel `{channel}` missing or fan control unvalidated")),
        });
    }
    for (zone, effect) in &profile.lighting_assignments {
        let supported = env.has_zone(zone) && env.has_capability(Capability::RgbEffectWrite);
        changes.push(PlannedChange {
            kind: ChangeKind::Lighting,
            target: zone.clone(),
            value: effect.clone(),
            supported,
            reason: (!supported)
                .then(|| format!("zone `{zone}` missing or lighting unvalidated")),
        });
    }

    ApplyPlan { platform_compatible, missing_capabilities, changes }
}

/// Apply a profile. Requires `confirmed` for any hardware write (spec §4.5).
pub fn apply(profile: &Profile, env: &mut dyn DeviceEnvironment, confirmed: bool) -> ApplyReport {
    let plan = plan(profile, env);

    if !plan.platform_compatible {
        return ApplyReport {
            applied: Vec::new(),
            rejected: Vec::new(),
            rolled_back: false,
            fully_successful: false,
            note: "profile is not compatible with this platform".into(),
        };
    }
    if !confirmed {
        return ApplyReport {
            applied: Vec::new(),
            rejected: Vec::new(),
            rolled_back: false,
            fully_successful: false,
            note: "confirmation required before applying hardware changes".into(),
        };
    }

    let supported: Vec<PlannedChange> = plan.supported().cloned().collect();
    let mut applied_changes: Vec<PlannedChange> = Vec::new();
    let mut applied = Vec::new();
    let mut rejected = Vec::new();
    let mut rolled_back = false;

    for change in &supported {
        let label = format!("{:?} {} = {}", change.kind, change.target, change.value);
        match env.apply_change(change) {
            Ok(()) => {
                applied.push(label);
                applied_changes.push(change.clone());
            }
            Err(reason) => {
                rejected.push((label, reason));
                // Stop and roll back everything already applied.
                env.rollback(&applied_changes);
                rolled_back = true;
                break;
            }
        }
    }

    let fully_successful =
        rejected.is_empty() && !plan.has_unsupported() && plan.missing_capabilities.is_empty();

    let note = if fully_successful {
        "profile applied".into()
    } else if rolled_back {
        "a change failed; applied changes were rolled back".into()
    } else {
        "profile partially applied: some components are unsupported on this hardware".into()
    };

    ApplyReport { applied, rejected, rolled_back, fully_successful, note }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    struct MockEnv {
        platform: Platform,
        caps: HashSet<Capability>,
        displays: HashSet<String>,
        channels: HashSet<String>,
        zones: HashSet<String>,
        fail_on: Option<String>,
        rolled_back: usize,
    }

    impl MockEnv {
        fn new(platform: Platform) -> Self {
            MockEnv {
                platform,
                caps: HashSet::new(),
                displays: HashSet::new(),
                channels: HashSet::new(),
                zones: HashSet::new(),
                fail_on: None,
                rolled_back: 0,
            }
        }
    }

    impl DeviceEnvironment for MockEnv {
        fn platform(&self) -> Platform {
            self.platform
        }
        fn has_capability(&self, cap: Capability) -> bool {
            self.caps.contains(&cap)
        }
        fn has_display(&self, id: &str) -> bool {
            self.displays.contains(id)
        }
        fn has_channel(&self, id: &str) -> bool {
            self.channels.contains(id)
        }
        fn has_zone(&self, id: &str) -> bool {
            self.zones.contains(id)
        }
        fn apply_change(&mut self, change: &PlannedChange) -> Result<(), String> {
            if self.fail_on.as_deref() == Some(change.target.as_str()) {
                Err("simulated write failure".into())
            } else {
                Ok(())
            }
        }
        fn rollback(&mut self, applied: &[PlannedChange]) {
            self.rolled_back = applied.len();
        }
    }

    fn profile() -> Profile {
        let mut p = Profile::new("balanced", "Balanced", 1000);
        p.layout_assignments.insert("thermalright".into(), "main".into());
        p.fan_curve_assignments.insert("cpu_fan".into(), "balanced".into());
        p.lighting_assignments.insert("mobo_argb".into(), "static-red".into());
        p
    }

    #[test]
    fn preview_marks_unsupported_components() {
        let mut env = MockEnv::new(Platform::Windows);
        env.displays.insert("thermalright".into());
        // No fan/lighting capability or targets.
        let plan = plan(&profile(), &env);
        assert!(plan.has_unsupported());
        assert_eq!(plan.supported().count(), 1); // only the layout
    }

    #[test]
    fn requires_confirmation() {
        let mut env = MockEnv::new(Platform::Windows);
        env.displays.insert("thermalright".into());
        let report = apply(&profile(), &mut env, false);
        assert!(report.applied.is_empty());
        assert!(!report.fully_successful);
        assert!(report.note.contains("confirmation"));
    }

    #[test]
    fn partial_apply_is_not_full_success() {
        let mut env = MockEnv::new(Platform::Windows);
        env.displays.insert("thermalright".into()); // only layout supported
        let report = apply(&profile(), &mut env, true);
        assert_eq!(report.applied.len(), 1);
        assert!(!report.fully_successful, "partial apply must not be full success");
        assert!(!report.rolled_back);
    }

    #[test]
    fn failure_triggers_rollback() {
        let mut env = MockEnv::new(Platform::Windows);
        env.displays.insert("thermalright".into());
        env.channels.insert("cpu_fan".into());
        env.zones.insert("mobo_argb".into());
        env.caps.insert(Capability::FanDutyWrite);
        env.caps.insert(Capability::RgbEffectWrite);
        // All three supported, but the lighting write fails.
        env.fail_on = Some("mobo_argb".into());
        let report = apply(&profile(), &mut env, true);
        assert!(report.rolled_back);
        assert!(!report.fully_successful);
        assert_eq!(report.rejected.len(), 1);
        // Layout + fan applied before the failure -> both rolled back.
        assert_eq!(env.rolled_back, 2);
    }

    #[test]
    fn full_success_when_everything_supported() {
        let mut env = MockEnv::new(Platform::Windows);
        env.displays.insert("thermalright".into());
        env.channels.insert("cpu_fan".into());
        env.zones.insert("mobo_argb".into());
        env.caps.insert(Capability::FanDutyWrite);
        env.caps.insert(Capability::RgbEffectWrite);
        let report = apply(&profile(), &mut env, true);
        assert!(report.fully_successful);
        assert_eq!(report.applied.len(), 3);
    }

    #[test]
    fn incompatible_platform_applies_nothing() {
        let mut env = MockEnv::new(Platform::BazziteLinux);
        let mut p = profile();
        p.platforms = vec![Platform::Windows];
        let report = apply(&p, &mut env, true);
        assert!(report.applied.is_empty());
        assert!(!report.fully_successful);
    }
}
