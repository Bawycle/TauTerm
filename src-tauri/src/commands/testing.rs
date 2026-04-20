// SPDX-License-Identifier: MPL-2.0

//! E2E testing commands — only compiled with the `e2e-testing` feature.
//!
//! These commands must never appear in production builds.

use std::sync::Arc;

use parking_lot::Mutex;
use tauri::State;

use crate::platform::pty_injectable::InjectableRegistry;
use crate::session::ids::PaneId;
use crate::session::registry::SessionRegistry;

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
#[specta::specta]
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
#[specta::specta]
pub async fn inject_ssh_failure(
    count: u32,
    failure_registry: State<'_, Arc<SshFailureRegistry>>,
) -> Result<(), String> {
    failure_registry.arm(count);
    Ok(())
}

// ---------------------------------------------------------------------------
// SSH connect-task delay injection
// ---------------------------------------------------------------------------

/// Milliseconds to sleep at the very start of the next `connect_task` run,
/// after the `Connecting` state event has already been emitted and the
/// overlay is visible on screen.
///
/// Single-shot: the value is atomically swapped to 0 on first read, so it
/// fires at most once per call to `inject_ssh_delay`.
static SSH_CONNECT_DELAY_MS: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

/// Consume the pending connect-task delay (resets to 0).
///
/// Returns `Some(ms)` if a non-zero delay was pending, `None` otherwise.
pub fn consume_ssh_connect_delay() -> Option<u64> {
    let v = SSH_CONNECT_DELAY_MS.swap(0, std::sync::atomic::Ordering::SeqCst);
    if v > 0 { Some(v) } else { None }
}

/// Arm the next `connect_task` with a synthetic delay.
///
/// The delay is inserted at the very start of `connect_task`, which runs
/// after `open_connection_inner` has already emitted the `Connecting` state
/// event and the overlay is rendered.  This holds the connection in
/// `connecting` state long enough for WebdriverIO to observe it.
///
/// Single-shot: consumed (zeroed) the first time `connect_task` runs after
/// this is set.
#[tauri::command]
#[specta::specta]
pub async fn inject_ssh_delay(delay_ms: u64) -> Result<(), String> {
    SSH_CONNECT_DELAY_MS.store(delay_ms, std::sync::atomic::Ordering::SeqCst);
    Ok(())
}

// ---------------------------------------------------------------------------
// SSH synthetic disconnect injection
// ---------------------------------------------------------------------------

/// Force-emit a `Disconnected` state-changed event for a pane.
///
/// Used by `ssh-overlay-states.spec.ts` to make the connecting overlay
/// disappear without depending on a real TCP connection failing.  The
/// `connect_task` continues running in the background (we cannot cancel it
/// after it has been spawned), but the frontend immediately sees `Disconnected`
/// and removes the overlay.  Any subsequent `Disconnected` emitted by the
/// connect_task when it eventually times out is a no-op (idempotent state).
#[tauri::command]
#[specta::specta]
pub async fn inject_ssh_disconnect(
    pane_id: crate::session::ids::PaneId,
    ssh_manager: tauri::State<'_, std::sync::Arc<crate::ssh::SshManager>>,
    app: tauri::AppHandle,
) -> Result<(), String> {
    use crate::events::{SshStateChangedEvent, emit_ssh_state_changed};
    use crate::ssh::SshLifecycleState;

    // Remove the connection entry and any pending prompts so the background
    // connect_task's own cleanup (which also calls remove) becomes a safe no-op.
    ssh_manager.purge_pane(&pane_id);
    ssh_manager.pending_passphrases.remove(&pane_id);

    emit_ssh_state_changed(
        &app,
        SshStateChangedEvent {
            pane_id: pane_id.clone(),
            state: SshLifecycleState::Disconnected {
                reason: Some("E2E test synthetic disconnect".to_string()),
            },
        },
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// Credential-prompt injection
// ---------------------------------------------------------------------------

/// Directly emit a `credential-prompt` event for a pane without requiring a
/// live SSH auth flow.
///
/// This lets E2E tests assert that the `.ssh-credential-dialog` renders when
/// a `credential-prompt` event arrives (frontend rendering path exercised).
///
/// No sender is stored in `pending_credentials`.  When the test submits the
/// dialog, `handleProvideCredentials` calls `clearCredentialPrompt()` first
/// (which closes the dialog) then invokes `provide_credentials` IPC — that
/// call returns `NoPendingCredentials`, which is silently swallowed by the
/// `catch {}` in the frontend handler.  The dialog correctly closes.
#[tauri::command]
#[specta::specta]
pub async fn inject_credential_prompt(
    pane_id: crate::session::ids::PaneId,
    host: String,
    username: String,
    app: tauri::AppHandle,
) -> Result<(), String> {
    use crate::events::{CredentialPromptEvent, emit_credential_prompt};

    emit_credential_prompt(
        &app,
        CredentialPromptEvent {
            pane_id,
            host,
            username,
            prompt: None,
            failed: false,
            is_keychain_available: false,
        },
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// PTY process exit injection
// ---------------------------------------------------------------------------

/// Simulate the termination of the process running in a pane.
///
/// Marks the pane as `Terminated` with the given `exit_code` and emits a
/// `notification-changed` event with a `ProcessExited` payload, reproducing
/// the same flow triggered by the PTY read task after a real EOF.
///
/// Used by E2E tests to exercise the "process exited" UI path (overlay, restart
/// button, etc.) without requiring a real process to die.
#[tauri::command]
#[specta::specta]
pub async fn inject_pane_exit(
    pane_id: PaneId,
    exit_code: i32,
    registry: State<'_, Arc<SessionRegistry>>,
    app: tauri::AppHandle,
) -> Result<(), String> {
    use crate::events::{
        emit_notification_changed,
        types::{NotificationChangedEvent, PaneNotificationDto},
    };

    // Transition pane to Terminated — uses a caller-supplied exit code instead of
    // waiting on the real child process.
    registry
        .set_pane_terminated_with_code(&pane_id, exit_code)
        .map_err(|e| e.to_string())?;

    // Emit the ProcessExited notification, mirroring the emit task's post-EOF path.
    if let Some((_, tab_state)) = registry.get_tab_state_for_pane(&pane_id) {
        emit_notification_changed(
            &app,
            NotificationChangedEvent {
                tab_id: tab_state.id,
                pane_id,
                notification: Some(PaneNotificationDto::ProcessExited {
                    exit_code: Some(exit_code),
                    signal_name: None,
                }),
            },
        );
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Foreground process injection
// ---------------------------------------------------------------------------

/// Inject a synthetic foreground process name for a pane.
///
/// After this call, `has_foreground_process` returns `true` for `pane_id`,
/// simulating a pane with a non-shell process running in the foreground.
/// This enables E2E tests to exercise the "close pane with foreground process"
/// confirmation dialog without spawning a real long-running process.
///
/// Call `inject_foreground_process` with an empty `process_name` to clear the
/// injection and restore the real `tcgetpgrp` behaviour.
#[tauri::command]
#[specta::specta]
pub async fn inject_foreground_process(
    pane_id: PaneId,
    process_name: String,
    registry: State<'_, Arc<SessionRegistry>>,
) -> Result<(), String> {
    if process_name.is_empty() {
        registry.injected_foreground.remove(&pane_id);
    } else {
        registry.injected_foreground.insert(pane_id, process_name);
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// SSH output injection (E2E)
// ---------------------------------------------------------------------------

/// Push synthetic bytes into the SSH VT pipeline for a pane.
///
/// The bytes are processed through the pane's `VtProcessor` (identical to the
/// real SSH reader in `ssh_task::extract_process_output`) and the resulting
/// `ProcessOutput` is sent to the coalescer via `SshInjectableRegistry`.
///
/// Requires a prior `create_mock_ssh_pane` call to wire up the coalescer
/// pipeline for the target pane.
#[tauri::command]
#[specta::specta]
pub async fn inject_ssh_output(
    pane_id: PaneId,
    data: Vec<u8>,
    registry: State<'_, Arc<SessionRegistry>>,
    ssh_registry: State<'_, Arc<crate::session::ssh_injectable::SshInjectableRegistry>>,
) -> Result<(), String> {
    let vt = registry.get_pane_vt(&pane_id).map_err(|e| e.to_string())?;

    let (output, _responses) = crate::session::ssh_task::extract_process_output(&vt, &data);

    // VT responses (CPR/DSR/DA) are discarded — no real SSH channel to write
    // them back to. This is acceptable for E2E test injection.
    ssh_registry.send(&pane_id, output).await
}

// ---------------------------------------------------------------------------
// Mock SSH pane creation (E2E)
// ---------------------------------------------------------------------------

/// Create a mock SSH pane for E2E testing without a real SSH connection.
///
/// Takes the existing PTY pane (already present in the registry from
/// `create_tab` at startup) and wires it as if it were an SSH-connected pane:
///
/// 1. Sets `pane.ssh_state = Some(Connected)`.
/// 2. Creates a bounded `mpsc::channel::<ProcessOutput>(256)`.
/// 3. Registers the sender in `SshInjectableRegistry`.
/// 4. Spawns a coalescer task (Task B) that consumes from the channel and
///    emits screen-update events — the same pipeline as a real SSH pane, but
///    without the reader (Task A) and without a russh channel.
/// 5. Emits an `ssh-state-changed` event with `Connected` so the frontend
///    sees the pane as SSH-connected.
///
/// After this call, `inject_ssh_output` can push bytes into the SSH VT
/// pipeline for this pane.
#[tauri::command]
#[specta::specta]
pub async fn create_mock_ssh_pane(
    pane_id: PaneId,
    registry: State<'_, Arc<SessionRegistry>>,
    ssh_registry: State<'_, Arc<crate::session::ssh_injectable::SshInjectableRegistry>>,
    app: tauri::AppHandle,
) -> Result<(), String> {
    use crate::events::{SshStateChangedEvent, emit_ssh_state_changed};
    use crate::session::output::{Coalescer, CoalescerConfig, CoalescerContext, run};
    use crate::session::ssh_task::SshTaskHandle;
    use crate::ssh::SshLifecycleState;
    use tokio::sync::mpsc;

    // Retrieve VT and frame-ack clock from the existing pane.
    let vt = registry.get_pane_vt(&pane_id).map_err(|e| e.to_string())?;
    let last_frame_ack_ms = registry
        .get_pane_frame_ack_clock(&pane_id)
        .ok_or_else(|| format!("pane not found: {pane_id}"))?;

    // Set SSH state on the pane.
    registry.set_ssh_state(&pane_id, SshLifecycleState::Connected);

    // Create the ProcessOutput channel (same capacity as real SSH pipeline).
    let (tx, rx) = mpsc::channel::<crate::session::output::ProcessOutput>(256);

    // Register sender so inject_ssh_output can reach it.
    ssh_registry.register(pane_id.clone(), tx);

    // Spawn coalescer task (Task B only — no reader task).
    let pane_id_e = pane_id.clone();
    let app_e = app.clone();
    let registry_arc = Arc::clone(&*registry);
    let registry_e = Arc::clone(&registry_arc);
    let ssh_registry_ref = Arc::clone(&*ssh_registry);

    let config = CoalescerConfig::ssh_default();
    let coalescer = Coalescer::new(&config);
    let ctx = CoalescerContext {
        app: app.clone(),
        pane_id: pane_id.clone(),
        vt,
        registry: registry_arc,
        last_frame_ack_ms,
        config,
    };

    let emit_task = tauri::async_runtime::spawn(async move {
        run(coalescer, ctx, rx).await;

        // Termination block — mirrors ssh_task.rs Task B.
        registry_e.set_ssh_state(&pane_id_e, SshLifecycleState::Closed);
        emit_ssh_state_changed(
            &app_e,
            SshStateChangedEvent {
                pane_id: pane_id_e.clone(),
                state: SshLifecycleState::Closed,
            },
        );

        // Clean up the injectable registry entry.
        ssh_registry_ref.remove(&pane_id_e);
    });

    // Store the task handle so it gets aborted on pane close.
    // We use a dummy read_abort (the emit_task itself) since there's no reader.
    let handle = SshTaskHandle::new(
        emit_task.inner().abort_handle(),
        emit_task.inner().abort_handle(),
    );
    registry.set_ssh_task(&pane_id, handle);

    // Emit Connected event so the frontend sees the SSH state.
    emit_ssh_state_changed(
        &app,
        SshStateChangedEvent {
            pane_id,
            state: SshLifecycleState::Connected,
        },
    );

    Ok(())
}
