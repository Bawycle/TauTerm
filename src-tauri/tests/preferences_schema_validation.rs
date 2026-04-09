// SPDX-License-Identifier: MPL-2.0

//! Integration tests — preferences schema validation (TEST-PREF-002).
//!
//! Covers:
//! - Numeric fields outside their logical range (font_size = 0, scrollback = max usize)
//! - `load_or_default` robustness on fuzz-minimal payloads (random bytes, XML, binary)
//! - Boundary values for fields that carry semantic constraints

use tau_term_lib::preferences::schema::{BellType, CursorStyle, Language, Preferences};
use tau_term_lib::preferences::store::PreferencesStore;

// ---------------------------------------------------------------------------
// Helper
// ---------------------------------------------------------------------------

/// Write bytes to a fresh temp dir with `$XDG_CONFIG_HOME/tauterm/preferences.json`
/// layout, run `load_or_default`, restore env, return the loaded preferences.
///
/// # Safety
/// Safe only under nextest process-per-test isolation.
fn load_prefs_from_bytes(content: &[u8]) -> Preferences {
    let tmp = tempfile::TempDir::new().expect("tempdir");
    let prefs_dir = tmp.path().join("tauterm");
    std::fs::create_dir_all(&prefs_dir).expect("create dir");
    std::fs::write(prefs_dir.join("preferences.json"), content).expect("write");

    let orig = std::env::var_os("XDG_CONFIG_HOME");
    // SAFETY: nextest process-per-test — no concurrent env readers.
    unsafe { std::env::set_var("XDG_CONFIG_HOME", tmp.path()) };

    let store = PreferencesStore::load_or_default();
    let prefs = store.read().get();

    // SAFETY: same as above.
    unsafe {
        match orig {
            Some(v) => std::env::set_var("XDG_CONFIG_HOME", v),
            None => std::env::remove_var("XDG_CONFIG_HOME"),
        }
    }
    prefs
}

// ---------------------------------------------------------------------------
// Numeric boundary — font_size
// ---------------------------------------------------------------------------

#[test]
fn font_size_zero_clamped_to_minimum() {
    // Values below the minimum [6.0, 72.0] are clamped at load time.
    let json = br#"{"appearance": {"fontSize": 0.0}}"#;
    let prefs = load_prefs_from_bytes(json);
    assert_eq!(prefs.appearance.font_size, 6.0);
}

#[test]
fn font_size_very_large_clamped_to_maximum() {
    // Values above 72.0 are clamped at load time.
    let json = br#"{"appearance": {"fontSize": 999.0}}"#;
    let prefs = load_prefs_from_bytes(json);
    assert_eq!(prefs.appearance.font_size, 72.0);
}

#[test]
fn font_size_negative_clamped_to_minimum() {
    // Negative values are not semantically valid — clamped to minimum, no panic.
    let json = br#"{"appearance": {"fontSize": -1.0}}"#;
    let prefs = load_prefs_from_bytes(json);
    assert_eq!(prefs.appearance.font_size, 6.0);
}

// ---------------------------------------------------------------------------
// Numeric boundary — opacity
// ---------------------------------------------------------------------------

#[test]
fn opacity_zero_accepted_no_panic() {
    let json = br#"{"appearance": {"opacity": 0.0}}"#;
    let prefs = load_prefs_from_bytes(json);
    assert_eq!(prefs.appearance.opacity, 0.0);
}

#[test]
fn opacity_one_accepted_no_panic() {
    let json = br#"{"appearance": {"opacity": 1.0}}"#;
    let prefs = load_prefs_from_bytes(json);
    assert_eq!(prefs.appearance.opacity, 1.0);
}

#[test]
fn opacity_out_of_range_above_clamped_to_maximum() {
    // Values above 1.0 are clamped at load time; no panic.
    let json = br#"{"appearance": {"opacity": 2.5}}"#;
    let prefs = load_prefs_from_bytes(json);
    assert_eq!(prefs.appearance.opacity, 1.0);
}

// ---------------------------------------------------------------------------
// Numeric boundary — scrollback_lines
// ---------------------------------------------------------------------------

#[test]
fn scrollback_lines_zero_clamped_to_minimum() {
    // Values below 100 are clamped at load time; no panic.
    let json = br#"{"terminal": {"scrollbackLines": 0}}"#;
    let prefs = load_prefs_from_bytes(json);
    assert_eq!(prefs.terminal.scrollback_lines, 100);
}

#[test]
fn scrollback_lines_large_value_accepted_no_panic() {
    // 1_000_000 lines — large but syntactically valid.
    let json = br#"{"terminal": {"scrollbackLines": 1000000}}"#;
    let prefs = load_prefs_from_bytes(json);
    assert_eq!(prefs.terminal.scrollback_lines, 1_000_000);
}

// ---------------------------------------------------------------------------
// Numeric boundary — cursor_blink_ms
// ---------------------------------------------------------------------------

#[test]
fn cursor_blink_ms_zero_accepted_no_panic() {
    let json = br#"{"appearance": {"cursorBlinkMs": 0}}"#;
    let prefs = load_prefs_from_bytes(json);
    assert_eq!(prefs.appearance.cursor_blink_ms, 0);
}

#[test]
fn cursor_blink_ms_max_u32_clamped_to_maximum() {
    // Values above 5000 ms are clamped at load time; no panic.
    let json = format!(r#"{{"appearance": {{"cursorBlinkMs": {}}}}}"#, u32::MAX);
    let prefs = load_prefs_from_bytes(json.as_bytes());
    assert_eq!(prefs.appearance.cursor_blink_ms, 5000);
}

// ---------------------------------------------------------------------------
// Enum boundaries — out-of-range variants fall back to defaults
// ---------------------------------------------------------------------------

#[test]
fn unknown_cursor_style_causes_full_default_fallback() {
    // serde rejects "beam" (not a valid CursorStyle variant) → full default.
    let json = br#"{"appearance": {"cursorStyle": "beam"}}"#;
    let prefs = load_prefs_from_bytes(json);
    // Falls back to default.
    assert_eq!(prefs.appearance.cursor_style, CursorStyle::default());
}

#[test]
fn unknown_bell_type_causes_full_default_fallback() {
    let json = br#"{"terminal": {"bellType": "vibrate"}}"#;
    let prefs = load_prefs_from_bytes(json);
    assert_eq!(prefs.terminal.bell_type, BellType::default());
}

#[test]
fn unknown_language_causes_full_default_fallback() {
    // "de" is not a valid Language variant → parse error → default.
    let json = br#"{"appearance": {"language": "de"}}"#;
    let prefs = load_prefs_from_bytes(json);
    assert_eq!(prefs.appearance.language, Language::default());
}

// ---------------------------------------------------------------------------
// Fuzz-minimal payloads — load_or_default must never panic
// ---------------------------------------------------------------------------

#[test]
fn load_or_default_does_not_panic_on_null_bytes() {
    let payload: Vec<u8> = vec![0u8; 256];
    let prefs = load_prefs_from_bytes(&payload);
    assert_eq!(prefs.appearance.font_family, "monospace");
}

#[test]
fn load_or_default_does_not_panic_on_random_ascii() {
    // Deterministic pseudo-random ASCII (no real RNG dependency in tests).
    let payload =
        b"x~#@&^%!<>|;:'\",./\\0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz";
    let prefs = load_prefs_from_bytes(payload);
    assert_eq!(prefs.terminal.scrollback_lines, 10_000);
}

#[test]
fn load_or_default_does_not_panic_on_xml_content() {
    let payload = br#"<?xml version="1.0"?><preferences><fontSize>14</fontSize></preferences>"#;
    let prefs = load_prefs_from_bytes(payload);
    assert_eq!(prefs.appearance.font_size, 14.0);
}

#[test]
fn load_or_default_does_not_panic_on_binary_content() {
    // Mix of valid UTF-8 and high bytes.
    let payload: Vec<u8> = (0u8..=255u8).collect();
    let prefs = load_prefs_from_bytes(&payload);
    assert_eq!(prefs.appearance.language, Language::En);
}

#[test]
fn load_or_default_does_not_panic_on_utf8_bom() {
    // BOM prefix on otherwise valid JSON.
    let mut payload = vec![0xEF, 0xBB, 0xBF]; // UTF-8 BOM
    payload.extend_from_slice(br#"{"appearance":{"fontFamily":"Hack"}}"#);
    // BOM causes JSON parse error → must fall back to defaults, not panic.
    let prefs = load_prefs_from_bytes(&payload);
    // Either parsed correctly or fell back — neither is a panic.
    assert!(!prefs.appearance.font_family.is_empty());
}

#[test]
fn load_or_default_does_not_panic_on_empty_json_object() {
    let prefs = load_prefs_from_bytes(b"{}");
    // Empty object → all defaults applied.
    assert_eq!(prefs.appearance.font_family, "monospace");
    assert_eq!(prefs.terminal.scrollback_lines, 10_000);
    assert_eq!(prefs.appearance.language, Language::En);
}

#[test]
fn load_or_default_does_not_panic_on_json_array() {
    // Top-level array is not a valid Preferences object → parse error → defaults.
    let prefs = load_prefs_from_bytes(b"[1, 2, 3]");
    assert_eq!(prefs.appearance.font_family, "monospace");
}

#[test]
fn load_or_default_does_not_panic_on_json_string() {
    let prefs = load_prefs_from_bytes(b"\"just a string\"");
    assert_eq!(prefs.appearance.font_family, "monospace");
}

#[test]
fn load_or_default_does_not_panic_on_deeply_nested_json() {
    // Deeply nested JSON that cannot map to Preferences.
    let prefs = load_prefs_from_bytes(br#"{"a":{"b":{"c":{"d":{"e":true}}}}}"#);
    assert_eq!(prefs.appearance.font_family, "monospace");
}

#[test]
fn load_or_default_does_not_panic_on_very_long_string_value() {
    // A 10 kB font name — valid JSON, unusual value.
    let long_name: String = "A".repeat(10_000);
    let json = format!(r#"{{"appearance":{{"fontFamily":"{}"}}}}"#, long_name);
    let prefs = load_prefs_from_bytes(json.as_bytes());
    assert_eq!(prefs.appearance.font_family, long_name);
}

// ---------------------------------------------------------------------------
// Serde field-level defaults — confirm partial objects apply per-field defaults
// ---------------------------------------------------------------------------

#[test]
fn missing_appearance_font_family_defaults_to_monospace() {
    // Appearance section present but fontFamily absent.
    let json = br#"{"appearance": {"fontSize": 16.0}}"#;
    let prefs = load_prefs_from_bytes(json);
    assert_eq!(prefs.appearance.font_family, "monospace");
}

#[test]
fn missing_terminal_bell_type_defaults_to_visual() {
    let json = br#"{"terminal": {"scrollbackLines": 1000}}"#;
    let prefs = load_prefs_from_bytes(json);
    assert_eq!(prefs.terminal.bell_type, BellType::Visual);
}

#[test]
fn missing_appearance_language_defaults_to_en() {
    let json = br#"{"appearance": {"fontFamily": "Terminus"}}"#;
    let prefs = load_prefs_from_bytes(json);
    assert_eq!(prefs.appearance.language, Language::En);
}
