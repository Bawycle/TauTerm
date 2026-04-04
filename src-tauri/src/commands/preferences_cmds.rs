// SPDX-License-Identifier: MPL-2.0

//! Preferences Tauri commands.
//!
//! Commands: get_preferences, update_preferences, get_themes, save_theme, delete_theme.

use std::sync::Arc;

use parking_lot::RwLock;
use tauri::State;

use crate::error::TauTermError;
use crate::preferences::{Preferences, PreferencesPatch, PreferencesStore, UserTheme};

#[tauri::command]
pub async fn get_preferences(
    prefs: State<'_, Arc<RwLock<PreferencesStore>>>,
) -> Result<Preferences, TauTermError> {
    Ok(prefs.read().get())
}

#[tauri::command]
pub async fn update_preferences(
    patch: PreferencesPatch,
    prefs: State<'_, Arc<RwLock<PreferencesStore>>>,
) -> Result<Preferences, TauTermError> {
    prefs.read().apply_patch(patch).map_err(TauTermError::from)
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
