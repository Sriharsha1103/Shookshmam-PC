//! Schema migration for imported profiles (PHASE-5-SPEC §5). Import/export must
//! carry a schema version and migrate older bundles forward. Only v1 exists
//! today; this is the hook where future upgrades chain.

use crate::model::{ProfileError, CURRENT_SCHEMA_VERSION};
use serde_json::Value;

/// Migrate a parsed profile JSON value up to the current schema version.
pub fn migrate(mut value: Value) -> Result<Value, ProfileError> {
    let version = value
        .get("schema_version")
        .and_then(|v| v.as_u64())
        .unwrap_or(0) as u32;

    if version > CURRENT_SCHEMA_VERSION {
        // A newer file than this build understands; refuse rather than guess.
        return Err(ProfileError::UnsupportedSchemaVersion(version));
    }

    // Future: while version < CURRENT_SCHEMA_VERSION { apply step; version += 1 }
    // v0 (version/field missing) is treated as the initial v1 shape.
    if version == 0 {
        if let Value::Object(map) = &mut value {
            map.insert("schema_version".into(), Value::from(CURRENT_SCHEMA_VERSION));
        }
    }

    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Profile;

    #[test]
    fn stamps_missing_version_as_current() {
        let mut p = Profile::new("id", "Name", 1);
        p.schema_version = CURRENT_SCHEMA_VERSION;
        let mut value: Value = serde_json::to_value(&p).unwrap();
        value.as_object_mut().unwrap().remove("schema_version");
        let migrated = migrate(value).unwrap();
        assert_eq!(migrated.get("schema_version").unwrap().as_u64().unwrap() as u32, CURRENT_SCHEMA_VERSION);
    }

    #[test]
    fn refuses_future_version() {
        let p = Profile::new("id", "Name", 1);
        let mut value = serde_json::to_value(&p).unwrap();
        value.as_object_mut().unwrap().insert("schema_version".into(), Value::from(999u32));
        assert_eq!(migrate(value), Err(ProfileError::UnsupportedSchemaVersion(999)));
    }
}
