// SPDX-License-Identifier: MPL-2.0

//! TauTerm backend — Tauri application setup.
//!
//! Registers all modules, injects managed state, and wires command handlers.
//! The architecture is documented in `docs/ARCHITECTURE.md`.

pub mod commands;
pub mod credentials;
pub mod error;
pub mod events;
pub mod ipc;
pub mod platform;
pub mod preferences;
pub mod security_load;
pub mod security_static_checks;
pub mod session;
pub mod ssh;
pub mod vt;
pub mod webview_data_dir;

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
        .with_writer(std::io::stderr)
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

    let specta_builder = ipc::make_builder();

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(ssh_manager)
        .manage(prefs)
        .manage(credential_manager)
        .invoke_handler(specta_builder.invoke_handler())
        .setup(move |app| {
            specta_builder.mount_events(app);
            // Create the main window manually (tauri.conf.json has "create": false).
            // On Linux, inject an instance-unique data directory to prevent WebKitGTK
            // cache conflicts between concurrent instances (ADR-0025).
            let window_config = app
                .config()
                .app
                .windows
                .first()
                .ok_or_else(|| anyhow::anyhow!("No window config in tauri.conf.json"))?;
            let mut builder = tauri::WebviewWindowBuilder::from_config(app, window_config)?;

            #[cfg(target_os = "linux")]
            {
                builder =
                    builder.data_directory(crate::webview_data_dir::resolve_webview_data_dir());
            }

            builder.build()?;

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

            #[cfg(feature = "e2e-testing")]
            app.manage(Arc::new(
                crate::session::ssh_injectable::SshInjectableRegistry::new(),
            ));

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
                source_pane_id: None,
            }) {
                tracing::error!("Failed to create initial tab during setup: {e}");
            }

            app.manage(registry);

            // Start the preferences file watcher for cross-instance sync.
            // Non-fatal: if the watcher fails to start, the app runs without live-sync.
            {
                let prefs_for_watcher = app
                    .state::<Arc<RwLock<crate::preferences::PreferencesStore>>>()
                    .inner()
                    .clone();
                let write_epoch = prefs_for_watcher.read().write_epoch.clone();
                let prefs_path = prefs_for_watcher.read().path().to_owned();
                match crate::preferences::watcher::start(
                    prefs_for_watcher,
                    app.handle().clone(),
                    write_epoch,
                    prefs_path,
                ) {
                    Ok(watcher) => {
                        app.manage(watcher);
                    }
                    Err(e) => {
                        tracing::warn!("Failed to start preferences watcher: {e}");
                    }
                }
            }

            // FS-FULL-009: restore full-screen state from saved preferences.
            // Skipped in E2E builds so tests always start in windowed mode.
            #[cfg(not(feature = "e2e-testing"))]
            {
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
            }

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
