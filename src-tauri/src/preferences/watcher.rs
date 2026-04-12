// SPDX-License-Identifier: MPL-2.0

//! Filesystem watcher for `preferences.toml`.
//!
//! Detects external modifications (e.g. by another TauTerm instance) and
//! reloads preferences into the in-memory store, emitting a
//! `preferences-changed` event so the frontend stays in sync.
//!
//! Design notes:
//! - Watches the **parent directory** (not the file itself) to survive
//!   atomic rename — inotify does not follow renamed inodes.
//! - Uses `write_epoch` (monotonic counter) to distinguish own writes from
//!   external writes. The store increments the epoch before every
//!   `save_to_disk` call; the watcher skips events whose epoch has changed
//!   since the last check.
//! - A 100 ms debounce coalesces rapid filesystem events (e.g. tmp-write +
//!   rename from atomic save).

use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

use notify::{EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use parking_lot::RwLock;
use tauri::Emitter;

use crate::events::{EVENT_PREFERENCES_CHANGED, PreferencesChangedEvent};
use crate::preferences::store::PreferencesStore;

/// Debounce window — filesystem events within this window after the first are
/// coalesced into a single reload.
const DEBOUNCE: Duration = Duration::from_millis(100);

/// Holds the `RecommendedWatcher`. When this value is dropped the watcher
/// stops and the background thread exits (channel sender closes → `recv`
/// returns `Err`).
pub struct PreferencesWatcher {
    _watcher: RecommendedWatcher,
}

/// Start watching for external changes to the preferences file.
///
/// Spawns a background `std::thread` that receives `notify` events and
/// reloads the store when an external write is detected.
///
/// # Errors
///
/// Returns a `notify::Error` if the watcher cannot be created or the
/// directory cannot be watched.
pub fn start(
    prefs_store: Arc<RwLock<PreferencesStore>>,
    app: tauri::AppHandle,
    write_epoch: Arc<AtomicU64>,
    watch_path: PathBuf,
) -> Result<PreferencesWatcher, notify::Error> {
    let (tx, rx) = std::sync::mpsc::channel();

    let mut watcher = RecommendedWatcher::new(tx, notify::Config::default())?;

    // Watch the parent directory — the file itself is replaced by atomic rename.
    let parent = watch_path
        .parent()
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| PathBuf::from("."));
    watcher.watch(&parent, RecursiveMode::NonRecursive)?;

    // Extract the filename we're interested in, for event filtering.
    let target_filename = watch_path
        .file_name()
        .map(|n| n.to_os_string())
        .unwrap_or_default();

    std::thread::Builder::new()
        .name("prefs-watcher".into())
        .spawn(move || {
            let mut last_seen_epoch = write_epoch.load(Ordering::SeqCst);
            let mut last_reload = Instant::now() - DEBOUNCE; // allow immediate first reload
            let mut pending_reload = false; // trailing-edge debounce flag
            // Timestamp of the last detected own-write.  Used to suppress
            // follow-up inotify events from the same atomic rename (e.g.
            // `Modify(Name(Both))` arriving after `Modify(Name(To))`).
            let mut last_own_write = Instant::now() - DEBOUNCE;

            loop {
                // Use `recv_timeout` so that a pending trailing reload is
                // processed after the debounce window even if no new event
                // arrives.
                let timeout = if pending_reload {
                    DEBOUNCE.saturating_sub(last_reload.elapsed())
                } else {
                    // Block indefinitely until the next event.
                    Duration::from_secs(3600)
                };

                let event = match rx.recv_timeout(timeout) {
                    Ok(Ok(event)) => Some(event),
                    Ok(Err(e)) => {
                        tracing::warn!("Preferences watcher error: {e}");
                        continue;
                    }
                    Err(std::sync::mpsc::RecvTimeoutError::Timeout) => None,
                    Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => break,
                };

                if let Some(event) = event {
                    tracing::debug!(
                        kind = ?event.kind,
                        paths = ?event.paths.iter().map(|p| p.file_name()).collect::<Vec<_>>(),
                        "Preferences watcher received event"
                    );

                    // Filter: only care about creates, modifications, and
                    // renames that affect our target file.  Remove events are
                    // excluded — a deleted file would reload as defaults and
                    // silently replace the in-memory state.
                    //
                    // Atomic rename (`tmp → preferences.toml`) produces
                    // `Modify(Name(To))` on inotify — this is a `Modify`
                    // variant and matches the filter.
                    let dominated =
                        matches!(event.kind, EventKind::Create(_) | EventKind::Modify(_));
                    if !dominated {
                        tracing::debug!(kind = ?event.kind, "Filtered out (not Create/Modify)");
                        continue;
                    }

                    let affects_target = event
                        .paths
                        .iter()
                        .any(|p| p.file_name().is_some_and(|name| name == target_filename));
                    if !affects_target {
                        continue;
                    }

                    // Check epoch: if changed, this is our own write — skip.
                    // Also suppress events arriving shortly after an epoch
                    // change — atomic rename generates multiple inotify events
                    // (Name(To) + Name(Both)) for a single write operation.
                    let current_epoch = write_epoch.load(Ordering::SeqCst);
                    if current_epoch != last_seen_epoch {
                        tracing::debug!(
                            current_epoch,
                            last_seen_epoch,
                            "Skipping own write (epoch changed)"
                        );
                        last_seen_epoch = current_epoch;
                        last_own_write = Instant::now();
                        continue;
                    }
                    if last_own_write.elapsed() < DEBOUNCE {
                        tracing::debug!("Skipping follow-up event from own atomic rename");
                        continue;
                    }

                    // Mark a pending reload.  If within the debounce window,
                    // the actual reload is deferred until the window expires
                    // (trailing-edge debounce), so no external write is lost.
                    pending_reload = true;

                    if last_reload.elapsed() < DEBOUNCE {
                        continue;
                    }
                }

                // If no pending reload, nothing to do (timeout with no work).
                if !pending_reload {
                    continue;
                }

                // External write detected — reload.
                pending_reload = false;
                let store = prefs_store.read();
                match store.reload_from_disk() {
                    Ok(prefs) => {
                        tracing::info!("Preferences reloaded from external change");
                        let event = PreferencesChangedEvent { preferences: prefs };
                        if let Err(e) = app.emit(EVENT_PREFERENCES_CHANGED, event) {
                            tracing::error!("Failed to emit preferences-changed: {e}");
                        }
                    }
                    Err(e) => {
                        tracing::warn!("Failed to reload preferences after external change: {e}");
                    }
                }
                last_reload = Instant::now();
            }

            tracing::debug!("Preferences watcher thread exiting");
        })
        .map_err(|e| notify::Error::generic(&e.to_string()))?;

    Ok(PreferencesWatcher { _watcher: watcher })
}
