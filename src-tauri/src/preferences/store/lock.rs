// SPDX-License-Identifier: MPL-2.0

//! Advisory file locking for preferences writes.
//!
//! Uses `fs4` (pure Rust: `flock` on Unix, `LockFileEx` on Windows) to prevent concurrent
//! TauTerm instances from corrupting `preferences.toml` during the
//! write-tmp-then-rename sequence.
//!
//! The lock file (`preferences.toml.lock`) is a separate, stable file — it is
//! never renamed or deleted, so `flock` works correctly even across the atomic
//! rename of the data file.

use std::fs::{File, OpenOptions};
use std::path::Path;
use std::thread;
use std::time::Duration;

use crate::error::PreferencesError;

/// RAII guard that holds an advisory file lock. The lock is released on drop.
#[derive(Debug)]
pub(super) struct PrefsLockGuard {
    file: File,
}

impl Drop for PrefsLockGuard {
    fn drop(&mut self) {
        let _ = fs4::fs_std::FileExt::unlock(&self.file);
    }
}

/// Retry interval between non-blocking lock attempts.
const RETRY_INTERVAL: Duration = Duration::from_millis(5);

/// Maximum number of retries before giving up.
/// 200 × 5 ms = 1 second total wait.
const MAX_RETRIES: u32 = 200;

/// Acquire an exclusive (write) lock on the lock file at `lock_path`.
///
/// Retries with a short sleep if the lock is held by another process.
/// Returns `PreferencesError::LockTimeout` after ~1 second of contention.
pub(super) fn acquire_exclusive(lock_path: &Path) -> Result<PrefsLockGuard, PreferencesError> {
    let file = open_lock_file(lock_path)?;
    for _ in 0..MAX_RETRIES {
        match fs4::fs_std::FileExt::try_lock_exclusive(&file) {
            Ok(true) => return Ok(PrefsLockGuard { file }),
            Ok(false) => thread::sleep(RETRY_INTERVAL),
            Err(e) => return Err(PreferencesError::Io(e)),
        }
    }
    Err(PreferencesError::LockTimeout)
}

/// Acquire a shared (read) lock on the lock file at `lock_path`.
///
/// Multiple readers can hold the lock concurrently. A shared lock blocks
/// only if an exclusive lock is held (and vice versa).
pub(super) fn acquire_shared(lock_path: &Path) -> Result<PrefsLockGuard, PreferencesError> {
    let file = open_lock_file(lock_path)?;
    for _ in 0..MAX_RETRIES {
        match fs4::fs_std::FileExt::try_lock_shared(&file) {
            Ok(true) => return Ok(PrefsLockGuard { file }),
            Ok(false) => thread::sleep(RETRY_INTERVAL),
            Err(e) => return Err(PreferencesError::Io(e)),
        }
    }
    Err(PreferencesError::LockTimeout)
}

/// Open (or create) the lock file. Parent directories are created if needed.
fn open_lock_file(path: &Path) -> Result<File, PreferencesError> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(path)
        .map_err(PreferencesError::Io)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Barrier};

    /// Lock can be acquired, dropped, then acquired again immediately.
    #[test]
    fn lock_acquired_and_released() {
        let dir = tempfile::tempdir().expect("tempdir");
        let lock_path = dir.path().join("test.lock");

        let guard = acquire_exclusive(&lock_path).expect("first acquire");
        drop(guard);
        let _guard2 = acquire_exclusive(&lock_path).expect("second acquire after release");
    }

    /// A second exclusive lock attempt succeeds once the first holder releases.
    #[test]
    fn exclusive_blocks_second_exclusive() {
        let dir = tempfile::tempdir().expect("tempdir");
        let lock_path = dir.path().join("test.lock");

        let barrier = Arc::new(Barrier::new(2));
        let lock_path_clone = lock_path.clone();
        let barrier_clone = barrier.clone();

        let handle = std::thread::spawn(move || {
            let _guard = acquire_exclusive(&lock_path_clone).expect("thread acquire");
            barrier_clone.wait(); // signal: lock is held
            // Hold the lock for 50 ms so the main thread has to retry.
            std::thread::sleep(Duration::from_millis(50));
            // _guard dropped here → lock released
        });

        barrier.wait(); // wait until the thread holds the lock
        // This should succeed after the thread releases (within the 1s timeout).
        let _guard = acquire_exclusive(&lock_path).expect("main acquire after thread release");
        handle.join().expect("thread join");
    }

    /// When the lock is held indefinitely, `acquire_exclusive` returns `LockTimeout`.
    #[test]
    fn timeout_on_contended_lock() {
        let dir = tempfile::tempdir().expect("tempdir");
        let lock_path = dir.path().join("test.lock");

        // Hold the lock in another thread for longer than the timeout.
        let lock_path_clone = lock_path.clone();
        let barrier = Arc::new(Barrier::new(2));
        let barrier_clone = barrier.clone();

        let handle = std::thread::spawn(move || {
            let _guard = acquire_exclusive(&lock_path_clone).expect("thread acquire");
            barrier_clone.wait(); // signal: lock is held
            // Hold for 3 seconds — much longer than the 1-second timeout.
            std::thread::sleep(Duration::from_secs(3));
        });

        barrier.wait(); // wait until the thread holds the lock
        let result = acquire_exclusive(&lock_path);
        assert!(
            matches!(result, Err(PreferencesError::LockTimeout)),
            "expected LockTimeout, got: {result:?}"
        );
        handle.join().expect("thread join");
    }
}
