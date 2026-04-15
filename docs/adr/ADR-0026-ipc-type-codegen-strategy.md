<!-- SPDX-License-Identifier: MPL-2.0 -->

# ADR-0026 — IPC type codegen strategy

**Date:** 2026-04-15
**Status:** Accepted

## Context

The TauTerm IPC surface currently comprises approximately 40 manually mirrored
types, 36 commands, and 16 events. Each Rust struct or enum in
`src-tauri/src/events/types.rs` and the command modules has a hand-written
TypeScript counterpart in `src/lib/ipc/types.ts`, `commands.ts`, and
`events.ts`. These files are maintained in sync by convention — there is no
automated check that the two sides agree on field names, field presence, serde
rename rules, or discriminated-union tag shapes.

An audit of the IPC layer (2026-04-15) confirmed four drift bugs:

1. **`duplicateConnection` parameter name mismatch (runtime bug).** The Rust
   command `duplicate_connection` declares its parameter as `connection_id:
   ConnectionId` (serde-renamed to `connectionId`). The TypeScript wrapper sent
   `{ id }` instead of `{ connectionId }`, causing a silent deserialization
   failure at runtime. This is the class of bug that is structurally impossible
   to prevent with manual mirroring.

2. **`SnapshotCell.width` documentation divergence.** The Rust doc describes
   width `0` as a "phantom continuation slot" following a wide character. The
   TypeScript JSDoc said "0 for combining", which is a different (and
   incorrect) semantic.

3. **Three commands with no type aliases.** `duplicate_connection`,
   `store_connection_password`, and `provide_passphrase` had invoke wrappers in
   `commands.ts` but no corresponding `*Command` type alias in `types.ts`,
   breaking the convention used by all other commands and making the IPC
   contract incomplete as a reference.

4. **`Osc52WriteRequestedEvent` missing from TypeScript.** The Rust backend
   emits `osc52-write-requested` events (implemented in `events/types.rs` and
   `session/pty_task/emitter.rs`), but the TypeScript side had no corresponding
   interface, no event listener, and no subscription in `useTerminalPane` —
   meaning OSC 52 clipboard writes were silently dropped by the frontend.

The `duplicateConnection` runtime bug satisfies the trigger threshold defined
in `TODO.md` for adopting codegen: a confirmed runtime parameter-name mismatch
caused by manual mirroring.

## Decision

Adopt **`tauri-specta`** for IPC type codegen.

## Options evaluated

### Option A — `tauri-specta` (types + invoke wrappers) — SELECTED

[`tauri-specta`](https://github.com/oscartbeaumont/tauri-specta) generates both
TypeScript types and invoke wrapper functions from annotated Rust types and
Tauri commands. It uses [`specta`](https://github.com/oscartbeaumont/specta)
for the type reflection layer.

**How it works:**

- All IPC types derive `specta::Type` in addition to `Serialize`/`Deserialize`.
- Command handlers are registered via `tauri_specta::Builder` instead of the
  plain `tauri::Builder::invoke_handler`.
- A build-time (or test-time) step calls `builder.export()` to emit
  `types.ts` and `commands.ts` as build artifacts.
- Serde attributes (`rename_all`, `tag`, `skip_serializing_if`) are honoured by
  specta's reflection, so the generated TypeScript matches the actual wire format.

**Pros:**

- Eliminates the entire class of parameter-name and field-name drift bugs,
  including the `duplicateConnection` bug that triggered this ADR.
- Handles `#[serde(tag = "type")]` discriminated unions correctly — the
  generated TypeScript uses the same tag field and variant names.
- Generates invoke wrappers with correct parameter names, so the frontend never
  constructs `invoke()` payloads manually.
- Well-maintained, Tauri 2-compatible (v2 support since `tauri-specta` 2.0).
- Migration can be done progressively (see Migration Plan below).

**Cons:**

- Adds `specta::Type` derive to every IPC-facing type. This is a compile-time
  and coupling cost: removing `tauri-specta` later requires removing the derive
  from all annotated types.
- Codegen step adds ~5 seconds to cold build (type reflection + file write).
- Generated code style may differ from the current hand-written style, requiring
  the team to accept the generated output as-is (or add a post-generation
  formatting step).
- Risk of lag between Tauri 2.x patch releases and `tauri-specta` compatibility.
  Mitigated by the fact that `tauri-specta` tracks Tauri releases closely.

### Option B — `ts-rs` (types only) — REJECTED

[`ts-rs`](https://github.com/Aleph-Alpha/ts-rs) generates TypeScript type
definitions from Rust types, but does **not** generate invoke wrappers.

This addresses the doc divergence and missing-type bugs (items 2–4 above) but
does not prevent the `duplicateConnection` class of bug (item 1), because the
invoke wrapper parameter names are still hand-written. Since item 1 is the
highest-severity bug and the primary trigger for this decision, `ts-rs` is
insufficient.

### Option C — Conformity tests (no codegen) — REJECTED

Write compile-time or CI-time tests that assert the TypeScript types match the
Rust types (e.g., by parsing both sides and comparing field names).

This detects drift after the fact but does not prevent it. The maintenance
burden scales linearly with the number of IPC types and commands. It also
requires the conformity test itself to be kept in sync — a meta-drift problem.
The codegen approach is structurally superior: it makes drift impossible rather
than merely detectable.

## Migration plan

Progressive migration in four independently shippable PRs:

1. **Annotate event types:** add `specta::Type` derive to all structs and enums
   in `src-tauri/src/events/types.rs` and command parameter/return types.
   No generated output yet — this is a compile-only change.

2. **Switch to generated types:** configure `tauri_specta::Builder`, run the
   export step, and replace `src/lib/ipc/types.ts` with the generated output.
   Hand-written JSDoc comments move to Rust doc-comments (which specta
   propagates to the generated TypeScript).

3. **Migrate command signatures and events:** register commands via
   `tauri_specta::Builder::invoke_handler()` instead of the manual
   `.invoke_handler(tauri::generate_handler![...])`. Derive
   `tauri_specta::Event` on event structs and register them via
   `collect_events![]`. Replace `src/lib/ipc/commands.ts` and
   `src/lib/ipc/events.ts` with the generated wrappers.

4. **Delete hand-written artifacts:** remove the old `types.ts`,
   `commands.ts`, and `events.ts` from version control. Add the generated
   files to `.gitignore` (or commit them as build artifacts — to be decided
   in a follow-up ADR if needed). Delete the `commands.test.ts`
   parameter-name tests that are now redundant.

Each PR is independently shippable: at every step, the application compiles
and all existing tests pass. During the transition (PRs 1–3), hand-written
types coexist with generated types; the generated types are authoritative.

## Consequences

**Positive:**

- Parameter-name drift bugs (like `duplicateConnection`) become structurally
  impossible. The generated invoke wrappers use the exact parameter names
  derived from the Rust function signatures via serde rename rules.
- `types.ts` and `commands.ts` become build artifacts derived from Rust source,
  eliminating the manual mirroring maintenance burden.
- Serde attributes (`rename_all`, `tag`, `skip_serializing_if`) are handled
  correctly by specta's reflection — no more manual translation of rename rules.
- New commands and event types are automatically available in TypeScript after
  adding the `specta::Type` derive — no separate TypeScript authoring step.

**Negative / risks:**

- `specta::Type` derive coupling on all IPC-facing types. If `tauri-specta` is
  abandoned or becomes incompatible with a future Tauri version, removing the
  derive from ~40 types is tedious but mechanical.
- Codegen step adds ~5 seconds to cold build. Incremental builds are not
  affected (the export only runs when IPC types change).
- During migration (PRs 1–3): generated types are authoritative, but
  hand-written types still exist in the repo. Contributors must be aware that
  edits to `types.ts` during this window will be overwritten.
- `tauri-specta` v2 supports typed events via `#[derive(tauri_specta::Event)]`
  and `collect_events![]`, generating typed TypeScript listeners alongside
  command wrappers. Both `events.ts` and `commands.ts` can be fully generated.
  The migration plan (steps 1–4) covers both.
