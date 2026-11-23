//! Declarative profile schema (PHASE-5-SPEC §3). Profiles MUST NOT contain
//! executable scripts or shell commands — only data: assignments, selectors and
//! preferences. Portable settings are kept separate from OS-specific device
//! identifiers by matching on capabilities + stable IDs at apply time.

use serde::{Deserialize, Serialize};
use shookshmam_device_model::Capability;
use std::collections::BTreeMap;

/// Current profile schema version.
pub const CURRENT_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Platform {
    Windows,
    BazziteLinux,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ControlModePref {
    Automatic,
    Manual,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Profile {
    pub schema_version: u32,
    pub id: String,
    pub name: String,
    pub description: String,
    pub platforms: Vec<Platform>,
    /// Capabilities this profile needs to apply fully.
    pub required_capabilities: Vec<Capability>,
    /// display id -> layout name.
    pub layout_assignments: BTreeMap<String, String>,
    /// fan channel id -> curve id.
    pub fan_curve_assignments: BTreeMap<String, String>,
    /// lighting zone id -> effect/profile name.
    pub lighting_assignments: BTreeMap<String, String>,
    pub preferred_control_mode: ControlModePref,
    pub launch_at_login: bool,
    pub background_operation: bool,
    pub created_unix_ms: i64,
    pub updated_unix_ms: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProfileError {
    UnsupportedSchemaVersion(u32),
    EmptyId,
    EmptyName,
    NoPlatforms,
    Deserialize(String),
}

impl std::fmt::Display for ProfileError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProfileError::UnsupportedSchemaVersion(v) => {
                write!(f, "unsupported profile schema version {v}")
            }
            ProfileError::EmptyId => write!(f, "profile id must not be empty"),
            ProfileError::EmptyName => write!(f, "profile name must not be empty"),
            ProfileError::NoPlatforms => write!(f, "profile must list at least one platform"),
            ProfileError::Deserialize(e) => write!(f, "could not parse profile: {e}"),
        }
    }
}

impl std::error::Error for ProfileError {}

impl Profile {
    /// A new, empty profile stamped at `now_ms`.
    pub fn new(id: impl Into<String>, name: impl Into<String>, now_ms: i64) -> Self {
        Profile {
            schema_version: CURRENT_SCHEMA_VERSION,
            id: id.into(),
            name: name.into(),
            description: String::new(),
            platforms: vec![Platform::Windows, Platform::BazziteLinux],
            required_capabilities: Vec::new(),
            layout_assignments: BTreeMap::new(),
            fan_curve_assignments: BTreeMap::new(),
            lighting_assignments: BTreeMap::new(),
            preferred_control_mode: ControlModePref::Automatic,
            launch_at_login: false,
            background_operation: false,
            created_unix_ms: now_ms,
            updated_unix_ms: now_ms,
        }
    }

    pub fn validate(&self) -> Result<(), ProfileError> {
        if self.schema_version != CURRENT_SCHEMA_VERSION {
            return Err(ProfileError::UnsupportedSchemaVersion(self.schema_version));
        }
        if self.id.trim().is_empty() {
            return Err(ProfileError::EmptyId);
        }
        if self.name.trim().is_empty() {
            return Err(ProfileError::EmptyName);
        }
        if self.platforms.is_empty() {
            return Err(ProfileError::NoPlatforms);
        }
        Ok(())
    }

    pub fn supports_platform(&self, platform: Platform) -> bool {
        self.platforms.contains(&platform)
    }

    /// Serialize to pretty JSON for export.
    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).expect("profile serializes")
    }

    /// Parse from JSON, running schema migration first, then validating.
    pub fn from_json(s: &str) -> Result<Profile, ProfileError> {
        let value: serde_json::Value =
            serde_json::from_str(s).map_err(|e| ProfileError::Deserialize(e.to_string()))?;
        let migrated = crate::migrate::migrate(value)?;
        let profile: Profile = serde_json::from_value(migrated)
            .map_err(|e| ProfileError::Deserialize(e.to_string()))?;
        profile.validate()?;
        Ok(profile)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_rejects_empty_and_bad_version() {
        let mut p = Profile::new("id", "Name", 1);
        assert!(p.validate().is_ok());
        p.id = "  ".into();
        assert_eq!(p.validate(), Err(ProfileError::EmptyId));
        let mut p = Profile::new("id", "Name", 1);
        p.schema_version = 99;
        assert_eq!(p.validate(), Err(ProfileError::UnsupportedSchemaVersion(99)));
    }

    #[test]
    fn json_round_trip() {
        let mut p = Profile::new("balanced", "Balanced", 1000);
        p.fan_curve_assignments.insert("cpu_fan".into(), "balanced".into());
        let json = p.to_json();
        let back = Profile::from_json(&json).unwrap();
        assert_eq!(p, back);
    }

    #[test]
    fn rejects_profile_with_no_platforms() {
        let mut p = Profile::new("id", "Name", 1);
        p.platforms.clear();
        assert_eq!(p.validate(), Err(ProfileError::NoPlatforms));
    }
}
