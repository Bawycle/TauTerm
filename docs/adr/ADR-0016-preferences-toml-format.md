<!-- SPDX-License-Identifier: MPL-2.0 -->

# ADR-0016 — Preferences persistence: TOML format with snake_case keys

**Date:** 2026-04-06
**Status:** Accepted
**Supersedes:** [ADR-0012](ADR-0012-preferences-json-file.md)

## Context

ADR-0012 chose JSON for preferences persistence, citing (a) natural fit for
complex nested objects (`SshConnectionConfig`, `UserTheme`), and (b) a single
serialization format across the codebase (IPC + disk).

In practice, two observations challenge that rationale:

1. **Human readability**: JSON is valid for data exchange but deliberately
   hostile to hand-editing (no comments, mandatory quoting for all keys,
   trailing-comma prohibition).  The preferences file is a user-facing artifact
   that advanced users legitimately edit in a text editor or manage with dotfile
   managers.  TOML is designed precisely for this use case: it is as readable as
   INI, as expressive as JSON, and supports inline comments.

2. **Coupling of formats**: the argument that "JSON is already used everywhere"
   conflates two distinct concerns.  IPC payloads are ephemeral, machine-to-machine
   data that benefits from camelCase alignment with TypeScript conventions.  The
   preferences file is a persistent, human-facing artifact where the POSIX/TOML
   convention (snake_case keys) is appropriate.  These two contexts have legitimately
   different conventions; sharing a format does not add value and forces one context
   to adopt the other's conventions.

The `toml` crate (version 0.8) was already a dependency.  The Rust preference
structs use `#[serde(rename_all = "camelCase")]` for IPC compatibility; a
key-renaming bridge in `preferences/store.rs` (camelCase ↔ snake_case) decouples
disk format from struct naming without duplicating types.

## Decision

Store preferences as a **TOML file at `~/.config/tauterm/preferences.toml`**
(respecting `XDG_CONFIG_HOME` if set) with **snake_case keys** following the
TOML/POSIX configuration file convention.

### Key naming

- On-disk keys: `snake_case` (e.g. `font_size`, `scrollback_lines`, `cursor_blink_ms`).
- IPC / Rust struct keys: `camelCase` (unchanged — driven by `#[serde(rename_all = "camelCase")]`).
- The bridge: `preferences/store.rs` converts keys via `camel_to_snake` on save
  and `snake_to_camel` on load, operating on the intermediate `toml::Value` tree.
  No struct duplication is required.

### Migration

If `preferences.toml` is absent but `preferences.json` exists (prior installation),
the JSON file is parsed using its existing camelCase keys (directly compatible with
the serde attributes) and used as the initial preferences.  The TOML file is written
on the first subsequent `save_to_disk` call.  The JSON file is not deleted automatically.

### Load strategy

Same robustness guarantees as ADR-0012 (see ARCHITECTURE.md §7.6):
- Missing file → `Preferences::default()`.
- Parse error → log `WARN`, return `Preferences::default()`.
- No automatic deletion of corrupted files.

## Alternatives considered

**Keep JSON (status quo from ADR-0012)**
The original rationale was sound at the time, but the ergonomic cost of
hand-editing camelCase JSON (quoting keys, no comments, trailing commas
forbidden) outweighs the benefit of format uniformity.  The uniformity argument
is also weakened by the recognition that IPC and disk persistence are
fundamentally different contexts.  Rejected.

**TOML with camelCase keys**
TOML technically allows any key name, including camelCase.  However, this is
unconventional and confusing: camelCase in TOML files has no precedent in the
ecosystem and would surprise users who hand-edit the file.  The explicit
IPC/disk decoupling implemented via the camelCase ↔ snake_case bridge costs
negligible runtime overhead (startup-only, < 50 KB file) and produces a
file that follows universal TOML conventions.  Rejected.

**Separate `PreferencesOnDisk` struct with `snake_case` serde attributes**
This approach avoids the key-rename bridge but requires maintaining two
parallel struct trees in sync, introducing structural duplication and a
`From`/`Into` conversion that must be updated on every schema change.
The bridge function operating on `toml::Value` is simpler to maintain: adding
a new field to `Preferences` requires no changes to `store.rs`.  Rejected.

**YAML**
YAML supports comments and is human-readable, but its parser is notoriously
complex (YAML 1.2 spec is 80 pages), with well-documented footguns (Norway
problem, implicit typing, multi-document streams).  TOML is simpler and more
predictable.  Rejected.

## Consequences

**Positive:**
- `preferences.toml` is legible and hand-editable with standard TOML conventions
  (snake_case keys, inline comments permitted by the format).
- IPC naming (camelCase) and disk naming (snake_case) follow the appropriate
  convention for each context, with no coupling.
- `serde(default)` forward-compatibility guarantee is preserved unchanged.
- Migration from existing JSON installations is transparent.
- No additional runtime dependency: `toml = "0.8"` was already present.

**Negative / risks:**
- The camelCase ↔ snake_case bridge adds a conceptual indirection in `store.rs`.
  The conversion is tested via integration tests; a new preference field is
  automatically handled by the key-rename function without any code change.
- The `snake_to_camel` → re-serialize → `Preferences::deserialize` load path
  incurs two extra parse steps.  For a file under 50 KB read once at startup,
  this is imperceptible.
- `preferences.json` files from prior installations are not automatically deleted.
  Users with both files will use TOML (TOML takes priority); the JSON file
  becomes a stale artifact that they may delete manually.

## Notes

The on-disk file produced by TauTerm looks like:

```toml
[appearance]
font_family = "monospace"
font_size = 14.0
cursor_style = "block"
cursor_blink_ms = 530
theme_name = "Umbra"
language = "en"

[terminal]
scrollback_lines = 10000
allow_osc52_write = false
word_delimiters = " \t|\\"'`&()*,;<=>[]{}~"

[keyboard]
# shortcut bindings…
```

The file is written via `toml::to_string_pretty`, which groups scalar and table
keys naturally by section.
