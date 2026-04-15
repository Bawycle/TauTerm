// SPDX-License-Identifier: MPL-2.0

//! Tauri-specta builder configuration.
//!
//! Constructs the [`tauri_specta::Builder`] with all registered commands and
//! events. Used by `lib.rs` (future PR A3) and by the `export_bindings`
//! integration test to generate TypeScript bindings.

use tauri_specta::{collect_commands, collect_events};

use crate::commands::{
    connection_cmds, input_cmds, preferences_cmds, session_cmds, ssh_cmds, ssh_prompt_cmds,
    system_cmds,
};
use crate::events::types::{
    BellTriggeredEvent, CredentialPromptEvent, CursorStyleChangedEvent,
    FullscreenStateChangedEvent, HostKeyPromptEvent, ModeStateChangedEvent,
    NotificationChangedEvent, Osc52WriteRequestedEvent, PassphrasePromptEvent,
    PreferencesChangedEvent, ScreenUpdateEvent, ScrollPositionChangedEvent,
    SessionStateChangedEvent, SshReconnectedEvent, SshStateChangedEvent, SshWarningEvent,
};

/// Build the tauri-specta builder with all commands and events.
///
/// The builder is consumed in two places:
/// - The `export_bindings` integration test (generates `src/lib/ipc/bindings.ts`)
/// - `lib.rs::run()` (future PR A3 — replaces `generate_handler![]`)
///
/// E2E testing commands are included only when the `e2e-testing` feature is
/// active. The `collect_commands!` macro does not support `#[cfg]` on
/// individual entries, so we use two separate function bodies.
#[cfg(not(feature = "e2e-testing"))]
pub fn make_builder() -> tauri_specta::Builder {
    tauri_specta::Builder::<tauri::Wry>::new()
        .commands(collect_commands![
            // Session commands
            session_cmds::create_tab,
            session_cmds::close_tab,
            session_cmds::rename_tab,
            session_cmds::set_pane_label,
            session_cmds::reorder_tab,
            session_cmds::split_pane,
            session_cmds::close_pane,
            session_cmds::set_active_tab,
            session_cmds::set_active_pane,
            session_cmds::has_foreground_process,
            // Input / screen commands
            input_cmds::send_input,
            input_cmds::scroll_pane,
            input_cmds::scroll_to_bottom,
            input_cmds::search_pane,
            input_cmds::get_pane_screen_snapshot,
            input_cmds::resize_pane,
            input_cmds::frame_ack,
            // SSH session commands
            ssh_cmds::open_ssh_connection,
            ssh_cmds::close_ssh_connection,
            ssh_cmds::reconnect_ssh,
            // SSH prompt commands
            ssh_prompt_cmds::provide_credentials,
            ssh_prompt_cmds::provide_passphrase,
            ssh_prompt_cmds::accept_host_key,
            ssh_prompt_cmds::reject_host_key,
            ssh_prompt_cmds::dismiss_ssh_algorithm_warning,
            // Connection config commands
            connection_cmds::get_connections,
            connection_cmds::save_connection,
            connection_cmds::delete_connection,
            connection_cmds::duplicate_connection,
            connection_cmds::store_connection_password,
            // Preferences commands
            preferences_cmds::get_preferences,
            preferences_cmds::update_preferences,
            preferences_cmds::get_themes,
            preferences_cmds::save_theme,
            preferences_cmds::delete_theme,
            // System commands
            system_cmds::get_session_state,
            system_cmds::copy_to_clipboard,
            system_cmds::get_clipboard,
            system_cmds::open_url,
            system_cmds::mark_context_menu_used,
            system_cmds::toggle_fullscreen,
        ])
        .events(collect_events![
            SessionStateChangedEvent,
            SshStateChangedEvent,
            ScreenUpdateEvent,
            ModeStateChangedEvent,
            ScrollPositionChangedEvent,
            CredentialPromptEvent,
            PassphrasePromptEvent,
            HostKeyPromptEvent,
            CursorStyleChangedEvent,
            BellTriggeredEvent,
            Osc52WriteRequestedEvent,
            SshWarningEvent,
            SshReconnectedEvent,
            FullscreenStateChangedEvent,
            NotificationChangedEvent,
            PreferencesChangedEvent,
        ])
}

/// E2E testing variant — includes the 7 testing commands in addition to all
/// production commands. Generated TypeScript bindings will contain wrappers
/// for the testing commands; this is an accepted tradeoff (dead code in prod
/// TS bundle, but the Rust handlers are cfg-gated).
#[cfg(feature = "e2e-testing")]
pub fn make_builder() -> tauri_specta::Builder {
    use crate::commands::testing;

    tauri_specta::Builder::<tauri::Wry>::new()
        .commands(collect_commands![
            // Session commands
            session_cmds::create_tab,
            session_cmds::close_tab,
            session_cmds::rename_tab,
            session_cmds::set_pane_label,
            session_cmds::reorder_tab,
            session_cmds::split_pane,
            session_cmds::close_pane,
            session_cmds::set_active_tab,
            session_cmds::set_active_pane,
            session_cmds::has_foreground_process,
            // Input / screen commands
            input_cmds::send_input,
            input_cmds::scroll_pane,
            input_cmds::scroll_to_bottom,
            input_cmds::search_pane,
            input_cmds::get_pane_screen_snapshot,
            input_cmds::resize_pane,
            input_cmds::frame_ack,
            // SSH session commands
            ssh_cmds::open_ssh_connection,
            ssh_cmds::close_ssh_connection,
            ssh_cmds::reconnect_ssh,
            // SSH prompt commands
            ssh_prompt_cmds::provide_credentials,
            ssh_prompt_cmds::provide_passphrase,
            ssh_prompt_cmds::accept_host_key,
            ssh_prompt_cmds::reject_host_key,
            ssh_prompt_cmds::dismiss_ssh_algorithm_warning,
            // Connection config commands
            connection_cmds::get_connections,
            connection_cmds::save_connection,
            connection_cmds::delete_connection,
            connection_cmds::duplicate_connection,
            connection_cmds::store_connection_password,
            // Preferences commands
            preferences_cmds::get_preferences,
            preferences_cmds::update_preferences,
            preferences_cmds::get_themes,
            preferences_cmds::save_theme,
            preferences_cmds::delete_theme,
            // System commands
            system_cmds::get_session_state,
            system_cmds::copy_to_clipboard,
            system_cmds::get_clipboard,
            system_cmds::open_url,
            system_cmds::mark_context_menu_used,
            system_cmds::toggle_fullscreen,
            // E2E testing commands
            testing::inject_pty_output,
            testing::inject_ssh_failure,
            testing::inject_ssh_delay,
            testing::inject_ssh_disconnect,
            testing::inject_credential_prompt,
            testing::inject_pane_exit,
            testing::inject_foreground_process,
        ])
        .events(collect_events![
            SessionStateChangedEvent,
            SshStateChangedEvent,
            ScreenUpdateEvent,
            ModeStateChangedEvent,
            ScrollPositionChangedEvent,
            CredentialPromptEvent,
            PassphrasePromptEvent,
            HostKeyPromptEvent,
            CursorStyleChangedEvent,
            BellTriggeredEvent,
            Osc52WriteRequestedEvent,
            SshWarningEvent,
            SshReconnectedEvent,
            FullscreenStateChangedEvent,
            NotificationChangedEvent,
            PreferencesChangedEvent,
        ])
}
