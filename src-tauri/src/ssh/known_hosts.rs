// SPDX-License-Identifier: MPL-2.0

//! TauTerm known-hosts file management.
//!
//! Maintains `~/.config/tauterm/known_hosts` in OpenSSH-compatible format.
//! Implements TOFU (Trust On First Use) host key verification.
//!
//! The OpenSSH `~/.ssh/known_hosts` is NOT read automatically at startup.
//! It can be imported on explicit user request via the Preferences UI (§8.3).

// Stub — full implementation in SSH integration pass.
