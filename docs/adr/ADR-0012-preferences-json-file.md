<!-- SPDX-License-Identifier: MPL-2.0 -->

# ADR-0012 — Preferences persistence: JSON file in XDG_CONFIG_HOME

**Date:** 2026-04-04
**Status:** Accepted

## Context

TauTerm must persist user preferences (appearance, terminal behavior, keyboard shortcuts, saved SSH connections, user-defined themes) across sessions. The preferences data includes:
- Scalar values: font size, scrollback size, cursor style, theme name
- Small collections: keyboard shortcut bindings, word delimiters
- Structured objects: saved SSH connections (`SshConnectionConfig`), user themes (`UserTheme`)

The data is small (typically < 50 KB), structured, and human-readable editing is a legitimate use case (advanced users may want to edit their config in a text editor). The access pattern is read-heavy: preferences are loaded once at startup and updated infrequently (on explicit user action in the Preferences panel).

## Decision

Store preferences as a **JSON file at `~/.config/tauterm/preferences.json`** (respecting `XDG_CONFIG_HOME` if set; defaulting to `~/.config` if not).

The file is managed by `PreferencesStore` (`preferences/store.rs`). It is loaded at application startup via `PreferencesStore::load_or_default()` (see ARCHITECTURE.md §7.6 for the load strategy and corruption recovery policy). It is written atomically on every preference update: write to a temporary file in the same directory, then `rename()` — this avoids partial writes.

Schema forward-compatibility: new fields are added with `#[serde(default)]` on the Rust struct. Missing fields in an older file are populated with defaults on load. There is no versioning field and no migration engine in v1; structural changes that are not backward-compatible with `serde(default)` will require an explicit migration step in a future version.

Saved SSH connections (`Vec<SshConnectionConfig>`) live under the `connections` sub-key of the preferences JSON. They are the single authoritative source of connection configuration data; the `SshManager` accesses them via `State<PreferencesStore>` (see ARCHITECTURE.md §3.2, `ssh/manager.rs`).

## Alternatives considered

**SQLite database (`rusqlite`)**
SQLite provides atomic writes, schema versioning, and efficient querying of large datasets. For TauTerm's preferences (< 50 KB, < 100 structured objects), SQLite is significantly over-engineered: it adds a native dependency, a database file format that is not human-readable, and a query layer for data that is entirely in-memory during the application's lifetime. Rejected: cost does not match scale.

**TOML file**
TOML is a human-friendly configuration format with strong type support and good `serde` integration (`toml` crate). The main drawback for TauTerm: TOML does not support arbitrary-length arrays of complex objects as cleanly as JSON (multiline TOML table arrays are verbose). The `SshConnectionConfig` and `UserTheme` objects contain nested maps and optional fields that serialize more naturally to JSON. Additionally, JSON is used throughout the rest of the codebase (IPC payloads, event types); keeping preferences in JSON maintains a single serialization format. Rejected: marginal ergonomic benefit does not justify a second serialization library.

**Platform-specific storage (dconf on Linux, NSUserDefaults on macOS, registry on Windows)**
Platform-specific storage would require PAL abstraction for preferences, separate from the data model. It would also break user workflows: many developers back up or sync `~/.config/` across machines. A platform-neutral JSON file in XDG_CONFIG_HOME is simpler and more portable. Rejected.

**Separate files per preference domain (preferences.json, connections.json, themes.json)**
Splitting into multiple files reduces the write blast radius (a theme update only rewrites themes.json) but adds complexity: a load error in one file must be handled independently, atomic updates across files are impossible without a lock file, and the `PreferencesStore` API becomes more complex. For v1 scale, the simplicity of one file outweighs the blast-radius concern. Rejected.

## Consequences

**Positive:**
- The preferences file is human-readable and human-editable. Advanced users can back it up, sync it with dotfile managers, or inspect it for debugging.
- A single file with atomic writes (write-then-rename) guarantees consistency: either the full new preferences are written, or the old file is intact.
- `serde(default)` provides forward compatibility: older files load correctly after application updates that add new preference fields.
- No additional runtime dependency beyond `serde_json` (already used for IPC types).

**Negative / risks:**
- Schema changes that rename or remove fields are not backward-compatible. A field that is removed from the Rust struct will be silently ignored on load (serde drops unknown fields by default). A field that is renamed requires a migration step. For v1, this risk is accepted; no migration infrastructure is built.
- The entire preferences file is rewritten on every `update_preferences` call. For the expected data size (< 50 KB), this is fast and safe. If `themes` or `connections` grow very large (hundreds of items), write cost could become perceptible; this is not expected in v1.
- Corruption recovery: if the JSON file is corrupted (incomplete write, manual edit error), the application loads defaults (see ARCHITECTURE.md §7.6). The corrupted file is not automatically deleted or repaired; the user must intervene manually. This is a conscious design choice: automatic deletion of user data on any parse error is not acceptable.

## Notes

The `XDG_CONFIG_HOME` environment variable must be respected per the XDG Base Directory Specification. If `XDG_CONFIG_HOME` is set, the preferences file is at `$XDG_CONFIG_HOME/tauterm/preferences.json`; otherwise at `$HOME/.config/tauterm/preferences.json`. The directory is created if it does not exist. Known-hosts are stored separately at `~/.config/tauterm/known_hosts` (OpenSSH format; not JSON).
