// SPDX-License-Identifier: MPL-2.0

//! E2E testing commands — only compiled with the `e2e-testing` feature.
//!
//! These commands must never appear in production builds.

use std::sync::Arc;

use parking_lot::Mutex;
use tauri::State;

use crate::platform::pty_injectable::InjectableRegistry;
use crate::session::ids::PaneId;

/// Push synthetic bytes directly into the VT pipeline for a pane.
///
/// The bytes bypass the real PTY and are delivered to the pane's VtProcessor
/// through the injectable mpsc channel. This is the primary mechanism for
/// E2E test determinism (ADR-0015).
///
/// The return type is `Result<(), String>` rather than a typed error struct.
/// Rationale: this is a testing command, not a production API. The frontend
/// test code checks for success/failure but does not need to discriminate error
/// variants. A plain `String` is acceptable here as a deliberate exception to
/// the IPC error typing rule.
#[tauri::command]
pub async fn inject_pty_output(
    pane_id: PaneId,
    data: Vec<u8>,
    registry: State<'_, Arc<InjectableRegistry>>,
) -> Result<(), String> {
    registry.send(&pane_id, data)
}

/// Counter that makes the next N `open_ssh_connection` calls fail immediately
/// with a synthetic error, regardless of pane ID.
///
/// Managed as Tauri state in e2e-testing builds. The `ssh_cmds::open_ssh_connection`
/// handler checks this counter before executing the real SSH open path. Each
/// check that finds a non-zero counter decrements it by one and returns the
/// synthetic error, so a single `inject_ssh_failure { count: 1 }` call causes
/// exactly one failure.
pub struct SshFailureRegistry(Mutex<u32>);

impl Default for SshFailureRegistry {
    fn default() -> Self {
        Self(Mutex::new(0))
    }
}

impl SshFailureRegistry {
    /// Create a registry with zero pending failures.
    pub fn new() -> Self {
        Self::default()
    }

    /// Schedule `count` consecutive `open_ssh_connection` failures.
    pub fn arm(&self, count: u32) {
        *self.0.lock() = count;
    }

    /// If there is at least one pending failure, decrement the counter and
    /// return `true`. The caller must then return the synthetic error.
    pub fn consume(&self) -> bool {
        let mut guard = self.0.lock();
        if *guard > 0 {
            *guard -= 1;
            true
        } else {
            false
        }
    }
}

/// Schedule `count` consecutive `open_ssh_connection` failures.
///
/// Call this before triggering the SSH open action in the frontend.
/// Each subsequent `open_ssh_connection` call decrements the counter and
/// returns a synthetic error until the counter reaches zero.
///
/// Typical usage in an E2E test: `inject_ssh_failure({ count: 1 })` before
/// clicking "Open in new tab" once — exactly one failure is injected, which
/// is enough to exercise the rollback path in `handleConnectionOpen`.
#[tauri::command]
pub async fn inject_ssh_failure(
    count: u32,
    failure_registry: State<'_, Arc<SshFailureRegistry>>,
) -> Result<(), String> {
    failure_registry.arm(count);
    Ok(())
}
