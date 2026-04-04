<!-- SPDX-License-Identifier: MPL-2.0 -->

# ADR-0002 — PTY management in native Rust

**Date:** 2026-04-04
**Status:** Accepted

## Context

TauTerm must create and manage pseudo-terminal pairs (master + slave) for each pane. Requirements include:
- `openpty()` / `posix_openpt()` to allocate PTY pairs
- `fork()` + `setsid()` + `TIOCSCTTY` to spawn child processes with a controlling terminal
- `ioctl(TIOCSWINSZ)` + SIGWINCH delivery on resize
- Non-blocking async I/O on the master fd (reading PTY output, writing keyboard input)
- Proper file descriptor hygiene (`O_CLOEXEC`, closing fds in fork child/parent)
- SIGCHLD / waitpid for exit detection

The question is whether to implement this directly in Rust using `libc` (and/or a thin PTY crate), or to delegate PTY management to an external process or helper binary.

## Decision

Implement PTY management **directly in Rust** using `libc` bindings (via the `libc` crate) and the `nix` crate for ergonomic POSIX wrappers, wrapped behind a `PtyBackend` trait (see ADR-0005 for the Platform Abstraction Layer).

The `portable-pty` crate (from the WezTerm project) provides a ready-made cross-platform PTY abstraction and is the preferred implementation vehicle for the `PtyBackend` trait's Linux implementation. If `portable-pty` proves unsuitable (API constraints, dependency conflicts, licensing), a bespoke `libc`-based implementation is the fallback.

## Alternatives considered

**Delegating PTY management to a subprocess (helper binary)**
Some terminal emulators spawn a dedicated helper process (similar to a PTY server) that manages all PTY operations and communicates with the main process via a socket. This provides OS-level isolation but adds latency, complexity, and a separate binary to maintain. Given that Tauri's Rust backend already runs in the same process with full OS access, this indirection provides no benefit and only adds overhead. Not chosen.

**Using an existing terminal emulator library (alacritty-terminal, wezterm-term)**
These libraries provide PTY management bundled with a VT parser and screen buffer. The coupling is appropriate for their design goals but too rigid for TauTerm: it would constrain the VT parser choice (see ADR-0003) and make the abstraction layers harder to test independently. Not chosen; the VT parser decision is kept separate.

**`asyncio`-style subprocess management (Tokio `process::Command`)**
Tokio's `process::Command` creates child processes but does not set up a controlling terminal (no `setsid` + `TIOCSCTTY`). Programs that require a real PTY (vi, bash interactive mode, any program that calls `isatty()`) would malfunction. Not chosen: a real PTY pair is a hard requirement.

## Consequences

**Positive:**
- Full control over PTY lifecycle, fd management, and environment setup (FS-PTY-001 through FS-PTY-014).
- The `PtyBackend` trait (ADR-0005) hides platform differences; the Linux implementation is encapsulated behind it.
- `portable-pty` provides tested cross-platform PTY code including resize and Windows ConPTY support, enabling the v2 cross-platform roadmap without a rewrite.
- Async I/O on the PTY master fd integrates cleanly with Tokio: the fd is registered as a `tokio::io::unix::AsyncFd`, enabling non-blocking reads with proper back-pressure.

**Negative / risks:**
- Direct `libc` / `nix` usage requires careful fd management. Errors (e.g., failing to close the slave fd in the parent) are silent and produce subtle bugs. This is mitigated by encapsulating all fork/exec logic in a single, well-tested function within the PTY module.
- `portable-pty` adds a dependency from the WezTerm ecosystem. If the crate's API evolves incompatibly, migration is bounded to the `PtyBackend` Linux implementation.

**Debt:**
None. The abstraction layer (ADR-0005) ensures the PTY implementation is swappable without touching the rest of the codebase.
