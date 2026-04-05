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

use std::sync::Arc;

use parking_lot::RwLock;

use tauri::Manager;

use crate::credentials::CredentialManager;
use crate::preferences::PreferencesStore;
use crate::session::SessionRegistry;
use crate::ssh::SshManager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Initialize tracing subscriber for logging.
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    // Load preferences from disk (fallback to defaults on parse/IO errors).
    // `PreferencesStore::load()` only fails if the config path cannot be determined
    // (e.g., $HOME unset). This is a programming error / broken system — we panic.
    let prefs: Arc<RwLock<PreferencesStore>> =
        PreferencesStore::load().expect("Failed to determine preferences path — is $HOME set?");

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

            let registry = SessionRegistry::new(
                pty_backend,
                app.handle().clone(),
                #[cfg(feature = "e2e-testing")]
                injectable_registry,
            );
            app.manage(registry);
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
            commands::session_cmds::reorder_tab,
            commands::session_cmds::split_pane,
            commands::session_cmds::close_pane,
            commands::session_cmds::set_active_pane,
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
            commands::ssh_prompt_cmds::accept_host_key,
            commands::ssh_prompt_cmds::reject_host_key,
            commands::ssh_prompt_cmds::dismiss_ssh_algorithm_warning,
            // Connection config commands
            commands::connection_cmds::get_connections,
            commands::connection_cmds::save_connection,
            commands::connection_cmds::delete_connection,
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
            // E2E testing commands (compiled only with --features e2e-testing)
            #[cfg(feature = "e2e-testing")]
            commands::testing::inject_pty_output,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
