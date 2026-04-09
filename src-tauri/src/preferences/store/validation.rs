// SPDX-License-Identifier: MPL-2.0

//! Preference value validation and clamping.
//!
//! Called after load and after patch application to ensure no out-of-range
//! values reach the in-memory store or the disk. All clamped values are
//! logged with `tracing::warn!` so operators can detect corrupted files.

use crate::preferences::schema::Preferences;

/// Clamp all numeric preference fields to their valid ranges.
///
/// Called:
/// 1. At the end of `load_from_disk` before returning `Preferences`.
/// 2. At the end of `apply_patch` before persisting.
///
/// Each clamped field emits a `warn!` event with the field name and original
/// value. The full filesystem path is never logged (security: username exposure).
pub(super) fn validate_and_clamp(prefs: &mut Preferences) {
    // --- appearance.font_size: [6.0, 72.0] ---
    let font_size = prefs.appearance.font_size;
    if font_size < 6.0 {
        tracing::warn!(
            field = "font_size",
            original = font_size,
            clamped = 6.0_f32,
            "preference value out of range, clamped to minimum"
        );
        prefs.appearance.font_size = 6.0;
    } else if font_size > 72.0 {
        tracing::warn!(
            field = "font_size",
            original = font_size,
            clamped = 72.0_f32,
            "preference value out of range, clamped to maximum"
        );
        prefs.appearance.font_size = 72.0;
    }

    // --- appearance.cursor_blink_ms: [0, 5000] ---
    let blink_ms = prefs.appearance.cursor_blink_ms;
    if blink_ms > 5000 {
        tracing::warn!(
            field = "cursor_blink_ms",
            original = blink_ms,
            clamped = 5000_u32,
            "preference value out of range, clamped to maximum"
        );
        prefs.appearance.cursor_blink_ms = 5000;
    }

    // --- appearance.opacity: [0.0, 1.0] ---
    let opacity = prefs.appearance.opacity;
    if opacity < 0.0 {
        tracing::warn!(
            field = "opacity",
            original = opacity,
            clamped = 0.0_f32,
            "preference value out of range, clamped to minimum"
        );
        prefs.appearance.opacity = 0.0;
    } else if opacity > 1.0 {
        tracing::warn!(
            field = "opacity",
            original = opacity,
            clamped = 1.0_f32,
            "preference value out of range, clamped to maximum"
        );
        prefs.appearance.opacity = 1.0;
    }

    // --- terminal.scrollback_lines: [100, 1_000_000] ---
    let scrollback = prefs.terminal.scrollback_lines;
    if scrollback < 100 {
        tracing::warn!(
            field = "scrollback_lines",
            original = scrollback,
            clamped = 100_usize,
            "preference value out of range, clamped to minimum"
        );
        prefs.terminal.scrollback_lines = 100;
    } else if scrollback > 1_000_000 {
        tracing::warn!(
            field = "scrollback_lines",
            original = scrollback,
            clamped = 1_000_000_usize,
            "preference value out of range, clamped to maximum"
        );
        prefs.terminal.scrollback_lines = 1_000_000;
    }

    // --- themes[*].line_height: [1.0, 2.0] if Some ---
    for theme in &mut prefs.themes {
        if let Some(lh) = theme.line_height {
            if lh < 1.0 {
                tracing::warn!(
                    field = "line_height",
                    theme = %theme.name,
                    original = lh,
                    clamped = 1.0_f32,
                    "theme line_height out of range, clamped to minimum"
                );
                theme.line_height = Some(1.0);
            } else if lh > 2.0 {
                tracing::warn!(
                    field = "line_height",
                    theme = %theme.name,
                    original = lh,
                    clamped = 2.0_f32,
                    "theme line_height out of range, clamped to maximum"
                );
                theme.line_height = Some(2.0);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_prefs() -> Preferences {
        Preferences::default()
    }

    #[test]
    fn font_size_below_min_is_clamped() {
        let mut p = make_prefs();
        p.appearance.font_size = 2.0;
        validate_and_clamp(&mut p);
        assert_eq!(p.appearance.font_size, 6.0);
    }

    #[test]
    fn font_size_above_max_is_clamped() {
        let mut p = make_prefs();
        p.appearance.font_size = 200.0;
        validate_and_clamp(&mut p);
        assert_eq!(p.appearance.font_size, 72.0);
    }

    #[test]
    fn font_size_within_range_is_unchanged() {
        let mut p = make_prefs();
        p.appearance.font_size = 14.0;
        validate_and_clamp(&mut p);
        assert_eq!(p.appearance.font_size, 14.0);
    }

    #[test]
    fn opacity_below_zero_is_clamped() {
        let mut p = make_prefs();
        p.appearance.opacity = -0.5;
        validate_and_clamp(&mut p);
        assert_eq!(p.appearance.opacity, 0.0);
    }

    #[test]
    fn opacity_above_one_is_clamped() {
        let mut p = make_prefs();
        p.appearance.opacity = 1.5;
        validate_and_clamp(&mut p);
        assert_eq!(p.appearance.opacity, 1.0);
    }

    #[test]
    fn cursor_blink_ms_above_max_is_clamped() {
        let mut p = make_prefs();
        p.appearance.cursor_blink_ms = 9999;
        validate_and_clamp(&mut p);
        assert_eq!(p.appearance.cursor_blink_ms, 5000);
    }

    #[test]
    fn scrollback_below_min_is_clamped() {
        let mut p = make_prefs();
        p.terminal.scrollback_lines = 10;
        validate_and_clamp(&mut p);
        assert_eq!(p.terminal.scrollback_lines, 100);
    }

    #[test]
    fn scrollback_above_max_is_clamped() {
        let mut p = make_prefs();
        p.terminal.scrollback_lines = 2_000_000;
        validate_and_clamp(&mut p);
        assert_eq!(p.terminal.scrollback_lines, 1_000_000);
    }

    #[test]
    fn theme_line_height_below_min_is_clamped() {
        let mut p = make_prefs();
        p.themes.push(crate::preferences::schema::UserTheme {
            name: "test-theme".to_string(),
            palette: Default::default(),
            foreground: "#fff".to_string(),
            background: "#000".to_string(),
            cursor_color: "#fff".to_string(),
            selection_bg: "#333".to_string(),
            line_height: Some(0.5),
        });
        validate_and_clamp(&mut p);
        assert_eq!(p.themes[0].line_height, Some(1.0));
    }

    #[test]
    fn theme_line_height_above_max_is_clamped() {
        let mut p = make_prefs();
        p.themes.push(crate::preferences::schema::UserTheme {
            name: "test-theme".to_string(),
            palette: Default::default(),
            foreground: "#fff".to_string(),
            background: "#000".to_string(),
            cursor_color: "#fff".to_string(),
            selection_bg: "#333".to_string(),
            line_height: Some(3.0),
        });
        validate_and_clamp(&mut p);
        assert_eq!(p.themes[0].line_height, Some(2.0));
    }

    #[test]
    fn theme_line_height_none_is_unchanged() {
        let mut p = make_prefs();
        p.themes.push(crate::preferences::schema::UserTheme {
            name: "test-theme".to_string(),
            palette: Default::default(),
            foreground: "#fff".to_string(),
            background: "#000".to_string(),
            cursor_color: "#fff".to_string(),
            selection_bg: "#333".to_string(),
            line_height: None,
        });
        validate_and_clamp(&mut p);
        assert_eq!(p.themes[0].line_height, None);
    }
}
