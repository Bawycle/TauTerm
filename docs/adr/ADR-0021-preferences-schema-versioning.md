<!-- SPDX-License-Identifier: MPL-2.0 -->

# ADR-0021 — Preferences schema versioning

**Date:** 2026-04-10
**Status:** Accepted

## Context

The `Preferences` struct (`src-tauri/src/preferences/schema.rs`) is the
top-level type serialized to `~/.config/tauterm/preferences.toml` (ADR-0016).
The current struct has no `schema_version` field.  Forward compatibility is
handled entirely by `#[serde(default)]`: fields present in the file but absent
from the struct are silently ignored; fields absent from the file receive their
`Default` value.

This works for additive changes (new fields with defaults) but is insufficient
for:

- **Field renames**: the old key is silently ignored; the field gets its default
  value.  The user's setting is lost without warning.
- **Field removals**: the old key is silently discarded.  Not harmful by itself,
  but the user's value cannot be recovered.
- **Type changes**: deserializing a value that was formerly a string as an enum
  variant (or vice versa) produces a silent fallback to the default if
  `#[serde(default)]` is present, hiding the incompatibility.
- **Restructuring** (moving a scalar into a sub-table, or splitting one field
  into two): impossible to handle without explicit migration logic.

ADR-0012 (§Consequences, Negative) and ADR-0016 (§Consequences, Negative)
both explicitly acknowledged this as deferred debt:

> "Schema changes that rename or remove fields are not backward-compatible. A
> field that is renamed requires a migration step. For v1, this risk is accepted;
> no migration infrastructure is built."

The codebase has already made one format migration (JSON → TOML, ADR-0016) using
a file-level check (`preferences.json` → `preferences.toml`).  That migration
did not need a version field because the two files have distinct names.  Future
in-format migrations cannot rely on a separate file name and require an explicit
version discriminant inside the file.

## Decision

Add a `schema_version: u32` field to `Preferences` and implement a sequential
migration engine in a new module `src-tauri/src/preferences/store/migration.rs`.

### Schema version field

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Preferences {
    #[serde(default)]
    pub schema_version: u32,
    // … existing fields …
}
```

The on-disk TOML key is `schema_version` (snake_case via the camelCase ↔
snake_case bridge in `store.rs`).  The initial value written by TauTerm is `1`.

Files that predate this change have no `schema_version` key; `#[serde(default)]`
yields `0`.  Version `0` is therefore defined as "pre-versioning" and is the
input to the first migration step (v0 → v1).

### Migration engine

```rust
// src-tauri/src/preferences/store/migration.rs
pub fn migrate(mut raw: serde_json::Value) -> serde_json::Value {
    let version = raw["schemaVersion"].as_u64().unwrap_or(0) as u32;

    if version < 1 {
        // v0 → v1: no structural change; stamp the version.
        raw["schemaVersion"] = serde_json::Value::from(1u32);
    }

    // Future steps follow the same pattern:
    // if version < 2 { /* transform raw */; raw["schemaVersion"] = 2.into(); }
    // if version < 3 { /* transform raw */; raw["schemaVersion"] = 3.into(); }

    raw
}
```

The engine operates on a `serde_json::Value` (the parsed TOML converted to JSON
via the existing camelCase bridge) before the final `serde_json::from_value::<Preferences>()`.

Each step is idempotent: running the engine on an already-migrated file is safe
(the `if version < N` guard skips steps the file has already passed).

The engine is called in `load_from_disk()` after the TOML → JSON key-rename
bridge and before the final deserialization into `Preferences`.

After migration, the updated `schema_version` is persisted on the next
`save_to_disk()` call (the normal save path already rewrites the full file).
There is no separate "save after load" step: the migrated struct is held in
memory; it is written when the user next changes a preference or at application
shutdown.

### Version registry

| Version | Change | Migration step |
|---------|--------|----------------|
| 0       | Pre-versioning (no field) | — (baseline) |
| 1       | Add `schema_version` field | v0 → v1: stamp version only |

Each future schema change that is not purely additive increments the version and
adds a migration step.

## Alternatives considered

**Continue with implicit serde defaults (status quo)**

Accept that field renames and removals silently reset the user's setting to the
default.  This is the current behavior.

The cost is that the first time TauTerm renames a field (e.g., `cursorBlinkMs`
→ `cursorBlinkInterval`), every user's setting silently reverts to the default
value on next startup.  There is no way to warn the user, no recovery path, and
no audit trail.  For a settings file that users legitimately hand-edit and back
up in dotfile managers, silent data loss on application update is not acceptable.
Rejected.

**Explicit opt-in migration per field (scattered migration logic)**

Add a custom `Deserialize` implementation for `Preferences` (or for individual
sub-structs) that handles legacy field names via `#[serde(alias)]` and applies
transformations inline.

This approach is fine for simple renames (a new field name with `#[serde(alias
= "old_name")]`), but does not extend to structural changes (splitting a field,
moving a field between sub-structs, or changing a type).  More importantly, it
scatters migration logic across multiple `Deserialize` implementations, making
it hard to understand the full migration history or test the complete path from
an old file to the current schema.  Rejected for structural migrations; `#[serde(alias)]`
remains a valid tool for simple renames within a migration step.

**Config file reset on parse error**

If `Preferences::default()` is already returned on any parse error (current
behavior in `io.rs`), one could argue that schema incompatibilities are "handled"
by the reset.  This is the worst possible outcome: the user loses all their
settings (custom theme, saved SSH connections, font preferences) silently on
every application update that modifies the schema.  Explicitly rejected as
catastrophic data loss.

**SQLite with a schema migration table**

Maintain preferences in an SQLite database with a `schema_version` table and
use a migration runner (e.g., `refinery` or `sqlx::migrate`).

This provides the most robust versioning and migration infrastructure, but it is
significantly over-engineered for a < 50 KB file accessed once at startup and
written rarely.  ADR-0012 already rejected SQLite on these grounds.  Rejected.

## Consequences

**Positive:**
- Each future field rename, removal, or structural change can be handled
  precisely, with no silent data loss.
- The migration engine is a pure function (`serde_json::Value` → `serde_json::Value`),
  making it straightforward to unit-test: provide a JSON object representing an
  old-format preferences file and assert the expected output.
- The version field gives operators and support staff a diagnostic data point
  when debugging preference-related issues.
- The v0 → v1 migration is a no-op (stamps the version only), so no existing
  preferences file is modified structurally by this change.

**Negative / risks:**
- Every future schema change that is not purely additive now requires a
  migration step.  This adds a small but real maintenance obligation: a developer
  adding a renamed field must also add a migration step and increment the version.
- The migration engine operates on `serde_json::Value` (not `toml::Value`),
  which means field names in migration steps must use camelCase (the IPC/internal
  naming convention), not snake_case (the on-disk TOML convention).  This is
  consistent with the existing key-rename bridge but must be documented clearly
  to avoid developer confusion.
- If a migration step contains a bug that corrupts the value, the user's
  preferences may be silently reset to defaults (the existing error recovery path
  in `io.rs` returns `Preferences::default()` on deserialization failure).  Each
  migration step must be unit-tested before shipping.
