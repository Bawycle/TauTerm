// SPDX-License-Identifier: MPL-2.0

//! E2E testing commands — only compiled with the `e2e-testing` feature.
//!
//! These commands must never appear in production builds.

use std::sync::Arc;

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
