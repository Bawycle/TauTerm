// SPDX-License-Identifier: MPL-2.0

use std::path::PathBuf;

use crate::error::PreferencesError;

/// Determine the preferences file path (TOML format).
pub(super) fn preferences_path() -> Result<PathBuf, PreferencesError> {
    let config_dir = dirs_or_home()?;
    Ok(config_dir.join("tauterm").join("preferences.toml"))
}

/// Get the XDG config directory, falling back to `~/.config`.
pub(super) fn dirs_or_home() -> Result<PathBuf, PreferencesError> {
    // Try XDG_CONFIG_HOME first, then fall back to $HOME/.config.
    if let Some(dir) = std::env::var_os("XDG_CONFIG_HOME").map(PathBuf::from) {
        return Ok(dir);
    }
    let home = std::env::var_os("HOME")
        .map(PathBuf::from)
        .ok_or_else(|| PreferencesError::Io(std::io::Error::other("HOME not set")))?;
    Ok(home.join(".config"))
}
