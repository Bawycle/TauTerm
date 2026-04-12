// SPDX-License-Identifier: MPL-2.0

//! WebKitGTK data directory resolution.
//!
//! Resolves an instance-unique data directory path so that multiple concurrent
//! TauTerm instances do not share a WebKitGTK cache directory.  The conflict
//! manifests as hard-link failures in WebKit's HTTP cache.
//!
//! ## Resolution order
//!
//! 1. `TAUTERM_DATA_DIR` env var — an absolute path supplied by the caller or
//!    a test harness.
//! 2. Automatic PID-based path: `$XDG_DATA_HOME/tau-term/webview/<pid>/`
//!    (or `$HOME/.local/share/tau-term/webview/<pid>/`).
//! 3. Fallback: `/tmp/tau-term-webview-<pid>/` when `$HOME` / `XDG_DATA_HOME`
//!    cannot be determined.
//!
//! ## Stale directory cleanup
//!
//! On every launch, the resolver sweeps the `webview/` parent for numeric
//! subdirectories whose PID is no longer live (checked via `/proc/<pid>/`
//! existence on Linux).  Stale directories are removed in the background.

use std::path::PathBuf;

/// Resolve a unique WebKitGTK data directory for this process.
pub fn resolve_webview_data_dir() -> PathBuf {
    // 1. Explicit override via environment variable.
    if let Some(val) = std::env::var_os("TAUTERM_DATA_DIR") {
        let path = PathBuf::from(&val);
        if path.is_absolute() {
            tracing::debug!("Using TAUTERM_DATA_DIR override for webview data directory");
            return path;
        }
        tracing::warn!("TAUTERM_DATA_DIR is not an absolute path, ignoring");
    }

    // 2. Auto PID-based path.
    let pid = std::process::id();
    if let Some(base) = xdg_data_home() {
        let webview_base = base.join("tau-term").join("webview");
        spawn_stale_cleanup(webview_base.clone());
        return webview_base.join(pid.to_string());
    }

    // 3. Fallback when $HOME / XDG_DATA_HOME are unset.
    tracing::warn!("Could not determine data home; using /tmp fallback for webview data directory");
    PathBuf::from(format!("/tmp/tau-term-webview-{pid}"))
}

/// Return `$XDG_DATA_HOME` or `$HOME/.local/share`.
fn xdg_data_home() -> Option<PathBuf> {
    if let Some(dir) = std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .filter(|d| d.is_absolute())
    {
        return Some(dir);
    }
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .filter(|p| p.is_absolute())
        .map(|h| h.join(".local").join("share"))
}

/// Spawn a background thread to remove stale PID-based data directories.
fn spawn_stale_cleanup(base: PathBuf) {
    std::thread::Builder::new()
        .name("webview-cleanup".into())
        .spawn(move || cleanup_stale_dirs(&base))
        .ok();
}

/// Remove subdirectories of `base` whose name is a numeric PID that is no
/// longer running.  Non-numeric entries are left untouched.
#[cfg(target_os = "linux")]
fn cleanup_stale_dirs(base: &std::path::Path) {
    let entries = match std::fs::read_dir(base) {
        Ok(e) => e,
        Err(_) => return,
    };
    let current_pid = std::process::id();
    for entry in entries.flatten() {
        let name = entry.file_name();
        let Some(name_str) = name.to_str() else {
            continue;
        };
        let Ok(pid) = name_str.parse::<u32>() else {
            continue; // non-numeric — leave it
        };
        if pid == current_pid {
            continue; // our own directory
        }
        // Check if the process is still alive via /proc.
        if std::path::Path::new(&format!("/proc/{pid}")).exists() {
            continue; // still running
        }
        if let Err(e) = std::fs::remove_dir_all(entry.path()) {
            tracing::debug!("Failed to remove stale webview directory for PID {pid}: {e}");
        } else {
            tracing::debug!("Removed stale webview directory for PID {pid}");
        }
    }
}

#[cfg(not(target_os = "linux"))]
fn cleanup_stale_dirs(_base: &std::path::Path) {
    // Stale cleanup requires /proc — deferred to Windows porting phase.
}

#[cfg(test)]
mod tests {
    use super::*;

    // SAFETY: env var mutations are safe under nextest's process-per-test isolation.

    #[test]
    fn env_var_absolute_is_used() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().to_path_buf();
        unsafe { std::env::set_var("TAUTERM_DATA_DIR", &path) };
        let result = resolve_webview_data_dir();
        unsafe { std::env::remove_var("TAUTERM_DATA_DIR") };
        assert_eq!(result, path);
    }

    #[test]
    fn env_var_relative_is_ignored() {
        unsafe { std::env::set_var("TAUTERM_DATA_DIR", "relative/path") };
        let result = resolve_webview_data_dir();
        unsafe { std::env::remove_var("TAUTERM_DATA_DIR") };
        let pid_str = std::process::id().to_string();
        assert!(
            result.to_str().unwrap().contains(&pid_str),
            "expected PID in path, got: {result:?}"
        );
    }

    #[test]
    fn env_var_unset_returns_pid_path() {
        unsafe { std::env::remove_var("TAUTERM_DATA_DIR") };
        let result = resolve_webview_data_dir();
        let pid_str = std::process::id().to_string();
        assert!(
            result.to_str().unwrap().contains(&pid_str),
            "expected PID in path, got: {result:?}"
        );
    }

    #[test]
    #[cfg(target_os = "linux")]
    fn stale_cleanup_removes_dead_pid() {
        let dir = tempfile::tempdir().expect("tempdir");
        // Use a PID that almost certainly doesn't exist.
        let fake_pid = "999999999";
        std::fs::create_dir_all(dir.path().join(fake_pid)).expect("create fake dir");
        cleanup_stale_dirs(dir.path());
        assert!(
            !dir.path().join(fake_pid).exists(),
            "stale dir should have been removed"
        );
    }

    #[test]
    #[cfg(target_os = "linux")]
    fn stale_cleanup_preserves_live_pid() {
        let dir = tempfile::tempdir().expect("tempdir");
        let my_pid = std::process::id().to_string();
        std::fs::create_dir_all(dir.path().join(&my_pid)).expect("create own dir");
        cleanup_stale_dirs(dir.path());
        assert!(
            dir.path().join(&my_pid).exists(),
            "own dir should be preserved"
        );
    }

    #[test]
    #[cfg(target_os = "linux")]
    fn stale_cleanup_preserves_non_numeric() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::create_dir_all(dir.path().join("default")).expect("create default dir");
        cleanup_stale_dirs(dir.path());
        assert!(
            dir.path().join("default").exists(),
            "non-numeric dir should be preserved"
        );
    }
}
