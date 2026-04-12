// SPDX-License-Identifier: MPL-2.0

//! Integration tests — concurrent preferences writes.
//!
//! Exercises file-locking (`fs4`), the `write_epoch` counter, and
//! `reload_from_disk` under concurrent access.  All tests use
//! `tempfile::TempDir` for filesystem isolation and the
//! `XDG_CONFIG_HOME` env-override pattern established by
//! `preferences_roundtrip.rs`.

use std::sync::Arc;
use std::sync::atomic::Ordering;

use tau_term_lib::preferences::schema::{
    AppearancePatch, Preferences, PreferencesPatch, TerminalPatch,
};
use tau_term_lib::preferences::store::PreferencesStore;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Build a `PreferencesStore` backed by a fresh tempdir via `XDG_CONFIG_HOME`.
///
/// Returns `(store, _tmpdir)`.  The `TempDir` guard must be kept alive for the
/// entire test — dropping it deletes the directory.
///
/// # Safety
/// Environment mutation is only safe under nextest's process-per-test
/// isolation (default).
fn store_in_tempdir() -> (
    Arc<parking_lot::RwLock<PreferencesStore>>,
    tempfile::TempDir,
) {
    let tmp = tempfile::TempDir::new().expect("create tempdir");
    // Create the tauterm config directory so the store can write to it.
    let prefs_dir = tmp.path().join("tauterm");
    std::fs::create_dir_all(&prefs_dir).expect("create prefs dir");

    let orig = std::env::var_os("XDG_CONFIG_HOME");
    // SAFETY: nextest process-per-test isolation — no concurrent env readers.
    unsafe { std::env::set_var("XDG_CONFIG_HOME", tmp.path()) };

    let store = PreferencesStore::load_or_default();

    // SAFETY: same as above.
    unsafe {
        match orig {
            Some(v) => std::env::set_var("XDG_CONFIG_HOME", v),
            None => std::env::remove_var("XDG_CONFIG_HOME"),
        }
    }

    (store, tmp)
}

/// Build a `PreferencesPatch` that sets only `font_size`.
fn font_size_patch(size: f32) -> PreferencesPatch {
    PreferencesPatch {
        appearance: Some(AppearancePatch {
            font_size: Some(size),
            ..Default::default()
        }),
        ..Default::default()
    }
}

/// Build a `PreferencesPatch` that sets only `scrollback_lines`.
fn scrollback_patch(lines: usize) -> PreferencesPatch {
    PreferencesPatch {
        terminal: Some(TerminalPatch {
            scrollback_lines: Some(lines),
            ..Default::default()
        }),
        ..Default::default()
    }
}

/// Read the preferences TOML from disk and parse it, asserting no corruption.
fn read_prefs_from_disk(tmp: &tempfile::TempDir) -> Preferences {
    let path = tmp.path().join("tauterm").join("preferences.toml");
    let content = std::fs::read_to_string(&path).expect("preferences.toml must exist after save");
    // Must be valid TOML — any corruption would surface here.
    let value: toml::Value = toml::from_str(&content).expect("file must be valid TOML");
    // Re-serialize through snake_case → camelCase conversion to get a Preferences struct.
    // The simplest way: reload via a fresh store.  But since we only need to verify
    // non-corruption, parsing as toml::Value is sufficient.  For full-struct validation
    // we attempt deserialization with snake_case keys (the on-disk format).
    let _ = value; // toml::Value parse succeeded — not corrupted.
    // For full struct validation, also parse directly (snake_case keys match serde aliases).
    toml::from_str::<Preferences>(&content)
        .expect("file must deserialize to Preferences without corruption")
}

// ---------------------------------------------------------------------------
// Test 1 — concurrent_writes_are_serialized
// ---------------------------------------------------------------------------

#[test]
fn concurrent_writes_are_serialized() {
    let (store, tmp) = store_in_tempdir();

    let store1 = Arc::clone(&store);
    let store2 = Arc::clone(&store);

    let t1 = std::thread::spawn(move || {
        store1
            .read()
            .apply_patch(font_size_patch(20.0))
            .expect("apply_patch thread 1");
    });

    let t2 = std::thread::spawn(move || {
        store2
            .read()
            .apply_patch(scrollback_patch(5_000))
            .expect("apply_patch thread 2");
    });

    t1.join().expect("thread 1 must not panic");
    t2.join().expect("thread 2 must not panic");

    // File on disk must be valid (not corrupted by concurrent writes).
    let _prefs = read_prefs_from_disk(&tmp);

    // Both writes must have been counted.
    let epoch = store.read().write_epoch.load(Ordering::SeqCst);
    assert_eq!(
        epoch, 2,
        "write_epoch must be 2 after two apply_patch calls"
    );
}

// ---------------------------------------------------------------------------
// Test 2 — write_epoch_increments_on_save
// ---------------------------------------------------------------------------

#[test]
fn write_epoch_increments_on_save() {
    let (store, _tmp) = store_in_tempdir();

    assert_eq!(
        store.read().write_epoch.load(Ordering::SeqCst),
        0,
        "epoch must start at 0"
    );

    store
        .read()
        .apply_patch(font_size_patch(16.0))
        .expect("first apply_patch");
    assert_eq!(
        store.read().write_epoch.load(Ordering::SeqCst),
        1,
        "epoch must be 1 after first save"
    );

    store
        .read()
        .apply_patch(font_size_patch(18.0))
        .expect("second apply_patch");
    assert_eq!(
        store.read().write_epoch.load(Ordering::SeqCst),
        2,
        "epoch must be 2 after second save"
    );
}

// ---------------------------------------------------------------------------
// Test 3 — reload_from_disk_picks_up_external_changes
// ---------------------------------------------------------------------------

#[test]
fn reload_from_disk_picks_up_external_changes() {
    let (store, tmp) = store_in_tempdir();

    // Save initial prefs to disk so the file exists.
    store
        .read()
        .apply_patch(font_size_patch(14.0))
        .expect("initial save");

    // Externally overwrite the file with a different font_size.
    // On-disk format uses snake_case keys (ADR-0016).
    let path = tmp.path().join("tauterm").join("preferences.toml");
    let external_toml = r#"
[appearance]
font_family = "monospace"
font_size = 20.0
cursor_style = "block"
cursor_blink_ms = 530
theme_name = "umbra"
opacity = 1.0
language = "en"
context_menu_hint_shown = false
fullscreen = false
hide_cursor_while_typing = false
show_pane_title_bar = true

[terminal]
scrollback_lines = 10000
allow_osc52_write = false
word_delimiters = " /\\()\"'-.,:;<>~!@#$%^&*|+=[]{}~?│"
bell_type = "visual"
confirm_multiline_paste = true

[keyboard]
bindings = {}
"#;
    std::fs::write(&path, external_toml).expect("external write");

    // Reload from disk.
    let reloaded = store
        .read()
        .reload_from_disk()
        .expect("reload_from_disk must succeed");

    assert_eq!(
        reloaded.appearance.font_size, 20.0,
        "reload must pick up the externally written font_size"
    );

    // In-memory state must also reflect the change.
    let in_memory = store.read().get();
    assert_eq!(
        in_memory.appearance.font_size, 20.0,
        "in-memory prefs must match reloaded value"
    );
}

// ---------------------------------------------------------------------------
// Test 4 — lock_prevents_concurrent_corruption (stress test)
// ---------------------------------------------------------------------------

#[test]
fn lock_prevents_concurrent_corruption() {
    let (store, tmp) = store_in_tempdir();

    let threads: Vec<_> = (0..10)
        .map(|thread_idx| {
            let store = Arc::clone(&store);
            std::thread::spawn(move || {
                for iter in 0..5 {
                    // Each thread uses a distinct font_size range to detect interleaving.
                    let size = 8.0_f32 + (thread_idx as f32) + (iter as f32) * 0.1;
                    // Clamp to valid range [6.0, 72.0].
                    let size = size.clamp(6.0_f32, 72.0_f32);
                    store
                        .read()
                        .apply_patch(font_size_patch(size))
                        .expect("apply_patch must not fail under contention");
                }
            })
        })
        .collect();

    for t in threads {
        t.join().expect("worker thread must not panic");
    }

    // File on disk must be valid after all concurrent writes.
    let _prefs = read_prefs_from_disk(&tmp);

    // 10 threads * 5 writes = 50 epoch increments.
    let epoch = store.read().write_epoch.load(Ordering::SeqCst);
    assert_eq!(
        epoch, 50,
        "write_epoch must equal 50 after 10 threads * 5 writes"
    );
}
