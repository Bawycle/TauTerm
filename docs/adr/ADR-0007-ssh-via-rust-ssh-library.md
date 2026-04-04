<!-- SPDX-License-Identifier: MPL-2.0 -->

# ADR-0007 — SSH implementation via pure-Rust SSH library

**Date:** 2026-04-04
**Status:** Accepted

## Context

TauTerm must support SSH sessions (FS-SSH-*) with:
- TCP connection + SSH handshake (client side)
- Host key verification (TOFU model, own known-hosts file, FS-SSH-011)
- Authentication: publickey, keyboard-interactive, password (FS-SSH-012)
- PTY channel request with terminal dimensions and modes (FS-SSH-013)
- Keepalive (FS-SSH-020) and connection health monitoring (FS-SSH-022)
- SSH agent forwarding explicitly NOT supported (FS-SEC-004)
- Deprecated algorithm detection (FS-SSH-014)

Options are: delegate to the system `ssh` binary, use a native binding to libssh2, or use a pure-Rust SSH library.

## Decision

Use **`russh`** as the SSH client library for v1. `russh` is a pure-Rust, async/tokio-native SSH library (used by Lapce and other Rust tools), with no system library dependency. It integrates naturally with TauTerm's async architecture and simplifies packaging.

`ssh2-rs` (libssh2 bindings) is NOT chosen for v1. It is synchronous and would require `spawn_blocking` wrappers throughout the SSH code path, adding complexity and defeating the async design. It remains available as an emergency fallback if `russh` proves insufficient for a hard blocker, but that would require a new ADR revision.

**Known limitations of `russh` in v1 (documented, not blocking):**
1. **Deprecated algorithm detection (FS-SSH-014):** `russh` may not expose negotiated algorithm names after handshake via a stable API. If this information is unavailable, the deprecated-algorithm warning (FS-SSH-014) will be omitted in v1. This is a documented limitation, not a connection blocker — the connection is still established and fully functional. The requirement remains in FS.md; its implementation is conditional on `russh` API availability.
2. **Hardware keys (FIDO2/PIV):** `russh` does not support hardware security keys. This is explicitly out of scope for v1.

## Alternatives considered

**Spawning the system `ssh` binary**
Simple to implement: `tokio::process::Command` → `ssh user@host` with args. However:
- No programmatic access to host key fingerprints (FS-SSH-011 requires showing SHA-256 fingerprint before accepting)
- No access to lifecycle state transitions (FS-SSH-010 states: Connecting, Authenticating, Connected)
- Password authentication requires PTY passthrough which conflicts with TauTerm's PTY management
- No control over keepalive negotiation (FS-SSH-020)
- Cannot detect deprecated algorithms (FS-SSH-014)
- Cannot prevent agent forwarding (FS-SEC-004) — the system ssh may forward based on user's ssh config
Not chosen: too many hard requirements are unimplementable via subprocess delegation.

**`libssh` (via C bindings)**
A capable C library but adds a native dependency that must be present on the user's system or bundled. The pure-Rust option is preferable for portability and binary distribution (AppImage, deb). Not chosen as primary option.

**Implement SSH from scratch**
The SSH protocol is complex (RFC 4251-4254 plus many extension RFCs). Implementation risk is extreme. Not chosen.

## Consequences

**Positive:**
- Full programmatic control over the SSH handshake, authentication, and channel management enables implementation of all FS-SSH requirements.
- A pure-Rust library has no system library dependency, simplifying packaging and cross-compilation.
- Async (tokio) integration is natural for TauTerm's async backend.
- The SSH session lifecycle state machine (Connecting → Authenticating → Connected → Disconnected) is driven by library callbacks and maps directly to the `SshLifecycleState` type in the IPC contract.

**Negative / risks:**
- `russh` is less mature than `libssh2`. Correctness of edge cases (certain server configurations, deprecated algorithm negotiation) must be validated against real servers during testing.
- Deprecated algorithm detection (FS-SSH-014) requires access to negotiated algorithm names after handshake. If `russh` does not expose this information via a stable API, the warning is omitted in v1 (see Known limitations above).
- The credential store integration (FS-CRED-001: Secret Service / keychain) is independent of the SSH library; credentials are retrieved via the PAL `CredentialStore` trait (ADR-0005) and passed to the library's authentication callbacks.

**Note — hashed hostname entries in `~/.ssh/known_hosts` import:**
When the user triggers the import action (FS-SSH-011: preferences UI imports entries from `~/.ssh/known_hosts` into TauTerm's known-hosts file), entries with hashed hostnames (format `|1|<base64-salt>|<base64-HMAC-SHA1>`) MUST be silently skipped. The HMAC-SHA-1 hash is one-directional: the plaintext hostname cannot be recovered from the hash, so these entries cannot be imported into a usable form. The implementation MUST count skipped entries and display the count to the user (e.g., "12 entries could not be imported because their hostnames are hashed"). Plaintext hostname entries are imported normally.

**Debt:**
The crate selection is resolved: `russh` is chosen for v1. The FS-SSH-014 deprecated-algorithm warning is a conditional implementation — present if `russh` exposes the negotiated algorithm after handshake, omitted with documentation if not.
