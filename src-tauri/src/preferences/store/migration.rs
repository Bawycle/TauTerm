// SPDX-License-Identifier: MPL-2.0

//! Preferences schema migration.
//!
//! Operates on a `serde_json::Value` representation of the preferences (after
//! TOML → JSON conversion) so that structural changes can be applied without
//! having to deserialize into a typed struct first.
//!
//! ## Adding a new migration
//!
//! 1. Increment `CURRENT_VERSION`.
//! 2. Add a `step_vN_to_vM` function that transforms the raw JSON value.
//! 3. Call it from `migrate()` with the appropriate version guard.
//! 4. Add a unit test for the new step.
//!
//! The key names in the JSON object use camelCase because the TOML → JSON
//! conversion goes through `serde_json::to_value(toml_val)` after the
//! `snake_to_camel` key rename step in `schema_convert.rs`. This means the
//! `schemaVersion` field (from `Preferences.schema_version` with
//! `rename_all = "camelCase"`) is the correct key to read and write here.

use serde_json::Value;

/// The current schema version. Must match `preferences::schema::PREFS_SCHEMA_VERSION`.
pub const CURRENT_VERSION: u32 = 1;

/// Migrate a raw preferences JSON value from its stored version to [`CURRENT_VERSION`].
///
/// Each migration step is guarded by a version check and is idempotent: running
/// it on an already-migrated value is a no-op. Steps are applied in order so
/// that multi-version jumps are handled correctly.
pub fn migrate(raw: Value) -> Value {
    // Only JSON objects can be valid preferences — non-objects (arrays, strings,
    // numbers, null) are corrupt inputs. Return them unchanged so the caller's
    // `serde_json::from_value::<Preferences>()` step will fail gracefully and
    // fall back to defaults. Never panic on unexpected input.
    if !raw.is_object() {
        return raw;
    }

    let version = raw
        .get("schemaVersion")
        .and_then(|v| v.as_u64())
        .unwrap_or(0) as u32;

    // v0 → v1: stamp the schema version field (no structural changes in v1).
    step_v0_to_v1(raw, version)
}

fn step_v0_to_v1(mut raw: Value, version: u32) -> Value {
    if version < 1 {
        // `raw` is guaranteed to be a Value::Object by the guard in `migrate`.
        if let Some(obj) = raw.as_object_mut() {
            obj.insert(
                "schemaVersion".to_owned(),
                Value::Number(CURRENT_VERSION.into()),
            );
        }
    }
    raw
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn migrate_v0_stamps_version() {
        let raw = serde_json::json!({ "appearance": {} });
        let migrated = migrate(raw);
        assert_eq!(
            migrated["schemaVersion"], CURRENT_VERSION,
            "v0 input must have schemaVersion stamped to CURRENT_VERSION"
        );
    }

    #[test]
    fn migrate_v1_is_idempotent() {
        let raw = serde_json::json!({ "schemaVersion": 1 });
        let migrated = migrate(raw);
        assert_eq!(
            migrated["schemaVersion"], 1,
            "already-v1 input must not be changed"
        );
    }

    #[test]
    fn migrate_preserves_existing_fields() {
        let raw = serde_json::json!({
            "appearance": { "fontSize": 14 },
            "terminal": { "scrollbackLines": 5000 }
        });
        let migrated = migrate(raw);
        assert_eq!(migrated["appearance"]["fontSize"], 14);
        assert_eq!(migrated["terminal"]["scrollbackLines"], 5000);
        assert_eq!(migrated["schemaVersion"], CURRENT_VERSION);
    }

    #[test]
    fn migrate_empty_object_stamps_version() {
        let raw = serde_json::json!({});
        let migrated = migrate(raw);
        assert_eq!(migrated["schemaVersion"], CURRENT_VERSION);
    }
}
