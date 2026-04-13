# Contributing to TauTerm

TauTerm is a personal project. Contributions are welcome, but review bandwidth is limited — there is no guaranteed response time.

Before starting significant work, please open an issue to discuss the approach. This avoids wasted effort on both sides.

To report a security vulnerability, see [SECURITY.md](SECURITY.md).

## Prerequisites

- **Rust 1.94.1** — pinned in `src-tauri/rust-toolchain.toml`, installed automatically by rustup
- **Node.js 22**
- **pnpm 10**
- System packages (Debian/Ubuntu):

```bash
sudo apt install libwebkit2gtk-4.1-dev libayatana-appindicator3-dev librsvg2-dev patchelf libssl-dev
```

Optional: **Podman** for running integration tests in containers.

## Getting Started

```bash
git clone git@github.com:Bawycle/TauTerm.git
cd TauTerm
pnpm install
pnpm tauri dev
```

Install the pre-push hook to run checks automatically before pushing:

```bash
./scripts/check-all.sh --install-hooks
```

## Verification

Run the full verification suite before submitting a pull request:

```bash
./scripts/check-all.sh           # Full suite (format + lint + tests + audit + containers + E2E)
./scripts/check-all.sh --fast    # Fast mode (format + lint + unit tests only)
```

The three mandatory steps can also be run individually:

| Step | Rust (from `src-tauri/`) | Frontend |
|---|---|---|
| Format | `cargo fmt -- --check` | `pnpm prettier --check src/` |
| Lint | `cargo clippy -- -D warnings` | `pnpm check` |
| Test | `cargo nextest run` | `pnpm vitest run` |

Optional integration and E2E tests require Podman and additional setup — see the Development section in [README.md](README.md).

## Coding Standards

### Commits

Use [Conventional Commits](https://www.conventionalcommits.org/):

```
type(scope): description
```

Types: `feat`, `fix`, `docs`, `style`, `chore`, `ci`.

Examples from the project history:

```
feat(webview): isolate WebKitGTK data directory per instance (ADR-0025)
fix(frontend): store selection in buffer-absolute coordinates
docs(todo): add multi-instance data dir isolation items
```

### License Headers

Every source file must have the MPL-2.0 SPDX identifier on its **first line**:

- Rust, TypeScript, JavaScript: `// SPDX-License-Identifier: MPL-2.0`
- Svelte, HTML: `<!-- SPDX-License-Identifier: MPL-2.0 -->`
- CSS: `/* SPDX-License-Identifier: MPL-2.0 */`

This is enforced by `scripts/check-spdx.sh` (part of the verification suite). JSON, lock files, and binary files are exempt.

### Rust

- Edition **2024**
- No `unwrap()` on user-facing data — use `?` or explicit error handling
- `unsafe` only in platform-specific modules (`platform/pty_*.rs`, `platform/clipboard_*.rs`), with a `// SAFETY:` comment
- Tests use `cargo nextest run` exclusively, not `cargo test`
- Never log filesystem paths containing usernames or home directories

Full rules: [`src-tauri/CLAUDE.md`](src-tauri/CLAUDE.md).

### Frontend

- Svelte 5 runes (`$state`, `$derived`, `$effect`)
- Every user-visible string through Paraglide: `import * as m from '$lib/paraglide/messages'`
- UI stack: Tailwind 4, Bits UI, Lucide — no hardcoded color or spacing values
- Formatting: Prettier with the project's `.prettierrc`

Full rules: [`src/CLAUDE.md`](src/CLAUDE.md).

### IPC

- Commands are coarse-grained (one per user action) and serializable with serde
- Events flow backend → frontend only; frontend uses `invoke()` for commands
- Multi-variant event payloads use discriminated unions (`#[serde(tag = "type")]`)
- Error types are serializable structs or enums, never bare strings

## Documentation to Read First

Before modifying an area, read the relevant documentation section (not the full document):

| Touching... | Read first... |
|---|---|
| UI feature or user-facing behavior | [`docs/UR.md`](docs/UR.md) + relevant file in [`docs/fs/`](docs/fs/) |
| Visual or UX decision | [`docs/uxd/`](docs/uxd/) + [`docs/AD.md`](docs/AD.md) |
| Architecture or IPC | [`docs/arch/`](docs/arch/) + relevant [`docs/adr/`](docs/adr/) |
| Test strategy | [`docs/testing/TESTING.md`](docs/testing/TESTING.md) |

## Pull Request Process

1. Branch from `dev`, not `master`
2. Use a descriptive branch name (e.g., `feat/osc8-hyperlinks`, `fix/scroll-reset`)
3. Run `./scripts/check-all.sh` — CI runs the same checks
4. PRs targeting `master` also trigger integration and E2E tests via CI

## License

By contributing, you agree that your contributions are licensed under the [MPL-2.0](LICENSE), the same license as the project.
