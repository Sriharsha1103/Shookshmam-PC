//! Phase 5 integration tests: profile portability (export/import round trip)
//! and the transactional apply never reporting a partial result as success.

use shookshmam_device_model::Capability;
use shookshmam_profiles::{apply, plan, preset, DeviceEnvironment, Platform, PlannedChange, Profile};
use std::collections::HashSet;

struct Env {
    platform: Platform,
    caps: HashSet<Capability>,
    channels: HashSet<String>,
}

impl DeviceEnvironment for Env {
    fn platform(&self) -> Platform {
        self.platform
    }
    fn has_capability(&self, cap: Capability) -> bool {
        self.caps.contains(&cap)
    }
    fn has_display(&self, _id: &str) -> bool {
        false
    }
    fn has_channel(&self, id: &str) -> bool {
        self.channels.contains(id)
    }
    fn has_zone(&self, _id: &str) -> bool {
        false
    }
    fn apply_change(&mut self, _change: &PlannedChange) -> Result<(), String> {
        Ok(())
    }
    fn rollback(&mut self, _applied: &[PlannedChange]) {}
}

#[test]
fn preset_exports_and_imports_identically() {
    let p = preset("balanced", 1000).unwrap();
    let json = p.to_json();
    let back = Profile::from_json(&json).unwrap();
    assert_eq!(p, back);
    // Portable format carries the schema version for migration.
    assert!(json.contains("schema_version"));
}

#[test]
fn apply_on_capable_but_unvalidated_env_is_not_full_success() {
    // Channel exists but fan control capability is absent (unvalidated) -> the
    // fan curve change is unsupported, so the apply is not fully successful.
    let mut env = Env {
        platform: Platform::BazziteLinux,
        caps: HashSet::new(),
        channels: ["cpu_fan".to_string()].into_iter().collect(),
    };
    let profile = preset("balanced", 1000).unwrap();
    let preview = plan(&profile, &env);
    assert!(preview.has_unsupported());

    let report = apply(&profile, &mut env, true);
    assert!(!report.fully_successful);
}

#[test]
fn apply_succeeds_when_capability_present() {
    let mut env = Env {
        platform: Platform::BazziteLinux,
        caps: [Capability::FanDutyWrite].into_iter().collect(),
        channels: ["cpu_fan".to_string()].into_iter().collect(),
    };
    let profile = preset("balanced", 1000).unwrap();
    let report = apply(&profile, &mut env, true);
    assert!(report.fully_successful);
    assert_eq!(report.applied.len(), 1);
}
