# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project

TauTerm is a terminal emulator built with Tauri 2. The Rust backend handles PTY management and terminal state; the Svelte frontend renders the terminal UI.
La plateforme cible, pour cette première version, est Linux (x86, x86_64, ARM32, ARM64, RISC-V).

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
pnpm tauri build        # Production build (bundles Rust + frontend)
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

### E2E (tauri-driver + WebdriverIO)

```bash
pnpm tauri build                     # Build app first
pnpm wdio                            # Run WebdriverIO tests via tauri-driver
```

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

### Tauri config

- Window entry: `src-tauri/tauri.conf.json`
- Capabilities (permissions): `src-tauri/capabilities/default.json` — add new plugin permissions here
- Frontend build output: `build/` (SvelteKit static adapter)
- Dev URL: `http://localhost:1420`

## Agent Team

Development is coordinated by a multi-agent team defined in `.claude/agents/`. Agent definitions are project-scoped and versioned with the codebase.

| Agent file | Name | Role |
|---|---|---|
| `tauterm-moe.md` | `moe` | Maître d'Œuvre — orchestration, task decomposition, arbitration |
| `tauterm-user-rep.md` | `user-rep` | User Representative — personas, acceptance criteria, UX validation |
| `tauterm-domain-expert.md` | `domain-expert` | Terminal/PTY domain expert — VT standards, PTY Linux, SSH, app compatibility |
| `tauterm-architect.md` | `architect` | Software Architect — IPC design, state machines, ADRs |
| `tauterm-ux-designer.md` | `ux-designer` | UX/UI Designer — design tokens, themes, WCAG 2.1 AA, component specs |
| `tauterm-security-expert.md` | `security-expert` | Security Expert & Tester — threat modeling, PTY/IPC/SSH review |
| `tauterm-rust-dev.md` | `rust-dev` | Rust Developer — PTY, VT parser, screen buffer, Tauri commands |
| `tauterm-frontend-dev.md` | `frontend-dev` | Frontend Developer — Svelte 5, terminal rendering, Tauri IPC |
| `tauterm-test-engineer.md` | `test-engineer` | Test Engineer — nextest, vitest, WebdriverIO, no-regression policy |

**Team name:** `tauterm-team` — runtime config at `~/.claude/teams/tauterm-team/config.json`

**Typical feature flow:** `moe` decomposes → `domain-expert` + `architect` → `ux-designer` + `user-rep` → `rust-dev` + `frontend-dev` (parallel) → `test-engineer` + `security-expert` (review).

## License

This project is licensed under **MPL-2.0**. Every new source file must include the SPDX identifier as its first line:

- Rust, TypeScript, JavaScript: `// SPDX-License-Identifier: MPL-2.0`
- Svelte, HTML: `<!-- SPDX-License-Identifier: MPL-2.0 -->`
- CSS: `/* SPDX-License-Identifier: MPL-2.0 */`

Do not add SPDX headers to JSON, lock files, or binary files.

## Constraints

- No `unwrap()` on user-facing data in Rust — use `?` or explicit error handling
- Rust tests use `nextest` exclusively (not `cargo test`)
- Keep IPC commands serializable with `serde` — no raw pointers or OS handles across the boundary
- Rust edition: **2024** (`Cargo.toml`) — prefer its idioms (precise capturing, `impl Trait` in closures, etc.)
- CSP is currently `null` in `tauri.conf.json` — tighten it incrementally as features are added (allowed origins, `script-src`, `connect-src`)
