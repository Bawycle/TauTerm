// SPDX-License-Identifier: MPL-2.0

//! Preferences Tauri commands.
//!
//! Commands: get_preferences, update_preferences, get_themes, save_theme, delete_theme.

use std::sync::Arc;

use parking_lot::RwLock;
use tauri::{AppHandle, State};

use crate::error::TauTermError;
use crate::preferences::{CursorStyle, Preferences, PreferencesPatch, PreferencesStore, UserTheme};
use crate::session::SessionRegistry;

#[tauri::command]
pub async fn get_preferences(
    prefs: State<'_, Arc<RwLock<PreferencesStore>>>,
) -> Result<Preferences, TauTermError> {
    Ok(prefs.read().get())
}

#[tauri::command]
pub async fn update_preferences(
    app: AppHandle,
    patch: PreferencesPatch,
    prefs: State<'_, Arc<RwLock<PreferencesStore>>>,
    registry: State<'_, Arc<SessionRegistry>>,
) -> Result<Preferences, TauTermError> {
    // Capture which live-propagatable fields were present in the patch before
    // consuming it (apply_patch takes ownership).
    let new_cursor_style: Option<CursorStyle> =
        patch.appearance.as_ref().and_then(|a| a.cursor_style);
    let new_osc52: Option<bool> = patch.terminal.as_ref().and_then(|t| t.allow_osc52_write);
    let new_scrollback: Option<usize> = patch.terminal.as_ref().and_then(|t| t.scrollback_lines);

    let updated = prefs
        .read()
        .apply_patch(patch)
        .map_err(TauTermError::from)?;

    // Propagate cursor shape to all existing panes so running sessions see the
    // change immediately. Applications can still override per-pane via DECSCUSR.
    if let Some(style) = new_cursor_style {
        registry.propagate_cursor_shape(&app, style.to_decscusr());
    }

    // Propagate OSC 52 write gate to all existing panes.
    if let Some(allow) = new_osc52 {
        registry.propagate_osc52_allow(allow);
    }

    // Scrollback capacity is fixed at pane-creation time (ScreenBuffer is not
    // dynamically resizable in v1). Log an informational message so developers
    // and advanced users are aware.
    // Note: log the effective (post-clamp) value from `updated`, not the raw
    // value from the patch — they may differ when clamping occurred.
    if new_scrollback.is_some() {
        let effective = updated.terminal.scrollback_lines;
        tracing::debug!(
            "scrollback_lines preference updated to {effective}; \
             applies to new panes — existing panes retain their current buffer size"
        );
    }

    Ok(updated)
}

#[tauri::command]
pub async fn get_themes(
    prefs: State<'_, Arc<RwLock<PreferencesStore>>>,
) -> Result<Vec<UserTheme>, TauTermError> {
    Ok(prefs.read().get_themes())
}

#[tauri::command]
pub async fn save_theme(
    theme: UserTheme,
    prefs: State<'_, Arc<RwLock<PreferencesStore>>>,
) -> Result<(), TauTermError> {
    prefs.read().save_theme(theme).map_err(TauTermError::from)
}

#[tauri::command]
pub async fn delete_theme(
    name: String,
    prefs: State<'_, Arc<RwLock<PreferencesStore>>>,
) -> Result<(), TauTermError> {
    prefs.read().delete_theme(&name).map_err(TauTermError::from)
}
