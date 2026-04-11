// SPDX-License-Identifier: MPL-2.0

//! TauTerm backend — Tauri application setup.
//!
//! Registers all modules, injects managed state, and wires command handlers.
//! The architecture is documented in `docs/ARCHITECTURE.md`.

pub mod commands;
pub mod credentials;
pub mod error;
pub mod events;
pub mod platform;
pub mod preferences;
pub mod security_load;
pub mod security_static_checks;
pub mod session;
pub mod ssh;
pub mod vt;

// These imports are only used by `run()`, which is gated behind
// `not(feature = "fuzz-testing")`.  Wrap them in the same cfg guard
// so the fuzz workspace compiles without unused-import warnings.
#[cfg(not(feature = "fuzz-testing"))]
use std::sync::Arc;

#[cfg(not(feature = "fuzz-testing"))]
use parking_lot::RwLock;

#[cfg(not(feature = "fuzz-testing"))]
use tauri::Manager;

#[cfg(not(feature = "fuzz-testing"))]
use crate::credentials::CredentialManager;
#[cfg(not(feature = "fuzz-testing"))]
use crate::preferences::PreferencesStore;
#[cfg(not(feature = "fuzz-testing"))]
use crate::session::{CreateTabConfig, SessionRegistry};
#[cfg(not(feature = "fuzz-testing"))]
use crate::ssh::SshManager;

// The `run()` function calls `tauri::generate_context!()`, which expands to
// code that is incompatible with nightly rustc (field layout changes in Tauri
// internals).  Gate it behind `not(fuzz-testing)` so the fuzz workspace can
// compile the crate with nightly without touching any Tauri proc-macro output.
#[cfg(not(feature = "fuzz-testing"))]
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Initialize tracing subscriber for logging.
    // Default filter: in dev builds, DEBUG+ on the TauTerm crate and WARN on
    // all dependencies; in release builds, WARN+ only.  `RUST_LOG` always
    // takes precedence (try_from_default_env succeeds when RUST_LOG is set).
    let default_filter = if cfg!(debug_assertions) {
        "tau_term_lib=debug,warn"
    } else {
        "warn"
    };
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new(default_filter)),
        )
        .init();

    // Load preferences from disk, falling back to defaults on any error (§7.5).
    // `load_or_default` never panics: if the config path cannot be determined
    // (e.g., $HOME unset) it logs a warning and returns an in-memory default store.
    let prefs: Arc<RwLock<PreferencesStore>> = PreferencesStore::load_or_default();

    let ssh_manager = SshManager::new();
    let credential_manager = Arc::new(CredentialManager::new());

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(ssh_manager)
        .manage(prefs)
        .manage(credential_manager)
        .setup(|app| {
            // `SessionRegistry` needs an `AppHandle` to emit events from PTY read tasks.
            // We create it inside `setup` where the `AppHandle` is available.

            // In e2e-testing builds: use the injectable backend and manage the registry
            // as Tauri state so `inject_pty_output` can push bytes into pane channels.
            // In production builds: use the real platform PTY backend.
            #[cfg(not(feature = "e2e-testing"))]
            let pty_backend: Arc<dyn platform::PtyBackend> =
                Arc::from(platform::create_pty_backend());

            #[cfg(feature = "e2e-testing")]
            let injectable_registry = Arc::new(platform::pty_injectable::InjectableRegistry::new());

            #[cfg(feature = "e2e-testing")]
            let pty_backend: Arc<dyn platform::PtyBackend> = Arc::new(
                platform::create_injectable_pty_backend(injectable_registry.clone()),
            );

            #[cfg(feature = "e2e-testing")]
            app.manage(injectable_registry.clone());

            #[cfg(feature = "e2e-testing")]
            app.manage(Arc::new(crate::commands::testing::SshFailureRegistry::new()));

            let prefs_for_registry = app
                .state::<Arc<RwLock<crate::preferences::PreferencesStore>>>()
                .inner()
                .clone();

            let registry = SessionRegistry::new(
                pty_backend,
                app.handle().clone(),
                prefs_for_registry,
                #[cfg(feature = "e2e-testing")]
                injectable_registry,
            );

            // Create the initial tab before registering the registry as Tauri state.
            // This guarantees that `get_session_state` always returns ≥1 tab on first
            // call, so the frontend never needs to call `create_tab` on startup.
            // Uses a login shell (FS-PTY-013) so ~/.bash_profile / ~/.zprofile are sourced.
            // Non-fatal: if PTY is unavailable, log and continue — the frontend will handle
            // an empty state gracefully via `session-state-changed` events.
            if let Err(e) = registry.create_tab(CreateTabConfig {
                label: None,
                cols: 80,
                rows: 24,
                pixel_width: 0,
                pixel_height: 0,
                shell: None,
                login: true,
            }) {
                tracing::error!("Failed to create initial tab during setup: {e}");
            }

            app.manage(registry);

            // FS-FULL-009: restore full-screen state from saved preferences.
            let saved_fullscreen = app
                .state::<Arc<RwLock<crate::preferences::PreferencesStore>>>()
                .read()
                .get()
                .appearance
                .fullscreen;
            if saved_fullscreen
                && let Some(window) = app.get_webview_window("main")
                && let Err(e) = window.set_fullscreen(true)
            {
                tracing::warn!("Could not restore fullscreen state: {e}");
            }

            Ok(())
        })
        // E2E testing commands — only compiled and registered when e2e-testing feature is active.
        // `generate_handler![]` supports `#[cfg]` on individual entries in Tauri 2.
        // If that ever stops working, use the duplicate-handler approach from
        // ADR-0015-implementation-notes.md §7.2.
        .invoke_handler(tauri::generate_handler![
            // Session commands
            commands::session_cmds::create_tab,
            commands::session_cmds::close_tab,
            commands::session_cmds::rename_tab,
            commands::session_cmds::set_pane_label,
            commands::session_cmds::reorder_tab,
            commands::session_cmds::split_pane,
            commands::session_cmds::close_pane,
            commands::session_cmds::set_active_tab,
            commands::session_cmds::set_active_pane,
            commands::session_cmds::has_foreground_process,
            // Input / screen commands
            commands::input_cmds::send_input,
            commands::input_cmds::scroll_pane,
            commands::input_cmds::scroll_to_bottom,
            commands::input_cmds::search_pane,
            commands::input_cmds::get_pane_screen_snapshot,
            commands::input_cmds::resize_pane,
            // SSH session commands
            commands::ssh_cmds::open_ssh_connection,
            commands::ssh_cmds::close_ssh_connection,
            commands::ssh_cmds::reconnect_ssh,
            // SSH prompt commands
            commands::ssh_prompt_cmds::provide_credentials,
            commands::ssh_prompt_cmds::provide_passphrase,
            commands::ssh_prompt_cmds::accept_host_key,
            commands::ssh_prompt_cmds::reject_host_key,
            commands::ssh_prompt_cmds::dismiss_ssh_algorithm_warning,
            // Connection config commands
            commands::connection_cmds::get_connections,
            commands::connection_cmds::save_connection,
            commands::connection_cmds::delete_connection,
            commands::connection_cmds::duplicate_connection,
            commands::connection_cmds::store_connection_password,
            // Preferences commands
            commands::preferences_cmds::get_preferences,
            commands::preferences_cmds::update_preferences,
            commands::preferences_cmds::get_themes,
            commands::preferences_cmds::save_theme,
            commands::preferences_cmds::delete_theme,
            // System commands
            commands::system_cmds::get_session_state,
            commands::system_cmds::copy_to_clipboard,
            commands::system_cmds::get_clipboard,
            commands::system_cmds::open_url,
            commands::system_cmds::mark_context_menu_used,
            commands::system_cmds::toggle_fullscreen,
            // E2E testing commands (compiled only with --features e2e-testing)
            #[cfg(feature = "e2e-testing")]
            commands::testing::inject_pty_output,
            #[cfg(feature = "e2e-testing")]
            commands::testing::inject_ssh_failure,
            #[cfg(feature = "e2e-testing")]
            commands::testing::inject_ssh_delay,
            #[cfg(feature = "e2e-testing")]
            commands::testing::inject_ssh_disconnect,
            #[cfg(feature = "e2e-testing")]
            commands::testing::inject_credential_prompt,
            #[cfg(feature = "e2e-testing")]
            commands::testing::inject_pane_exit,
            #[cfg(feature = "e2e-testing")]
            commands::testing::inject_foreground_process,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
