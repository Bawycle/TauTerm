# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project

TauTerm is a terminal emulator built with Tauri 2. The Rust backend handles PTY management and terminal state; the Svelte frontend renders the terminal UI.
La plateforme cible, pour cette première version, est Linux (x86_64). ARM64 (aarch64) est supporté mais sans binaires ni packages distribués — les utilisateurs doivent compiler depuis les sources.

## Documentation

- [`docs/UR.md`](docs/UR.md) — User Requirements: personas, interaction model, feature requirements (source of truth for what users need)
- [`docs/fs/`](docs/fs/) — Functional Specifications: numbered requirements (`FS-XXX-NNN`) with acceptance criteria, MoSCoW priorities, and traceability to UR. Entry point: `docs/fs/README.md`. **The only document that uses normative language (`MUST`/`SHALL`/`SHOULD`/`MAY`). Source of truth for what the system must do.**
- [`docs/AD.md`](docs/AD.md) — Artistic Direction: visual identity, Umbra theme design intent, typography and color philosophy (source of truth for aesthetic decisions)
- [`docs/uxd/`](docs/uxd/) — UX/UI Design: design token system, component specifications, interaction patterns, IPC contract. Entry point: `docs/uxd/README.md`. **Source of truth for how things look and behave from a UX perspective. Never restates FS requirements — references FS IDs instead.**
- [`docs/arch/`](docs/arch/) — Architecture: module decomposition, IPC contract, state machines, concurrency model, platform abstraction, security strategy (source of truth for how the system is built). Entry point: `docs/arch/README.md`.
- [`docs/testing/TESTING.md`](docs/testing/TESTING.md) — Testing strategy: test pyramid, unit/integration/E2E, coverage policy, no-regression policy
- [`docs/adr/`](docs/adr/) — Architecture Decision Records: rationale behind structural decisions (ADR-0001 through ADR-0015)
- [`docs/test-protocols/functional-pty-vt-ssh-preferences-ui-ipc.md`](docs/test-protocols/functional-pty-vt-ssh-preferences-ui-ipc.md) — Functional Test Protocol: 93 scenarios covering PTY/session, VT parser, SSH lifecycle, preferences & i18n, UI & accessibility, IPC contract
- [`docs/test-protocols/security-pty-ipc-ssh-credentials-csp-osc52.md`](docs/test-protocols/security-pty-ipc-ssh-credentials-csp-osc52.md) — Security Test Protocol: threat model, 28 scenarios covering PTY injection, IPC boundary, SSH auth, credential storage, CSP/WebView, OSC52, input validation

## Commands

### Development

```bash
pnpm tauri dev          # Start full app (Rust backend + Vite frontend on :1420)
pnpm dev                # Frontend only (Vite dev server)
pnpm check              # TypeScript/Svelte type checking
pnpm check:watch        # Watch mode type checking
```

### Build

```bash
pnpm tauri build        # Production build (bundles Rust + frontend); set bundle.targets: ["appimage"] in tauri.conf.json to produce AppImage
pnpm build              # Frontend only
```

### Rust (run from src-tauri/)

```bash
cargo nextest run                    # Run all tests
cargo nextest run <test_name>        # Run a single test
cargo clippy -- -D warnings         # Lint (must pass clean)
cargo fmt                            # Format
cargo fmt -- --check                 # Check formatting without writing
```

### Frontend

```bash
pnpm vitest                          # Unit tests
pnpm prettier --check src/          # Check formatting
pnpm prettier --write src/          # Format
```

### SecretService integration tests (Podman — Linux only)

```bash
./scripts/run-keyring-tests.sh             # build image + run
./scripts/run-keyring-tests.sh --no-build  # reuse existing image
```

These tests (`src-tauri/tests/credentials_integration.rs`) require a live GNOME Keyring
daemon and are **not** included in `cargo nextest run`. They run in an isolated Podman
container with a virtual framebuffer (Xvfb) and auto-dismissed password prompt (xdotool).
See `docs/testing/TESTING.md` for the full rationale.

### E2E (tauri-driver + WebdriverIO)

```bash
pnpm tauri build --no-bundle -- --features e2e-testing   # --no-bundle skips AppImage/deb packaging; --features e2e-testing is MANDATORY (enables inject_pty_output)
pnpm wdio                                                 # Run WebdriverIO tests via tauri-driver
```

> **Note:** the `--features e2e-testing` flag is mandatory. Without it, `inject_pty_output` is not
> compiled in, injections are silently ignored, and PTY round-trip tests fail. `--no-bundle` skips
> AppImage/deb packaging and speeds up the build.

## Architecture

### IPC Boundary

All frontend↔backend communication goes through Tauri commands (`#[tauri::command]` in Rust, `invoke()` in TypeScript). Keep commands coarse-grained: one command per user action, not per data field.

### Rust backend (`src-tauri/src/`)

- `lib.rs` — Tauri app setup: plugin registration, command handler registration via `generate_handler![]`, `run()` entrypoint
- `main.rs` — thin entrypoint delegating to `tau_term_lib::run()`
- New modules go in `src-tauri/src/` and are declared in `lib.rs`

Architecture target: terminal state machine in Rust (PTY, VT parser, screen buffer), exposed to frontend via Tauri commands and events. Use `tauri::AppHandle` for emitting events to the window.

### Svelte frontend (`src/`)

- `src/routes/+page.svelte` — main terminal view
- `src/routes/+layout.ts` — SSR disabled (`export const ssr = false` — required for Tauri)
- Static SPA adapter (no server-side rendering); fallback to `index.html`

State management: Svelte 5 runes (`$state`, `$derived`, `$effect`). No centralized store needed unless cross-component state grows complex — prefer component-local state first.

UI stack: **Tailwind 4** (utility classes), **Bits UI** (headless primitives), **Lucide-svelte** (icons). Use design tokens via Tailwind's `@theme` — no hardcoded color/spacing values.

### i18n (Paraglide JS)

- `src/lib/i18n/messages/en.json` + `fr.json` — locale catalogues (source of truth)
- `src/lib/paraglide/` — **generated code, do not commit, do not edit manually** (listed in `.gitignore`). Auto-generated by Vite at `pnpm dev` / `pnpm build` startup — no manual generation step needed.
- `src/lib/state/locale.svelte.ts` — reactive locale state; `setLocale()` persists via IPC
- Components import message accessors: `import * as m from '$lib/paraglide/messages'`, then `{m.some_key()}`. **Never use `{@html}` with message accessors.**
- Every user-visible string must go through Paraglide — including `aria-label`, `title`, and tooltip attributes. Exception: non-translatable proper nouns (e.g. "TauTerm" as a brand name).

### Tauri config

- Window entry: `src-tauri/tauri.conf.json`
- Capabilities (permissions): `src-tauri/capabilities/default.json` — add new plugin permissions here
- Frontend build output: `build/` (SvelteKit static adapter)
- Dev URL: `http://localhost:1420`

## Agent Team

Development is coordinated by a multi-agent team defined in `.claude/agents/`. Agent definitions are project-scoped and versioned with the codebase.

| Agent file | Name | Role |
|---|---|---|
| `tauterm-moe.md` | `moe` | Maître d'Œuvre — arbitration, quality gate, escalation |
| `tauterm-user-rep.md` | `user-rep` | User Representative — personas, acceptance criteria, UX validation |
| `tauterm-domain-expert.md` | `domain-expert` | Terminal/PTY domain expert — VT standards, PTY Linux, SSH, app compatibility |
| `tauterm-architect.md` | `architect` | Software Architect — IPC design, state machines, ADRs |
| `tauterm-ux-designer.md` | `ux-designer` | UX/UI Designer — design tokens, themes, WCAG 2.1 AA, component specs |
| `tauterm-security-expert.md` | `security-expert` | Security Expert & Tester — threat modeling, PTY/IPC/SSH review |
| `tauterm-rust-dev.md` | `rust-dev` | Rust Developer — PTY, VT parser, screen buffer, Tauri commands |
| `tauterm-frontend-dev.md` | `frontend-dev` | Frontend Developer — Svelte 5, terminal rendering, Tauri IPC |
| `tauterm-test-engineer.md` | `test-engineer` | Test Engineer — nextest, vitest, WebdriverIO, no-regression policy |
| `tauterm-perf-expert.md` | `perf-expert` | Performance Expert — profiling, throughput/latency analysis, Rust + Svelte 5 optimization |

**Team name:** `tauterm-team` — runtime config at `~/.claude/teams/tauterm-team/config.json`

**Typical feature flow:** `domain-expert` + `architect` (constraints) → `ux-designer` + `user-rep` (UX/acceptance) → `rust-dev` + `frontend-dev` in parallel (implementation) → `test-engineer` + `security-expert` + `perf-expert` (review). `moe` arbitrates when specialists disagree and validates quality gates.

## License

This project is licensed under **MPL-2.0**. Every new source file must include the SPDX identifier as its first line:

- Rust, TypeScript, JavaScript: `// SPDX-License-Identifier: MPL-2.0`
- Svelte, HTML: `<!-- SPDX-License-Identifier: MPL-2.0 -->`
- CSS: `/* SPDX-License-Identifier: MPL-2.0 */`

Do not add SPDX headers to JSON, lock files, or binary files.

## Working with the Documentation

These documents are large. **Never read a doc file in full** — always identify the relevant section first (from the table of contents or heading structure), then read only that section.

Before making any decision or modification — code or documentation — read the relevant sections first:

| If touching… | Read first… |
|---|---|
| Any UI feature or user-facing behaviour | Relevant section in `docs/UR.md` + matching `FS-*` file in `docs/fs/` (see `docs/fs/README.md`) |
| Any visual/UX decision (layout, tokens, components) | Relevant section in `docs/uxd/` (see `docs/uxd/README.md` for the section index); `docs/AD.md` only if aesthetic decisions are involved |
| Any architectural decision (modules, IPC, state, data flow) | Relevant section in `docs/arch/` (see `docs/arch/README.md`) + the ADR(s) it references in `docs/adr/` |
| Any test strategy or coverage decision | `docs/testing/TESTING.md` |
| Any new feature end-to-end | All of the above, section by section |

This rule applies equally to all agents. Reading a section takes a few hundred lines; reading a whole document wastes context and defeats the purpose.

### FS.md vs UXD.md — SSOT partition rule

These two documents have a strict, non-overlapping responsibility boundary:

| Question | Belongs in |
|---|---|
| What must the system do? | `FS.md` only |
| How does it look / feel / animate? | `UXD.md` only |
| Is it testable as a pass/fail criterion? | `FS.md` only |
| Does a designer need it to draw a mockup? | `UXD.md` only |

**Hard rules — violation = SSOT breach:**
- Normative language (`MUST`, `SHALL`, `SHOULD`, `MAY`) is **exclusively** `docs/fs/`. Never write it in `docs/uxd/`.
- If a requirement is in `docs/fs/`, `docs/uxd/` references its ID (`(FS-TAB-009)`) and describes only the *design expression* — it does not restate the requirement.
- Implementation details (API names, CSS property names, algorithm choices) belong in source code and comments, not in either doc.
- When adding to either document, check the other first: if the information already exists there in any form, add a cross-reference instead of duplicating.

## Constraints

- No `unwrap()` on user-facing data in Rust — use `?` or explicit error handling
- Rust tests use `nextest` exclusively (not `cargo test`)
- Keep IPC commands serializable with `serde` — no raw pointers or OS handles across the boundary
- Rust edition: **2024** (`Cargo.toml`) — prefer its idioms (precise capturing, `impl Trait` in closures, etc.)
- CSP is configured in `tauri.conf.json` — `style-src 'unsafe-inline'` is retained for Tailwind 4 dev mode and must be reviewed when production build is validated
- `language` field in `AppearancePrefs` MUST be `enum Language { En, Fr }` — never a free `String` across IPC (FS-I18N-006)
- Never log filesystem paths that include usernames or home directories (e.g. `/home/<user>/…`). Log the filename or a generic label only (e.g. `"preferences.json"`, not `path.display()`).

### Svelte coding rules

See [`src/CLAUDE.md`](src/CLAUDE.md) for Svelte-specific rules (runes patterns, component IDs, i18n, UI stack).

### Rust safety rules

See [`src-tauri/CLAUDE.md`](src-tauri/CLAUDE.md) for Rust-specific rules (type casts, DashMap, unsafe, underflow guards, etc.).

### IPC event rules

- Backend events that affect a specific entity (tab, pane, session) must include **that entity's ID** explicitly in the payload (e.g. `closed_tab_id`, not just `active_tab_id`). Frontend handlers must use that ID to locate and update the correct entity — never infer the affected entity from implicit context.
- Multi-variant events (`session-state-changed`, etc.) must use a discriminated payload (`#[serde(tag = "type")]` in Rust, typed union in TypeScript). Never use a flat struct with optional fields that forces the handler to infer the operation from which fields are present or absent.
- IPC error types crossing the boundary must be `#[derive(Serialize)]` structs or enums — never bare `format!("{e}")` strings. The frontend must be able to discriminate errors by type, not by string content.
- Events flow in one direction only: backend → frontend. Frontend actions always go through `invoke()` commands.

## État d'avancement

See [`TODO.md`](TODO.md) for the full list of pending work (gaps identified by cross-referencing docs vs codebase).
