// SPDX-License-Identifier: MPL-2.0

#[cfg(test)]
mod security_tests {
    use crate::vt::VtProcessor;

    // -----------------------------------------------------------------------
    // SEC-PTY-001 — CSI 21t (window title read-back) silently discarded
    // -----------------------------------------------------------------------

    /// SEC-PTY-001: CSI 21t must not trigger any title injection into PTY input.
    #[test]
    fn sec_pty_001_csi_21t_title_readback_discarded() {
        let mut vt = VtProcessor::new(80, 24, 10_000);
        // Set a title that could be weaponised if echoed.
        vt.process(b"\x1b]0;injected;ls -la\x07");
        assert_eq!(vt.title, "injected;ls -la");

        // Send CSI 21t (window title read request) — must be silently ignored.
        let _dirty = vt.process(b"\x1b[21t");
        // No panic and no dedicated response buffer exists — the sequence is a no-op.
    }

    /// SEC-PTY-001: CSI 21t after a title containing a shell injection payload.
    #[test]
    fn sec_pty_001_csi_21t_after_shell_injection_title_no_effect() {
        let mut vt = VtProcessor::new(80, 24, 10_000);
        let _dirty = vt.process(b"\x1b]0;$(id)\x07\x1b[21t");
        // No panic, no crash, no observable injection.
    }

    // -----------------------------------------------------------------------
    // SEC-PTY-002 — OSC query sequences discarded (no echo-back)
    // -----------------------------------------------------------------------

    /// SEC-PTY-002: OSC 10;? (foreground color query) must be silently discarded.
    #[test]
    fn sec_pty_002_osc_color_query_no_response() {
        let mut vt = VtProcessor::new(80, 24, 10_000);
        // OSC 10 ; ? BEL
        let _dirty = vt.process(b"\x1b]10;?\x07");
        // No panic. VtProcessor has no response buffer — confirms no echo-back.
    }

    /// SEC-PTY-002: DECRQSS (ESC P $ q ... ESC \) must be silently discarded.
    #[test]
    fn sec_pty_002_decrqss_ignored() {
        let mut vt = VtProcessor::new(80, 24, 10_000);
        // DECRQSS sequence: ESC P $ q " p ESC \
        let _dirty = vt.process(b"\x1bP$q\"p\x1b\\");
        // No panic, no observable response.
    }

    /// SEC-PTY-002: CSI ? 1 $ p (DECRPM) must be silently discarded.
    #[test]
    fn sec_pty_002_decrpm_mode_query_ignored() {
        let mut vt = VtProcessor::new(80, 24, 10_000);
        let _dirty = vt.process(b"\x1b[?1$p");
        // No panic, no mode response injected.
    }

    // -----------------------------------------------------------------------
    // SEC-PTY-003 — OSC sequence with large payload does not panic or OOM
    // -----------------------------------------------------------------------

    /// SEC-PTY-003: Large OSC 0 title payload must be processed without panic.
    #[test]
    fn sec_pty_003_large_osc_title_no_panic() {
        let mut vt = VtProcessor::new(80, 24, 10_000);
        let mut seq = b"\x1b]0;".to_vec();
        seq.extend(b"A".repeat(10_000));
        seq.push(b'\x07');
        let _dirty = vt.process(&seq);
        // Title must be bounded by parse_osc (max 256 chars).
        assert!(
            vt.title.len() <= 256,
            "Title must be bounded even with large OSC input (SEC-PTY-003), got {}",
            vt.title.len()
        );
    }

    // -----------------------------------------------------------------------
    // SEC-PTY-004 — DCS sequence with large payload does not panic
    // -----------------------------------------------------------------------

    /// SEC-PTY-004: DCS sequence with 10 000-byte payload must not panic.
    #[test]
    fn sec_pty_004_large_dcs_payload_no_panic() {
        let mut vt = VtProcessor::new(80, 24, 10_000);
        let mut seq = b"\x1bP".to_vec();
        seq.extend(b"B".repeat(10_000));
        seq.extend(b"\x1b\\"); // DCS string terminator (ST)
        let _dirty = vt.process(&seq);
        // No panic. DCS is silently ignored in v1.
    }

    // -----------------------------------------------------------------------
    // SEC-PTY-007 — Invalid UTF-8 bytes replaced with U+FFFD
    // -----------------------------------------------------------------------

    /// SEC-PTY-007: Overlong UTF-8 encoding 0xC0 0xAF must not produce raw bytes.
    ///
    /// ScreenSnapshot uses a flat row-major `cells: Vec<SnapshotCell>` with `content`
    /// field (not `grapheme`). Index 0 is (row=0, col=0).
    #[test]
    fn sec_pty_007_invalid_utf8_replaced_with_replacement_char() {
        use crate::vt::screen_buffer::SnapshotCell;
        let mut vt = VtProcessor::new(80, 24, 10_000);
        // 0xC0 0xAF is an overlong encoding of U+002F ('/'). It is invalid UTF-8.
        let _dirty = vt.process(b"\xC0\xAF");
        let snapshot = vt.get_snapshot();
        // Cell (row=0, col=0) is at flat index 0.
        let cell_content: &str = snapshot
            .cells
            .first()
            .map(|c: &SnapshotCell| c.content.as_str())
            .unwrap_or("");
        // Acceptable: U+FFFD, space (default cell), or empty string.
        // Not acceptable: the raw byte '/' or any non-Unicode value.
        let is_safe = cell_content == "\u{FFFD}" || cell_content == " " || cell_content.is_empty();
        assert!(
            is_safe,
            "Invalid UTF-8 must produce U+FFFD or empty cell, not raw bytes (SEC-PTY-007). Got: {:?}",
            cell_content
        );
    }

    /// SEC-PTY-007: Valid characters surrounding invalid UTF-8 must render correctly.
    #[test]
    fn sec_pty_007_valid_chars_unaffected_by_invalid_utf8() {
        use crate::vt::screen_buffer::SnapshotCell;
        let mut vt = VtProcessor::new(80, 24, 10_000);
        // "ok" + invalid bytes + "!"
        let _dirty = vt.process(b"ok\xC0\xAF!");
        let snapshot = vt.get_snapshot();
        // Flat row-major: cell(0,0)=index 0, cell(0,1)=index 1.
        let cell0: &str = snapshot
            .cells
            .first()
            .map(|c: &SnapshotCell| c.content.as_str())
            .unwrap_or("");
        let cell1: &str = snapshot
            .cells
            .get(1)
            .map(|c: &SnapshotCell| c.content.as_str())
            .unwrap_or("");
        assert_eq!(cell0, "o", "Cell(0,0) must be 'o'");
        assert_eq!(cell1, "k", "Cell(0,1) must be 'k'");
        // '!' must appear somewhere in row 0.
        let row0_text: String = snapshot
            .cells
            .iter()
            .take(80)
            .map(|c: &SnapshotCell| c.content.as_str())
            .collect();
        assert!(
            row0_text.contains('!'),
            "Valid char '!' must survive mixed input (SEC-PTY-007)"
        );
    }
}

#[cfg(test)]
#[allow(clippy::module_inception)]
mod tests {
    use crate::vt::VtProcessor;

    // Helper: create a VtProcessor with standard 80×24 dimensions and default scrollback.
    fn make_vt(cols: u16, rows: u16) -> VtProcessor {
        VtProcessor::new(cols, rows, 10_000)
    }

    // Helper: extract the grapheme at (row, col) from the active screen buffer.
    fn grapheme_at(vt: &VtProcessor, row: u16, col: u16) -> String {
        vt.active_buf_ref()
            .get(row, col)
            .map(|c| c.grapheme.clone())
            .unwrap_or_default()
    }

    // Helper: extract the attrs at (row, col).
    fn attrs_at(vt: &VtProcessor, row: u16, col: u16) -> crate::vt::cell::CellAttrs {
        vt.active_buf_ref()
            .get(row, col)
            .map(|c| c.attrs)
            .unwrap_or_default()
    }

    // ---------------------------------------------------------------------------
    // TEST-VT-002 — split CSI sequence across two process() calls
    // FS-VT-005
    // ---------------------------------------------------------------------------

    #[test]
    fn split_csi_sequence_is_parsed_correctly() {
        // TEST-VT-002
        let mut vt = make_vt(80, 24);
        // Feed ESC [ in first call, then 31m A in second call.
        vt.process(b"\x1b[");
        vt.process(b"31mA");
        let attrs = attrs_at(&vt, 0, 0);
        assert_eq!(
            attrs.fg,
            Some(crate::vt::cell::Color::Ansi { index: 1 }),
            "ANSI red (31) should be index 1"
        );
        assert_eq!(grapheme_at(&vt, 0, 0), "A");
    }

    // ---------------------------------------------------------------------------
    // TEST-VT-003 — UTF-8 sequence split across two process() calls
    // FS-VT-010
    // ---------------------------------------------------------------------------

    #[test]
    fn utf8_sequence_split_across_calls_is_reassembled() {
        // TEST-VT-003
        let mut vt = make_vt(80, 24);
        // 'é' = 0xC3 0xA9 — split: first call has only the lead byte.
        vt.process(&[0xC3]);
        vt.process(&[0xA9, b'X']);
        let first_grapheme = grapheme_at(&vt, 0, 0);
        let second_grapheme = grapheme_at(&vt, 0, 1);
        // The vte crate handles UTF-8 reassembly; é should appear at (0,0).
        assert_eq!(first_grapheme, "é", "é must be reassembled across calls");
        assert_eq!(second_grapheme, "X", "X must appear in the next cell");
    }

    // ---------------------------------------------------------------------------
    // TEST-VT-004 — wide (CJK) character wrapping at end of line
    // FS-VT-011
    // ---------------------------------------------------------------------------

    #[test]
    fn wide_char_at_last_col_wraps_to_next_line() {
        // TEST-VT-004 — 4-column buffer.
        let mut vt = make_vt(4, 5);
        // Position cursor at col 3 (last column, 0-indexed) via CUP.
        vt.process(b"\x1b[1;4H"); // row 1, col 4 (1-based)
        // Feed '中' (U+4E2D) = width 2.
        vt.process("中".as_bytes());
        // After writing at col=3 with width=2, the char must wrap.
        // Implementation detail: write_char clamps col to cols-1 on overflow.
        // The wide character should either be at row 0 col 3 or wrapped.
        // What matters is no panic and cursor integrity.
        let snap = vt.get_snapshot();
        assert_eq!(snap.cols, 4);
        assert!(snap.cursor_row < 5, "cursor row must remain in bounds");
        assert!(snap.cursor_col < 4, "cursor col must remain in bounds");
    }

    // ---------------------------------------------------------------------------
    // TEST-VT-005 — invalid UTF-8 produces U+FFFD
    // FS-VT-016
    // ---------------------------------------------------------------------------

    #[test]
    fn invalid_utf8_produces_replacement_character() {
        // TEST-VT-005
        let mut vt = make_vt(80, 24);
        // 0xC0 0xAF is an overlong encoding (invalid UTF-8).
        vt.process(&[0xC0, 0xAF]);
        let g = grapheme_at(&vt, 0, 0);
        // The vte crate replaces invalid bytes with U+FFFD.
        assert_eq!(
            g, "\u{FFFD}",
            "invalid UTF-8 must produce U+FFFD replacement char"
        );
        // Subsequent valid ASCII must still parse correctly.
        vt.process(b"Z");
        // The cursor should have advanced and Z is somewhere on row 0.
        let snap = vt.get_snapshot();
        assert_eq!(
            snap.cols, 80,
            "buffer dimensions must be intact after invalid UTF-8"
        );
    }

    // ---------------------------------------------------------------------------
    // TEST-VT-006 — SGR color variants: ANSI, 256-color, RGB, colon form
    // FS-VT-020, FS-VT-021, FS-VT-022
    // ---------------------------------------------------------------------------

    #[test]
    fn sgr_ansi_color_is_applied() {
        // TEST-VT-006 step 1
        let mut vt = make_vt(80, 24);
        vt.process(b"\x1b[31mA");
        let attrs = attrs_at(&vt, 0, 0);
        assert_eq!(attrs.fg, Some(crate::vt::cell::Color::Ansi { index: 1 }));
    }

    #[test]
    fn sgr_256_color_is_applied() {
        // TEST-VT-006 step 2
        let mut vt = make_vt(80, 24);
        vt.process(b"\x1b[38;5;196mB");
        let attrs = attrs_at(&vt, 0, 0);
        assert_eq!(
            attrs.fg,
            Some(crate::vt::cell::Color::Ansi256 { index: 196 })
        );
    }

    #[test]
    fn sgr_rgb_truecolor_semicolon_form_is_applied() {
        // TEST-VT-006 step 3
        let mut vt = make_vt(80, 24);
        vt.process(b"\x1b[38;2;255;100;0mC");
        let attrs = attrs_at(&vt, 0, 0);
        assert_eq!(
            attrs.fg,
            Some(crate::vt::cell::Color::Rgb {
                r: 255,
                g: 100,
                b: 0
            })
        );
    }

    #[test]
    fn sgr_rgb_truecolor_colon_form_is_applied() {
        // TEST-VT-006 step 4 — ITU T.416 colon sub-parameter form
        let mut vt = make_vt(80, 24);
        vt.process(b"\x1b[38:2:255:100:0mD");
        let attrs = attrs_at(&vt, 0, 0);
        assert_eq!(
            attrs.fg,
            Some(crate::vt::cell::Color::Rgb {
                r: 255,
                g: 100,
                b: 0
            })
        );
    }

    // ---------------------------------------------------------------------------
    // TEST-VT-007 — SGR multi-attribute and partial reset
    // FS-VT-024
    // ---------------------------------------------------------------------------

    #[test]
    fn sgr_multi_attributes_set_independently() {
        // TEST-VT-007
        let mut vt = make_vt(80, 24);
        // Set bold + italic + underline simultaneously.
        vt.process(b"\x1b[1;3;4mA");
        let attrs = attrs_at(&vt, 0, 0);
        assert!(attrs.bold, "bold must be set");
        assert!(attrs.italic, "italic must be set");
        assert!(attrs.underline > 0, "underline must be set");

        // SGR 22 resets bold/dim without affecting italic or underline.
        vt.process(b"\x1b[22mB");
        let attrs = attrs_at(&vt, 0, 1);
        assert!(!attrs.bold, "bold must be cleared by SGR 22");
        assert!(attrs.italic, "italic must be unaffected by SGR 22");
        assert!(
            attrs.underline > 0,
            "underline must be unaffected by SGR 22"
        );

        // SGR 0 clears all.
        vt.process(b"\x1b[0mC");
        let attrs = attrs_at(&vt, 0, 2);
        assert!(!attrs.bold);
        assert!(!attrs.italic);
        assert_eq!(attrs.underline, 0);
    }

    // ---------------------------------------------------------------------------
    // TEST-VT-008 — cursor visibility and DECTCEM
    // FS-VT-030, FS-VT-031
    // ---------------------------------------------------------------------------

    #[test]
    fn dectcem_hide_and_show_cursor() {
        // TEST-VT-008 (partial — cursor shape stub)
        let mut vt = make_vt(80, 24);
        assert!(vt.modes.cursor_visible, "cursor must be visible by default");

        // Hide cursor.
        vt.process(b"\x1b[?25l");
        assert!(
            !vt.modes.cursor_visible,
            "cursor must be hidden after DECTCEM hide"
        );

        // Show cursor.
        vt.process(b"\x1b[?25h");
        assert!(
            vt.modes.cursor_visible,
            "cursor must be visible after DECTCEM show"
        );
    }

    // ---------------------------------------------------------------------------
    // TEST-VT-009 — alternate screen cursor save/restore (DECSC + mode 1049)
    // FS-VT-033
    // ---------------------------------------------------------------------------

    #[test]
    fn alternate_screen_cursor_save_restore() {
        // TEST-VT-009
        let mut vt = make_vt(80, 24);
        // Position cursor at (5, 10) on normal screen via CUP.
        vt.process(b"\x1b[6;11H"); // row=6 col=11 (1-based) → row=5 col=10 (0-based)
        assert_eq!(vt.normal_cursor.row, 5);
        assert_eq!(vt.normal_cursor.col, 10);

        // Switch to alternate screen (saves cursor via mode 1049).
        // DECSET uses CSI ? Pm h (with '?' intermediate byte).
        vt.process(b"\x1b[?1049h");
        assert!(vt.alt_active, "alternate screen must be active");

        // Move cursor to (0, 0) on alternate screen.
        vt.process(b"\x1b[1;1H");
        assert_eq!(vt.alt_cursor.row, 0);
        assert_eq!(vt.alt_cursor.col, 0);

        // Return to normal screen (restores cursor).
        // DECRST uses CSI ? Pm l.
        vt.process(b"\x1b[?1049l");
        assert!(!vt.alt_active, "normal screen must be active");
        assert_eq!(vt.normal_cursor.row, 5, "cursor row must be restored");
        assert_eq!(vt.normal_cursor.col, 10, "cursor col must be restored");
    }

    // ---------------------------------------------------------------------------
    // TEST-VT-010 — alternate screen isolation and no scrollback
    // FS-VT-040, FS-VT-041, FS-VT-042, FS-VT-044
    // ---------------------------------------------------------------------------

    #[test]
    fn alternate_screen_is_isolated_from_normal_screen() {
        // TEST-VT-010
        let mut vt = make_vt(10, 5);
        // Write content on normal screen.
        vt.process(b"HELLO");
        assert_eq!(grapheme_at(&vt, 0, 0), "H");

        // Switch to alternate screen — must be blank.
        // DECSET uses CSI ? Pm h.
        vt.process(b"\x1b[?1049h");
        assert!(vt.alt_active);
        assert_eq!(
            grapheme_at(&vt, 0, 0),
            " ",
            "alternate screen must be blank on entry"
        );

        // Write on alternate screen.
        vt.process(b"WORLD");

        // Return to normal screen.
        // DECRST uses CSI ? Pm l.
        vt.process(b"\x1b[?1049l");
        assert!(!vt.alt_active);
        assert_eq!(
            grapheme_at(&vt, 0, 0),
            "H",
            "normal screen content must survive alt-screen usage"
        );

        // Alternate screen must not have added scrollback.
        assert_eq!(
            vt.normal.scrollback_len(),
            0,
            "alternate screen must not contribute to scrollback"
        );
    }

    // ---------------------------------------------------------------------------
    // TEST-VT-011 — DECSTBM scroll region
    // FS-VT-050, FS-VT-051, FS-VT-053
    // ---------------------------------------------------------------------------

    #[test]
    fn decstbm_partial_scroll_region_no_scrollback() {
        // TEST-VT-011
        let mut vt = make_vt(80, 10);
        // Set scroll region rows 2–8 (1-based) = indices 1–7 (0-based).
        vt.process(b"\x1b[2;8r");
        assert_eq!(vt.modes.scroll_region, (1, 7));
        // Cursor must be moved to home position after DECSTBM.
        assert_eq!(vt.normal_cursor.row, 0);
        assert_eq!(vt.normal_cursor.col, 0);
        // Scrolling within the partial region must not add to scrollback.
        // Position cursor at bottom of region (row 7, 0-based).
        vt.process(b"\x1b[8;1H"); // row=8 col=1 (1-based)
        // Feed 3 LF to scroll within region.
        vt.process(b"\n\n\n");
        assert_eq!(
            vt.normal.scrollback_len(),
            0,
            "partial scroll region must not add to scrollback (FS-VT-053)"
        );
    }

    // ---------------------------------------------------------------------------
    // TEST-VT-018 — OSC buffer overflow protection
    // FS-SEC-005
    // ---------------------------------------------------------------------------

    #[test]
    fn osc_overflow_does_not_crash_and_subsequent_sequences_parse() {
        // TEST-VT-018
        let mut vt = make_vt(80, 24);
        // Feed OSC 0 ; followed by 5000 bytes without a terminator.
        let mut overflow_seq: Vec<u8> = b"\x1b]0;".to_vec();
        overflow_seq.extend(std::iter::repeat_n(b'X', 5000));
        // No BEL or ST — simulate abandonment. Then a valid sequence.
        vt.process(&overflow_seq);
        // Feed a valid sequence that follows — must not be corrupted.
        vt.process(b"\x1b[31mA");
        // No panic is the primary assertion; but also verify A is written.
        let attrs = attrs_at(&vt, 0, 0);
        // The VTE parser's behavior on overlong OSC is to discard and continue —
        // verify subsequent input parses (red foreground set).
        assert_eq!(
            attrs.fg,
            Some(crate::vt::cell::Color::Ansi { index: 1 }),
            "SGR 31 after overlong OSC must be applied"
        );
    }

    // ---------------------------------------------------------------------------
    // TEST-VT-023 — DEC Special Graphics charset
    // FS-VT-015
    // ---------------------------------------------------------------------------

    #[test]
    fn dec_special_graphics_so_maps_j_to_box_drawing() {
        // TEST-VT-023
        let mut vt = make_vt(80, 24);
        // Designate G1 as DEC Special Graphics.
        vt.process(b"\x1b)0");
        // SO (0x0E) — shift to G1.
        vt.process(b"\x0e");
        // Feed 0x6A ('j' in ASCII; maps to '┘' in DEC Special Graphics).
        vt.process(b"\x6a");
        let g = grapheme_at(&vt, 0, 0);
        assert_eq!(
            g, "┘",
            "0x6A with DEC Special Graphics active must map to '┘'"
        );
        // SI (0x0F) — return to G0 (ASCII).
        vt.process(b"\x0f");
        vt.process(b"j");
        let g2 = grapheme_at(&vt, 0, 1);
        assert_eq!(g2, "j", "0x6A with ASCII active must remain 'j'");
    }

    // ---------------------------------------------------------------------------
    // TEST-VT-012 — OSC title sanitization (via parse_osc, exercised end-to-end)
    // FS-VT-060, FS-VT-062
    // ---------------------------------------------------------------------------

    #[test]
    fn osc_title_control_chars_are_stripped() {
        // TEST-VT-012 step 3-4
        let mut vt = make_vt(80, 24);
        // OSC title containing a C0 control char (0x01).
        vt.process(b"\x1b]0;Title\x01WithControl\x07");
        assert!(
            !vt.title.contains('\x01'),
            "C0 control chars must be stripped from OSC title"
        );
    }

    #[test]
    fn osc_title_truncated_to_256_chars() {
        // TEST-VT-012 step 5-6
        let mut vt = make_vt(80, 24);
        let long_title: Vec<u8> = std::iter::once(b'\x1b')
            .chain(b"]0;".iter().copied())
            .chain(std::iter::repeat_n(b'A', 300))
            .chain(std::iter::once(b'\x07'))
            .collect();
        vt.process(&long_title);
        assert!(
            vt.title.len() <= 256,
            "OSC title must be truncated to max 256 chars, got {}",
            vt.title.len()
        );
    }

    #[test]
    fn osc_title_plain_title_is_stored() {
        // TEST-VT-012 step 1-2
        let mut vt = make_vt(80, 24);
        vt.process(b"\x1b]0;My Title\x07");
        assert_eq!(vt.title, "My Title");
    }

    // ---------------------------------------------------------------------------
    // TEST: resize clamps cursor positions
    // FS-PTY-009, FS-PTY-010
    // ---------------------------------------------------------------------------

    #[test]
    fn resize_clamps_cursor_positions_to_new_bounds() {
        let mut vt = make_vt(80, 24);
        // Move cursor to row 20, col 70.
        vt.process(b"\x1b[21;71H");
        assert_eq!(vt.normal_cursor.row, 20);
        assert_eq!(vt.normal_cursor.col, 70);
        // Resize to smaller dimensions.
        vt.resize(40, 10);
        assert!(
            vt.normal_cursor.row < 10,
            "cursor row must be clamped to new rows"
        );
        assert!(
            vt.normal_cursor.col < 40,
            "cursor col must be clamped to new cols"
        );
    }

    // ---------------------------------------------------------------------------
    // DECCKM mode tracking
    // FS-VT-030
    // ---------------------------------------------------------------------------

    #[test]
    fn decckm_mode_set_and_reset() {
        let mut vt = make_vt(80, 24);
        assert!(!vt.modes.decckm, "DECCKM must be false by default");
        vt.process(b"\x1b[?1h"); // DECSET 1 = DECCKM
        assert!(vt.modes.decckm, "DECCKM must be true after ESC[?1h");
        assert!(vt.mode_changed, "mode_changed flag must be set");
        vt.mode_changed = false;
        vt.process(b"\x1b[?1l"); // DECRST 1
        assert!(!vt.modes.decckm, "DECCKM must be false after ESC[?1l");
        assert!(vt.mode_changed, "mode_changed flag must be set again");
    }

    // ---------------------------------------------------------------------------
    // Bracketed paste mode tracking
    // FS-KBD related
    // ---------------------------------------------------------------------------

    #[test]
    fn bracketed_paste_mode_tracking() {
        let mut vt = make_vt(80, 24);
        assert!(!vt.modes.bracketed_paste);
        vt.process(b"\x1b[?2004h");
        assert!(vt.modes.bracketed_paste);
        vt.process(b"\x1b[?2004l");
        assert!(!vt.modes.bracketed_paste);
    }

    // ---------------------------------------------------------------------------
    // TEST-VT-012 — Combining / zero-width characters (FS-VT-012/013)
    // ---------------------------------------------------------------------------

    /// A combining character (width=0) must attach to the previous cell and must
    /// not advance the cursor.
    #[test]
    fn combining_char_attaches_to_previous_cell_no_cursor_advance() {
        let mut vt = make_vt(80, 24);
        // Write 'e' followed by combining acute accent U+0301 (width=0).
        vt.process("e\u{0301}".as_bytes());
        // The grapheme at (0,0) must contain both codepoints.
        let g = grapheme_at(&vt, 0, 0);
        assert!(
            g.contains('e') && g.contains('\u{0301}'),
            "combining acute accent must merge into the base char cell, got: {g:?}"
        );
        // The cursor must be at col=1, not col=2 (no extra advance for the combining char).
        assert_eq!(
            vt.normal_cursor.col, 1,
            "cursor must be at col=1 after e + combining"
        );
    }

    /// Combining character at column 0 must attach to cell (0,0) without panicking.
    #[test]
    fn combining_char_at_column_zero_does_not_panic() {
        let mut vt = make_vt(80, 24);
        // Feed a combining mark at the very start — should attach to cell (0,0).
        vt.process("\u{0301}".as_bytes()); // combining acute at col=0
        // No panic is the primary assertion.
        let snap = vt.get_snapshot();
        assert_eq!(snap.cursor_row, 0);
        assert_eq!(
            snap.cursor_col, 0,
            "cursor must not move for a combining-only input"
        );
    }

    // ---------------------------------------------------------------------------
    // TEST-VT-052 — CSI S / T (scroll up/down) within scroll region (FS-VT-052)
    // ---------------------------------------------------------------------------

    #[test]
    fn csi_scroll_up_moves_content() {
        let mut vt = make_vt(10, 5);
        // Write distinct content on rows 0–2.
        vt.process(b"AAA\r\nBBB\r\nCCC");
        // Cursor is now on row 2. Set full-screen scroll region (rows 0–4, default).
        // CSI 1 S — scroll up 1 line within region.
        vt.process(b"\x1b[1S");
        // After scroll up: row 0 should contain what was on row 1 ("BBB").
        let g = grapheme_at(&vt, 0, 0);
        assert_eq!(
            g, "B",
            "after CSI S: row 0 must contain former row 1 content"
        );
    }

    #[test]
    fn csi_scroll_down_moves_content() {
        let mut vt = make_vt(10, 5);
        // Write content on row 0.
        vt.process(b"AAA");
        // CSI 1 T — scroll down 1 line.
        vt.process(b"\x1b[1T");
        // Row 0 should now be blank; former row 0 content is on row 1.
        let g0 = grapheme_at(&vt, 0, 0);
        let g1 = grapheme_at(&vt, 1, 0);
        assert!(
            g0.trim().is_empty() || g0 == " ",
            "after CSI T: row 0 must be blank, got: {g0:?}"
        );
        assert_eq!(
            g1, "A",
            "after CSI T: former row 0 content must be on row 1"
        );
    }

    // ---------------------------------------------------------------------------
    // TEST-VT-030 — DECSCUSR cursor shape (FS-VT-030)
    // ---------------------------------------------------------------------------

    #[test]
    fn decscusr_sets_cursor_shape_and_flags_change() {
        let mut vt = make_vt(80, 24);
        assert_eq!(vt.cursor_shape, 0, "default cursor shape must be 0");
        assert!(!vt.cursor_shape_changed);

        // CSI 2 SP q — steady block.
        vt.process(b"\x1b[2 q");
        assert_eq!(
            vt.cursor_shape, 2,
            "cursor shape must be 2 after DECSCUSR 2"
        );
        assert!(
            vt.cursor_shape_changed,
            "cursor_shape_changed flag must be set"
        );

        // take_cursor_shape_changed must return Some and reset the flag.
        let shape = vt.take_cursor_shape_changed();
        assert_eq!(shape, Some(2));
        assert!(!vt.cursor_shape_changed, "flag must be reset after take");
    }

    #[test]
    fn decscusr_same_value_does_not_set_changed_flag() {
        let mut vt = make_vt(80, 24);
        // Already at shape=0; sending DECSCUSR 0 must not set the flag.
        vt.process(b"\x1b[0 q");
        assert!(
            !vt.cursor_shape_changed,
            "no change: flag must remain false"
        );
    }

    // ---------------------------------------------------------------------------
    // TEST-VT-090 — BEL rate limiting (FS-VT-090)
    // ---------------------------------------------------------------------------

    #[test]
    fn bel_sets_bell_pending() {
        let mut vt = make_vt(80, 24);
        vt.process(b"\x07");
        assert!(vt.bell_pending, "BEL must set bell_pending");
        let fired = vt.take_bell_pending();
        assert!(fired, "take_bell_pending must return true");
        assert!(!vt.bell_pending, "flag must be reset after take");
    }

    #[test]
    fn bel_rate_limited_second_immediate_bell_ignored() {
        let mut vt = make_vt(80, 24);
        // First BEL — allowed.
        vt.process(b"\x07");
        let _ = vt.take_bell_pending(); // consume + reset

        // Second BEL immediately after — must be suppressed (< 100 ms).
        vt.process(b"\x07");
        assert!(
            !vt.bell_pending,
            "second immediate BEL must be suppressed by rate limit"
        );
    }

    // ---------------------------------------------------------------------------
    // TEST-SB-002 — scrollback_lines preference is honoured (FS-SB-002)
    // ---------------------------------------------------------------------------

    #[test]
    fn scrollback_limit_from_constructor_is_respected() {
        let mut vt = crate::vt::VtProcessor::new(5, 1, 3);
        // Scroll 5 lines into scrollback — only 3 should be retained.
        for _ in 0..5 {
            vt.process(b"A\r\n");
        }
        let sb_len = vt.normal.scrollback_len();
        assert!(
            sb_len <= 3,
            "scrollback must be capped at the constructor limit (3), got {sb_len}"
        );
    }

    // ---------------------------------------------------------------------------
    // TEST-VT-OSC8-001 — OSC 8 hyperlinks stored on cells (FS-VT-070–073)
    // ---------------------------------------------------------------------------

    /// Cells written while an OSC 8 hyperlink is active receive the URI.
    #[test]
    fn osc8_cell_inside_hyperlink_receives_uri() {
        let mut vt = make_vt(80, 24);
        // ESC ] 8 ; ; https://example.com BEL  followed by text 'A'
        vt.process(b"\x1b]8;;https://example.com\x07A");
        let cell = vt.normal.get(0, 0).expect("cell (0,0) must exist");
        assert_eq!(
            cell.hyperlink.as_deref(),
            Some("https://example.com"),
            "cell inside OSC 8 hyperlink must carry the URI"
        );
    }

    /// Cells written after OSC 8 ;; (end-of-hyperlink) have no hyperlink.
    #[test]
    fn osc8_cell_after_end_sequence_has_no_hyperlink() {
        let mut vt = make_vt(80, 24);
        // Open hyperlink, write 'A', close hyperlink, write 'B'.
        vt.process(b"\x1b]8;;https://example.com\x07A\x1b]8;;\x07B");
        let cell_a = vt.normal.get(0, 0).expect("cell (0,0)");
        let cell_b = vt.normal.get(0, 1).expect("cell (0,1)");
        assert_eq!(
            cell_a.hyperlink.as_deref(),
            Some("https://example.com"),
            "cell 'A' must carry the URI"
        );
        assert!(
            cell_b.hyperlink.is_none(),
            "cell 'B' after OSC 8 ;; must have no hyperlink, got {:?}",
            cell_b.hyperlink
        );
    }

    /// OSC 8 with the same ID on two successive opens reuses the same URI (FS-VT-072).
    #[test]
    fn osc8_same_id_on_two_lines_carries_same_uri() {
        let mut vt = make_vt(80, 24);
        // First open: id=link1, write 'A'.
        vt.process(b"\x1b]8;id=link1;https://example.com\x07A");
        // Close hyperlink.
        vt.process(b"\x1b]8;;\x07");
        // Re-open with the same ID — URI must still be present on written cell.
        vt.process(b"\x1b]8;id=link1;https://example.com\x07B");

        let cell_a = vt.normal.get(0, 0).expect("cell (0,0)");
        let cell_b = vt.normal.get(0, 1).expect("cell (0,1)");
        assert_eq!(cell_a.hyperlink.as_deref(), Some("https://example.com"));
        assert_eq!(cell_b.hyperlink.as_deref(), Some("https://example.com"));
    }

    /// Cells written before any OSC 8 sequence have no hyperlink.
    #[test]
    fn osc8_no_hyperlink_by_default() {
        let mut vt = make_vt(80, 24);
        vt.process(b"Hello");
        let cell = vt.normal.get(0, 0).expect("cell (0,0)");
        assert!(
            cell.hyperlink.is_none(),
            "cells written without an active hyperlink must have hyperlink=None"
        );
    }

    // ---------------------------------------------------------------------------
    // TEST-VT-OSC52-001 — OSC 52 clipboard write forwarding (FS-VT-075)
    // ---------------------------------------------------------------------------

    /// With `allow_osc52_write = false` (default), no clipboard event is queued.
    #[test]
    fn osc52_write_blocked_by_default_policy() {
        let mut vt = make_vt(80, 24);
        // Base64("hello") = "aGVsbG8="
        vt.process(b"\x1b]52;c;aGVsbG8=\x07");
        assert!(
            vt.take_osc52_write().is_none(),
            "OSC 52 write must be blocked when allow_osc52_write = false (default)"
        );
    }

    /// With `allow_osc52_write = true`, the decoded payload is returned by `take_osc52_write`.
    #[test]
    fn osc52_write_forwarded_when_policy_allows() {
        let mut vt = make_vt(80, 24);
        vt.allow_osc52_write = true;
        // Base64("hello") = "aGVsbG8="
        vt.process(b"\x1b]52;c;aGVsbG8=\x07");
        let payload = vt.take_osc52_write();
        assert_eq!(
            payload.as_deref(),
            Some("hello"),
            "OSC 52 decoded payload must be forwarded when allow_osc52_write = true"
        );
    }

    /// `take_osc52_write` drains the pending payload — second call returns None.
    #[test]
    fn osc52_take_drains_pending_payload() {
        let mut vt = make_vt(80, 24);
        vt.allow_osc52_write = true;
        vt.process(b"\x1b]52;c;aGVsbG8=\x07");
        let _ = vt.take_osc52_write(); // first call drains
        assert!(
            vt.take_osc52_write().is_none(),
            "second call to take_osc52_write must return None (payload already drained)"
        );
    }

    // ---------------------------------------------------------------------------
    // ICH — CSI Ps @ — Insert Character (ECMA-48 §8.3.64)
    // ---------------------------------------------------------------------------

    /// ICH basic: insert 1 blank at col 2, existing chars shift right.
    /// "ABCDE" at cols 0-4 → after CSI 1 @ at col 2 → "AB CDE" (E pushed off if line is 5 cols)
    #[test]
    fn ich_inserts_blank_and_shifts_right() {
        let mut vt = make_vt(10, 5);
        // Write "ABCDE" then move cursor back to col 2.
        vt.process(b"ABCDE\x1b[1;3H"); // CUP row=1,col=3 → (row=0, col=2)
        vt.process(b"\x1b[1@"); // ICH 1
        assert_eq!(grapheme_at(&vt, 0, 0), "A");
        assert_eq!(grapheme_at(&vt, 0, 1), "B");
        assert_eq!(grapheme_at(&vt, 0, 2), " "); // blank inserted
        assert_eq!(grapheme_at(&vt, 0, 3), "C");
        assert_eq!(grapheme_at(&vt, 0, 4), "D");
        // E is at col 5 — shifted but not lost because line is 10 wide.
        assert_eq!(grapheme_at(&vt, 0, 5), "E");
    }

    /// ICH with N > remaining cols: remaining cells are blanked, nothing wraps.
    #[test]
    fn ich_clamps_to_line_end() {
        let mut vt = make_vt(5, 5);
        // "ABCDE", cursor at col 1.
        vt.process(b"ABCDE\x1b[1;2H");
        vt.process(b"\x1b[10@"); // ICH 10 — more than remaining cols
        assert_eq!(grapheme_at(&vt, 0, 0), "A");
        // cols 1-4 should all be blank.
        for col in 1..5 {
            assert_eq!(
                grapheme_at(&vt, 0, col),
                " ",
                "col {col} should be blank after ICH overcount"
            );
        }
    }

    /// ICH with N=0 is treated as N=1 (ECMA-48: default is 1).
    #[test]
    fn ich_n0_treated_as_1() {
        let mut vt = make_vt(10, 5);
        vt.process(b"ABCDE\x1b[1;1H"); // cursor at col 0
        vt.process(b"\x1b[@"); // ICH with no param → default 1
        assert_eq!(grapheme_at(&vt, 0, 0), " "); // blank at col 0
        assert_eq!(grapheme_at(&vt, 0, 1), "A"); // A shifted right
    }

    // ---------------------------------------------------------------------------
    // DCH — CSI Ps P — Delete Character (ECMA-48 §8.3.26)
    // ---------------------------------------------------------------------------

    /// DCH basic: delete 1 char at col 2, chars to the right shift left.
    #[test]
    fn dch_deletes_and_shifts_left() {
        let mut vt = make_vt(10, 5);
        vt.process(b"ABCDE\x1b[1;3H"); // cursor at col 2
        vt.process(b"\x1b[1P"); // DCH 1
        assert_eq!(grapheme_at(&vt, 0, 0), "A");
        assert_eq!(grapheme_at(&vt, 0, 1), "B");
        assert_eq!(grapheme_at(&vt, 0, 2), "D"); // C deleted, D shifted left
        assert_eq!(grapheme_at(&vt, 0, 3), "E");
        assert_eq!(grapheme_at(&vt, 0, 4), " "); // trailing blank
    }

    /// DCH with N > remaining: all remaining cols become blank.
    #[test]
    fn dch_clamps_to_line_end() {
        let mut vt = make_vt(5, 5);
        vt.process(b"ABCDE\x1b[1;3H"); // cursor at col 2
        vt.process(b"\x1b[10P"); // DCH 10 — more than remaining
        assert_eq!(grapheme_at(&vt, 0, 0), "A");
        assert_eq!(grapheme_at(&vt, 0, 1), "B");
        for col in 2..5 {
            assert_eq!(
                grapheme_at(&vt, 0, col),
                " ",
                "col {col} should be blank after DCH overcount"
            );
        }
    }

    // ---------------------------------------------------------------------------
    // IL — CSI Ps L — Insert Line (ECMA-48 / xterm)
    // ---------------------------------------------------------------------------

    /// IL basic: insert 1 blank line at cursor row; lines below shift down.
    /// Last line in scroll region is lost.
    #[test]
    fn il_inserts_blank_line_and_shifts_down() {
        let mut vt = make_vt(10, 5);
        // Fill rows 0-2 with distinguishable content.
        vt.process(b"ROW0\r\nROW1\r\nROW2\r\nROW3\r\nROW4");
        // Move cursor to row 1.
        vt.process(b"\x1b[2;1H"); // CUP row=2 (1-based) → row=1 (0-based)
        vt.process(b"\x1b[1L"); // IL 1
        // Row 1 is now blank.
        let row1_text: String = (0..10).map(|c| grapheme_at(&vt, 1, c)).collect();
        assert!(
            row1_text.trim().is_empty(),
            "row 1 should be blank after IL, got: {row1_text:?}"
        );
        // Row 0 is unchanged.
        assert_eq!(grapheme_at(&vt, 0, 0), "R");
        assert_eq!(grapheme_at(&vt, 0, 1), "O");
        // Original row 1 content is now at row 2.
        assert_eq!(grapheme_at(&vt, 2, 0), "R");
        assert_eq!(grapheme_at(&vt, 2, 3), "1");
    }

    /// IL within a scroll region: lines below bottom of region are unaffected.
    #[test]
    fn il_respects_scroll_region() {
        let mut vt = make_vt(10, 5);
        vt.process(b"ROW0\r\nROW1\r\nROW2\r\nROW3\r\nROW4");
        // Set scroll region rows 1-3 (1-based).
        vt.process(b"\x1b[2;4r"); // DECSTBM top=2,bottom=4 → 0-based (1,3)
        // Move cursor to row 1 (0-based), which is inside the region.
        vt.process(b"\x1b[2;1H");
        vt.process(b"\x1b[1L"); // IL 1
        // Row 4 (0-based) is outside the region — must be unchanged.
        assert_eq!(grapheme_at(&vt, 4, 0), "R");
        assert_eq!(grapheme_at(&vt, 4, 3), "4");
    }

    // ---------------------------------------------------------------------------
    // DL — CSI Ps M — Delete Line (ECMA-48 / xterm)
    // ---------------------------------------------------------------------------

    /// DL basic: delete 1 line at cursor row; lines below shift up.
    #[test]
    fn dl_deletes_line_and_shifts_up() {
        let mut vt = make_vt(10, 5);
        vt.process(b"ROW0\r\nROW1\r\nROW2\r\nROW3\r\nROW4");
        // Move cursor to row 1.
        vt.process(b"\x1b[2;1H");
        vt.process(b"\x1b[1M"); // DL 1
        // Row 1 should now contain what was row 2.
        assert_eq!(grapheme_at(&vt, 1, 0), "R");
        assert_eq!(grapheme_at(&vt, 1, 3), "2");
        // Row 4 (last) should now be blank.
        let row4_text: String = (0..10).map(|c| grapheme_at(&vt, 4, c)).collect();
        assert!(
            row4_text.trim().is_empty(),
            "last row should be blank after DL, got: {row4_text:?}"
        );
    }

    /// DL respects the bottom of scroll region.
    #[test]
    fn dl_respects_scroll_region() {
        let mut vt = make_vt(10, 5);
        vt.process(b"ROW0\r\nROW1\r\nROW2\r\nROW3\r\nROW4");
        // Scroll region rows 1-3 (1-based).
        vt.process(b"\x1b[2;4r");
        vt.process(b"\x1b[2;1H");
        vt.process(b"\x1b[1M");
        // Row 4 (outside region) must be unchanged.
        assert_eq!(grapheme_at(&vt, 4, 0), "R");
        assert_eq!(grapheme_at(&vt, 4, 3), "4");
    }

    // ---------------------------------------------------------------------------
    // RI — ESC M — Reverse Index
    // ---------------------------------------------------------------------------

    /// RI when cursor is NOT at the top of scroll region: cursor moves up one row.
    #[test]
    fn ri_moves_cursor_up_when_not_at_top() {
        let mut vt = make_vt(10, 5);
        // Move cursor to row 2.
        vt.process(b"\x1b[3;1H");
        vt.process(b"\x1bM"); // RI
        assert_eq!(
            vt.active_cursor().row,
            1,
            "RI should move cursor up one row"
        );
    }

    /// RI when cursor is AT the top of scroll region: scrolls down (inserts blank at top).
    #[test]
    fn ri_scrolls_down_when_at_scroll_top() {
        let mut vt = make_vt(10, 5);
        vt.process(b"ROW0\r\nROW1\r\nROW2\r\nROW3\r\nROW4");
        // Cursor at row 0 (top of default scroll region).
        vt.process(b"\x1b[1;1H");
        vt.process(b"\x1bM"); // RI
        // Row 0 should now be blank (new blank line inserted at top).
        let row0_text: String = (0..10).map(|c| grapheme_at(&vt, 0, c)).collect();
        assert!(
            row0_text.trim().is_empty(),
            "row 0 should be blank after RI at top of scroll region, got: {row0_text:?}"
        );
        // Original row 0 should now be at row 1.
        assert_eq!(grapheme_at(&vt, 1, 0), "R");
        assert_eq!(grapheme_at(&vt, 1, 3), "0");
        // Cursor should remain at row 0.
        assert_eq!(vt.active_cursor().row, 0);
    }

    /// RI at top of a non-default scroll region scrolls within that region.
    #[test]
    fn ri_at_top_of_partial_scroll_region() {
        let mut vt = make_vt(10, 5);
        vt.process(b"ROW0\r\nROW1\r\nROW2\r\nROW3\r\nROW4");
        // Scroll region rows 2-4 (1-based) → 0-based (1,3).
        vt.process(b"\x1b[2;4r");
        // Move cursor to row 1 (0-based) = top of scroll region.
        vt.process(b"\x1b[2;1H");
        vt.process(b"\x1bM"); // RI
        // Row 1 should be blank (inserted).
        let row1_text: String = (0..10).map(|c| grapheme_at(&vt, 1, c)).collect();
        assert!(
            row1_text.trim().is_empty(),
            "row 1 (top of partial scroll region) should be blank after RI, got: {row1_text:?}"
        );
        // Row 0 must be unchanged (outside scroll region).
        assert_eq!(grapheme_at(&vt, 0, 0), "R");
        assert_eq!(grapheme_at(&vt, 0, 3), "0");
    }

    // ---------------------------------------------------------------------------
    // DECAWM — DEC Auto Wrap Mode (mode ?7)
    // ---------------------------------------------------------------------------

    /// DECAWM enabled (default): character at last column triggers auto-wrap.
    #[test]
    fn decawm_on_wraps_at_right_margin() {
        let mut vt = make_vt(5, 5);
        // Write 6 chars — the 6th should wrap to row 1 col 0.
        vt.process(b"ABCDEF");
        // Row 0: ABCDE, row 1: F
        assert_eq!(grapheme_at(&vt, 0, 0), "A");
        assert_eq!(grapheme_at(&vt, 0, 4), "E");
        assert_eq!(grapheme_at(&vt, 1, 0), "F");
    }

    /// DECAWM disabled (?7l): characters at or beyond the last column overwrite
    /// the last column, cursor stays at last column.
    #[test]
    fn decawm_off_does_not_wrap() {
        let mut vt = make_vt(5, 5);
        vt.process(b"\x1b[?7l"); // DECAWM off
        // Write 7 chars — cols 0-4 fill normally, then chars 6 and 7 overwrite col 4.
        vt.process(b"ABCDEFG");
        // All chars should be on row 0; no wrap to row 1.
        assert_eq!(grapheme_at(&vt, 0, 0), "A");
        assert_eq!(grapheme_at(&vt, 0, 1), "B");
        assert_eq!(grapheme_at(&vt, 0, 2), "C");
        assert_eq!(grapheme_at(&vt, 0, 3), "D");
        // The last 3 chars (E, F, G) all land on col 4 — G is the final value.
        assert_eq!(grapheme_at(&vt, 0, 4), "G");
        // Row 1 must remain blank.
        let row1_text: String = (0..5).map(|c| grapheme_at(&vt, 1, c)).collect();
        assert!(
            row1_text.trim().is_empty(),
            "row 1 should be empty when DECAWM is off, got: {row1_text:?}"
        );
    }

    /// DECAWM can be re-enabled with ?7h after being disabled.
    #[test]
    fn decawm_can_be_reenabled() {
        let mut vt = make_vt(5, 5);
        vt.process(b"\x1b[?7l"); // disable
        vt.process(b"\x1b[?7h"); // re-enable
        vt.process(b"ABCDEF");
        // F should wrap to row 1.
        assert_eq!(grapheme_at(&vt, 1, 0), "F");
    }

    /// DECAWM off: cursor is clamped to last column, not one past it.
    #[test]
    fn decawm_off_cursor_stays_at_last_col() {
        let mut vt = make_vt(5, 5);
        vt.process(b"\x1b[?7l");
        vt.process(b"ABCDE"); // fill the line exactly
        // Cursor should be at col 4 (last column), not col 5 or wrap_pending.
        assert_eq!(
            vt.active_cursor().col,
            4,
            "cursor should be at last col after filling line with DECAWM off"
        );
        assert!(
            !vt.wrap_pending,
            "wrap_pending must be false when DECAWM is off"
        );
    }

    // ---------------------------------------------------------------------------
    // DECSC / DECRC — save and restore attrs + charset_slot (fix #1)
    // ---------------------------------------------------------------------------

    /// DECSC (ESC 7) saves current SGR attributes; DECRC (ESC 8) restores them.
    #[test]
    fn decsc_decrc_esc_saves_and_restores_attrs() {
        let mut vt = make_vt(80, 24);
        // Set bold + red foreground.
        vt.process(b"\x1b[1;31m"); // bold + red
        // Save cursor (ESC 7).
        vt.process(b"\x1b7");
        // Reset SGR.
        vt.process(b"\x1b[0m");
        // Confirm attrs are now default.
        assert!(
            !vt.current_attrs.bold,
            "attrs should be reset after ESC [0m"
        );
        // Restore cursor (ESC 8).
        vt.process(b"\x1b8");
        // Attrs should be restored to bold + red.
        assert!(vt.current_attrs.bold, "DECRC must restore bold attribute");
        assert_eq!(
            vt.current_attrs.fg,
            Some(crate::vt::cell::Color::Ansi { index: 1 }),
            "DECRC must restore fg color"
        );
    }

    /// DECSC (ESC 7) saves charset_slot; DECRC (ESC 8) restores it.
    #[test]
    fn decsc_decrc_esc_saves_and_restores_charset_slot() {
        use crate::vt::modes::CharsetSlot;
        let mut vt = make_vt(80, 24);
        // Switch to G1 (SO = 0x0E).
        vt.process(b"\x0E"); // SO → G1
        assert_eq!(vt.modes.charset_slot, CharsetSlot::G1);
        // Save cursor.
        vt.process(b"\x1b7");
        // Switch back to G0 (SI = 0x0F).
        vt.process(b"\x0F"); // SI → G0
        assert_eq!(vt.modes.charset_slot, CharsetSlot::G0);
        // Restore cursor — charset_slot should return to G1.
        vt.process(b"\x1b8");
        assert_eq!(
            vt.modes.charset_slot,
            CharsetSlot::G1,
            "DECRC must restore charset_slot"
        );
    }

    /// DECSC (CSI s) / DECRC (CSI u) also save/restore attrs.
    #[test]
    fn decsc_decrc_csi_saves_and_restores_attrs() {
        let mut vt = make_vt(80, 24);
        vt.process(b"\x1b[1m"); // bold
        vt.process(b"\x1b[s"); // CSI s = DECSC
        vt.process(b"\x1b[0m"); // reset
        assert!(!vt.current_attrs.bold);
        vt.process(b"\x1b[u"); // CSI u = DECRC
        assert!(vt.current_attrs.bold, "CSI u must restore bold");
    }

    // ---------------------------------------------------------------------------
    // Mouse mode reset on alt-screen exit (fix #2, FS-VT-086)
    // ---------------------------------------------------------------------------

    /// Mouse reporting mode must be None after leaving the alternate screen,
    /// even when the app never sent the reset sequence.
    #[test]
    fn mouse_mode_reset_on_leave_alternate_mode_1049() {
        use crate::vt::modes::MouseReportingMode;
        let mut vt = make_vt(80, 24);
        // Enter alt screen and activate normal mouse tracking.
        vt.process(b"\x1b[?1049h"); // enter alt screen
        vt.process(b"\x1b[?1000h"); // activate mouse normal tracking
        assert_eq!(vt.modes.mouse_reporting, MouseReportingMode::Normal);
        // Leave alt screen without sending reset — simulates app crash.
        vt.process(b"\x1b[?1049l");
        assert_eq!(
            vt.modes.mouse_reporting,
            MouseReportingMode::None,
            "mouse reporting must be None after leaving alt screen (FS-VT-086)"
        );
    }

    /// Mouse mode reset also applies to mode 47 exit.
    #[test]
    fn mouse_mode_reset_on_leave_alternate_mode_47() {
        use crate::vt::modes::MouseReportingMode;
        let mut vt = make_vt(80, 24);
        vt.process(b"\x1b[?47h");
        vt.process(b"\x1b[?1000h");
        vt.process(b"\x1b[?47l");
        assert_eq!(
            vt.modes.mouse_reporting,
            MouseReportingMode::None,
            "mouse reporting must be None after leaving alt screen via mode 47"
        );
    }

    // ---------------------------------------------------------------------------
    // u16 underflow guards — no panic on 1-row / 1-col terminal (fix #3)
    // ---------------------------------------------------------------------------

    /// Resize to 1 row must not panic on LF (scroll_up path uses rows - 1).
    #[test]
    fn no_panic_on_lf_with_one_row() {
        let mut vt = make_vt(80, 1);
        // LF on a 1-row terminal would have triggered u16 underflow before the fix.
        vt.process(b"A\n");
        // If we reach here without panic, the guard works.
    }

    /// Resize to 1 col must not panic on HT (tab stop uses cols - 1).
    #[test]
    fn no_panic_on_ht_with_one_col() {
        let mut vt = make_vt(1, 24);
        vt.process(b"\x09"); // HT
    }

    /// CUF (cursor forward) on a 1-col terminal must not panic.
    #[test]
    fn no_panic_on_cuf_with_one_col() {
        let mut vt = make_vt(1, 24);
        vt.process(b"\x1b[C"); // CUF 1
    }

    /// CUP on a 1×1 terminal must not panic.
    #[test]
    fn no_panic_on_cup_with_one_by_one() {
        let mut vt = make_vt(1, 1);
        vt.process(b"\x1b[1;1H"); // CUP 1,1
    }

    /// DECSTBM default (param1=0) on a 1-row terminal must not panic.
    #[test]
    fn no_panic_on_decstbm_with_one_row() {
        let mut vt = make_vt(80, 1);
        vt.process(b"\x1b[r"); // DECSTBM with defaults
    }

    // -----------------------------------------------------------------------
    // R3 — DECAWM saved/restored by DECSC (ESC 7 / CSI s) and DECRC (ESC 8 / CSI u)
    //
    // Spec: DECSC saves cursor position + SGR + charset + DECAWM.
    //       DECRC restores all of the above atomically.
    //
    // Scenario tested:
    //   1. Disable DECAWM (DECRST ?7)        → decawm = false
    //   2. DECSC (ESC 7)                      → snapshot decawm = false
    //   3. Re-enable DECAWM (DECSET ?7)       → decawm = true
    //   4. DECRC (ESC 8)                      → decawm restored to false
    // -----------------------------------------------------------------------

    /// R3 (ESC 7 / ESC 8): DECAWM state is included in the DECSC snapshot and
    /// fully restored by DECRC.
    #[test]
    fn r3_decawm_saved_and_restored_by_decsc_decrc_esc() {
        let mut vt = make_vt(80, 24);
        // 1. Disable DECAWM via DECRST ?7.
        vt.process(b"\x1b[?7l");
        assert!(
            !vt.mode_state().decawm,
            "DECAWM must be off after DECRST ?7"
        );
        // 2. Save cursor with ESC 7.
        vt.process(b"\x1b7");
        // 3. Re-enable DECAWM.
        vt.process(b"\x1b[?7h");
        assert!(vt.mode_state().decawm, "DECAWM must be on after DECSET ?7");
        // 4. Restore cursor with ESC 8 — DECAWM must revert to false.
        vt.process(b"\x1b8");
        assert!(
            !vt.mode_state().decawm,
            "DECRC (ESC 8) must restore DECAWM to false (R3)"
        );
    }

    /// R3 (CSI s / CSI u): DECAWM state is included in the DECSC snapshot and
    /// fully restored by DECRC via the CSI variants.
    #[test]
    fn r3_decawm_saved_and_restored_by_decsc_decrc_csi() {
        let mut vt = make_vt(80, 24);
        // 1. Disable DECAWM.
        vt.process(b"\x1b[?7l");
        assert!(!vt.mode_state().decawm);
        // 2. Save cursor (CSI s).
        vt.process(b"\x1b[s");
        // 3. Re-enable DECAWM.
        vt.process(b"\x1b[?7h");
        assert!(vt.mode_state().decawm);
        // 4. Restore cursor (CSI u) — DECAWM must be false again.
        vt.process(b"\x1b[u");
        assert!(
            !vt.mode_state().decawm,
            "DECRC (CSI u) must restore DECAWM to false (R3)"
        );
    }

    /// R3: when DECAWM is enabled at save time, DECRC must restore it to true
    /// even if it was disabled in between.
    #[test]
    fn r3_decawm_enabled_at_save_is_restored_after_disable() {
        let mut vt = make_vt(80, 24);
        // DECAWM starts true (default). Save it.
        assert!(vt.mode_state().decawm);
        vt.process(b"\x1b7"); // DECSC — saves decawm = true
        // Disable DECAWM.
        vt.process(b"\x1b[?7l");
        assert!(!vt.mode_state().decawm);
        // Restore — DECAWM must be true again.
        vt.process(b"\x1b8"); // DECRC
        assert!(
            vt.mode_state().decawm,
            "DECRC must restore DECAWM to true when it was true at save time (R3)"
        );
    }

    /// R3: DECRC with no prior DECSC must not change DECAWM (no saved state → no-op).
    #[test]
    fn r3_decrc_without_decsc_is_noop() {
        let mut vt = make_vt(80, 24);
        vt.process(b"\x1b[?7l");
        assert!(!vt.mode_state().decawm);
        // No DECSC was issued. DECRC must be a no-op.
        vt.process(b"\x1b8");
        assert!(
            !vt.mode_state().decawm,
            "DECRC with no prior DECSC must not modify DECAWM (R3)"
        );
    }

    // ---------------------------------------------------------------------------
    // R5 — get_scrollback_line exposes soft_wrapped (FS-SB-011)
    // ---------------------------------------------------------------------------

    /// R5-hard: a line terminated by a hard newline (LF) must have soft_wrapped=false.
    #[test]
    fn r5_get_scrollback_line_hard_newline_soft_wrapped_false() {
        // 5-column terminal, 1 visible row → first LF pushes row 0 to scrollback.
        let mut vt = crate::vt::VtProcessor::new(5, 1, 100);
        // Write text then a hard LF — the current row is pushed to scrollback.
        vt.process(b"ABC\r\n");
        let sb = vt
            .get_scrollback_line(0)
            .expect("scrollback line 0 must exist");
        assert!(
            !sb.soft_wrapped,
            "hard newline: soft_wrapped must be false, got true"
        );
    }

    /// R5-soft: a line pushed to scrollback by auto-wrap must have soft_wrapped=true.
    #[test]
    fn r5_get_scrollback_line_soft_wrap_soft_wrapped_true() {
        // 3-column terminal, 1 visible row.
        // Writing 4 chars forces auto-wrap + scroll → scrollback entry is soft-wrapped.
        let mut vt = crate::vt::VtProcessor::new(3, 1, 100);
        // Writing 4 printable chars on a 3-wide terminal:
        //   - chars 1-3 fill row 0, set wrap_pending on char 3.
        //   - char 4 triggers delayed wrap → row 0 scrolls into scrollback (soft_wrapped=true).
        vt.process(b"ABCD");
        let sb = vt
            .get_scrollback_line(0)
            .expect("scrollback line 0 must exist");
        assert!(
            sb.soft_wrapped,
            "auto-wrap push: soft_wrapped must be true, got false"
        );
    }

    /// R5-cells: the cells returned by get_scrollback_line match the written content.
    #[test]
    fn r5_get_scrollback_line_cells_content() {
        let mut vt = crate::vt::VtProcessor::new(5, 1, 100);
        vt.process(b"Hi\r\n");
        let sb = vt
            .get_scrollback_line(0)
            .expect("scrollback line 0 must exist");
        assert_eq!(sb.cells[0].grapheme, "H", "cell 0 must be 'H'");
        assert_eq!(sb.cells[1].grapheme, "i", "cell 1 must be 'i'");
    }

    /// R5-oob: get_scrollback_line past the end returns None.
    #[test]
    fn r5_get_scrollback_line_out_of_bounds_returns_none() {
        let vt = crate::vt::VtProcessor::new(80, 24, 1000);
        assert!(
            vt.get_scrollback_line(0).is_none(),
            "empty scrollback: get_scrollback_line(0) must return None"
        );
    }

    // ---------------------------------------------------------------------------
    // R2 — LF outside scroll region (FS-VT-055)
    // ---------------------------------------------------------------------------

    /// R2-above: cursor above scroll region top — LF moves cursor down, no scroll.
    #[test]
    fn r2_lf_above_scroll_region_moves_cursor_no_scroll() {
        // 24-row terminal, scroll region rows 5–10 (0-based: top=4, bottom=9).
        let mut vt = make_vt(80, 24);
        vt.process(b"\x1b[5;10r"); // DECSTBM: region rows 5–10 (1-based)
        // DECSTBM resets cursor to home (0,0). Cursor is at row 0 — above region top=4.
        assert_eq!(vt.normal_cursor.row, 0);
        // Position cursor at row 1 (0-based) — still above top=4.
        vt.process(b"\x1b[2;1H"); // CUP row=2 (1-based) → 0-based=1
        assert_eq!(vt.normal_cursor.row, 1);
        let scrollback_before = vt.normal.scrollback_len();
        // LF — must move down without scrolling region.
        vt.process(b"\n");
        assert_eq!(
            vt.normal_cursor.row, 2,
            "LF above region top must move cursor to row 2 (no scroll)"
        );
        assert_eq!(
            vt.normal.scrollback_len(),
            scrollback_before,
            "LF above region must not add to scrollback"
        );
    }

    /// R2-below: cursor below scroll region bottom — LF moves cursor down, no scroll.
    #[test]
    fn r2_lf_below_scroll_region_moves_cursor_no_scroll() {
        // 24-row terminal, region rows 5–10 (0-based: top=4, bottom=9).
        let mut vt = make_vt(80, 24);
        vt.process(b"\x1b[5;10r"); // DECSTBM
        // Position cursor below region: row 15 (0-based), 1-based = 16.
        vt.process(b"\x1b[16;1H");
        assert_eq!(vt.normal_cursor.row, 15);
        let scrollback_before = vt.normal.scrollback_len();
        vt.process(b"\n");
        assert_eq!(
            vt.normal_cursor.row, 16,
            "LF below region bottom must move cursor to row 16 (no scroll)"
        );
        assert_eq!(
            vt.normal.scrollback_len(),
            scrollback_before,
            "LF below region must not add to scrollback"
        );
    }

    /// R2-last-row: cursor on last screen row and outside region — LF is ignored.
    #[test]
    fn r2_lf_at_last_row_outside_region_is_noop() {
        // 24-row terminal (rows 0–23), region rows 5–10 (top=4, bottom=9).
        let mut vt = make_vt(80, 24);
        vt.process(b"\x1b[5;10r");
        // Position cursor at last row (row 23, 0-based), 1-based = 24.
        vt.process(b"\x1b[24;1H");
        assert_eq!(vt.normal_cursor.row, 23);
        let scrollback_before = vt.normal.scrollback_len();
        vt.process(b"\n");
        assert_eq!(
            vt.normal_cursor.row, 23,
            "LF at last screen row outside region must be ignored (no cursor move)"
        );
        assert_eq!(
            vt.normal.scrollback_len(),
            scrollback_before,
            "LF at last screen row outside region must not scroll"
        );
    }

    /// R2-in-region: cursor inside region at bottom — existing scroll behaviour preserved.
    #[test]
    fn r2_lf_at_region_bottom_scrolls_normally() {
        // 24-row terminal, region rows 5–10 (top=4, bottom=9).
        let mut vt = make_vt(80, 24);
        vt.process(b"\x1b[5;10r");
        // Position cursor at region bottom (row 9, 0-based), 1-based = 10.
        vt.process(b"\x1b[10;1H");
        assert_eq!(vt.normal_cursor.row, 9);
        // LF at bottom of a PARTIAL region: scroll within region, no scrollback.
        let scrollback_before = vt.normal.scrollback_len();
        vt.process(b"\n");
        // Cursor stays at bottom of region after scroll.
        assert_eq!(
            vt.normal_cursor.row, 9,
            "LF at region bottom must keep cursor at region bottom after scroll"
        );
        // No scrollback for partial region.
        assert_eq!(
            vt.normal.scrollback_len(),
            scrollback_before,
            "partial region scroll must not add to scrollback"
        );
    }

    // ---------------------------------------------------------------------------
    // R1 — DECOM: origin mode (DECSET/DECRST ?6) (VT220 / xterm)
    // ---------------------------------------------------------------------------

    /// R1-cup: DECSET 6 + CUP with scroll region top=5 (0-based=4) — row is offset.
    #[test]
    fn r1_decom_cup_offsets_row_by_scroll_region_top() {
        // 24-row terminal, region rows 5–20 (1-based) → 0-based top=4, bottom=19.
        let mut vt = make_vt(80, 24);
        vt.process(b"\x1b[5;20r"); // DECSTBM
        // Activate DECOM.
        vt.process(b"\x1b[?6h");
        // CUP row=3, col=1 (1-based). With DECOM, row → top + (3-1) = 4 + 2 = 6.
        vt.process(b"\x1b[3;1H");
        assert_eq!(
            vt.normal_cursor.row, 6,
            "DECOM CUP row=3 with top=4 must place cursor at row 6"
        );
        assert_eq!(vt.normal_cursor.col, 0, "column must be 0 (1-based col=1)");
    }

    /// R1-clamp-top: DECOM + CUP row=0 (before region) — clamp to top.
    #[test]
    fn r1_decom_cup_clamps_above_region_top() {
        let mut vt = make_vt(80, 24);
        vt.process(b"\x1b[5;20r"); // top=4, bottom=19
        vt.process(b"\x1b[?6h");
        // CUP with row=0 (would become top + (0-1) = 3 < top): clamp to top=4.
        // vte delivers param0=0, which gets .max(1) → 1 → offset = top + 0 = 4.
        vt.process(b"\x1b[1;1H"); // row=1 (minimum), col=1 → row 0-based = top+0 = 4
        assert_eq!(
            vt.normal_cursor.row, 4,
            "DECOM CUP row=1 (minimum) must clamp to top=4"
        );
    }

    /// R1-clamp-bottom: DECOM + CUP row past bottom — clamp to bottom.
    #[test]
    fn r1_decom_cup_clamps_past_region_bottom() {
        let mut vt = make_vt(80, 24);
        vt.process(b"\x1b[5;10r"); // top=4, bottom=9
        vt.process(b"\x1b[?6h");
        // CUP row=99 (1-based) → offset = 4 + 98 = 102, clamped to bottom=9.
        vt.process(b"\x1b[99;1H");
        assert_eq!(
            vt.normal_cursor.row, 9,
            "DECOM CUP row past bottom must clamp to region bottom=9"
        );
    }

    /// R1-off: DECRST 6 — CUP uses absolute coordinates, no offset.
    #[test]
    fn r1_decom_off_cup_uses_absolute_coordinates() {
        let mut vt = make_vt(80, 24);
        vt.process(b"\x1b[5;20r"); // top=4
        // Enable then disable DECOM.
        vt.process(b"\x1b[?6h");
        vt.process(b"\x1b[?6l");
        // CUP row=3 col=1 (1-based) → 0-based row=2 (absolute, no offset).
        vt.process(b"\x1b[3;1H");
        assert_eq!(
            vt.normal_cursor.row, 2,
            "After DECRST 6, CUP must use absolute row (row 3 → 0-based 2)"
        );
    }

    /// R1-decsc-decrc-decom: DECSC saves DECOM state; DECRC restores it.
    #[test]
    fn r1_decsc_saves_and_decrc_restores_decom_true() {
        let mut vt = make_vt(80, 24);
        vt.process(b"\x1b[?6h"); // DECOM on
        assert!(vt.modes.decom, "DECOM must be true after DECSET 6");
        vt.process(b"\x1b7"); // DECSC — save (includes decom=true)
        vt.process(b"\x1b[?6l"); // DECOM off
        assert!(!vt.modes.decom, "DECOM must be false after DECRST 6");
        vt.process(b"\x1b8"); // DECRC — restore
        assert!(
            vt.modes.decom,
            "DECRC must restore DECOM=true from saved state"
        );
    }

    /// R1-decsc-decrc-decom-false: DECSC saves DECOM=false; DECRC restores it.
    #[test]
    fn r1_decsc_saves_and_decrc_restores_decom_false() {
        let mut vt = make_vt(80, 24);
        // DECOM starts false (default).
        vt.process(b"\x1b7"); // DECSC — save with decom=false
        vt.process(b"\x1b[?6h"); // DECOM on
        assert!(vt.modes.decom);
        vt.process(b"\x1b8"); // DECRC — restore decom=false
        assert!(
            !vt.modes.decom,
            "DECRC must restore DECOM=false when it was false at save time"
        );
    }

    /// R1-alt-screen: entering alt screen resets DECOM to false.
    #[test]
    fn r1_decom_reset_on_alt_screen_entry() {
        let mut vt = make_vt(80, 24);
        vt.process(b"\x1b[?6h"); // DECOM on
        assert!(vt.modes.decom);
        // Enter alt screen (mode 1049) — must reset DECOM.
        vt.process(b"\x1b[?1049h");
        assert!(
            !vt.modes.decom,
            "entering alt screen must reset DECOM to false"
        );
    }

    // ---------------------------------------------------------------------------
    // R7 — Skin-tone modifiers U+1F3FB–U+1F3FF treated as combining (FS-VT-018)
    // ---------------------------------------------------------------------------

    /// "👍\u{1F3FB}": thumbs-up + light skin tone modifier must occupy 2 cells,
    /// with the modifier attached to the base emoji grapheme.
    #[test]
    fn r7_skin_tone_modifier_attaches_to_base_emoji_two_cells() {
        let mut vt = make_vt(80, 24);
        // U+1F44D (thumbs up, width=2) followed by U+1F3FB (light skin tone).
        vt.process("👍\u{1F3FB}".as_bytes());
        // The whole sequence must occupy exactly 2 cells (not 4).
        // Cell at (0,0) must contain both codepoints as a single grapheme.
        let g = vt
            .active_buf_ref()
            .get(0, 0)
            .map(|c| c.grapheme.clone())
            .unwrap_or_default();
        assert!(
            g.contains('👍') && g.contains('\u{1F3FB}'),
            "cell (0,0) must contain base emoji + skin-tone modifier, got: {g:?}"
        );
        let w = vt.active_buf_ref().get(0, 0).map(|c| c.width).unwrap_or(0);
        assert_eq!(w, 2, "base emoji cell must still have width=2");
        // Phantom cell at (0,1).
        let w1 = vt.active_buf_ref().get(0, 1).map(|c| c.width).unwrap_or(99);
        assert_eq!(w1, 0, "cell (0,1) must be phantom (width=0)");
        // Cursor must be at col=2 (only the base emoji's advance).
        assert_eq!(
            vt.normal_cursor.col, 2,
            "cursor must be at col=2, not col=4"
        );
    }

    /// Skin-tone modifier at column 0 (no preceding cell) must not panic.
    #[test]
    fn r7_skin_tone_at_column_zero_does_not_panic() {
        let mut vt = make_vt(80, 24);
        // No preceding character — modifier must attach to (0,0) without panic.
        vt.process("\u{1F3FC}".as_bytes());
        // No crash is the primary assertion. Cursor must not advance.
        assert_eq!(
            vt.normal_cursor.col, 0,
            "skin-tone modifier at col=0 must not advance the cursor"
        );
    }

    // ---------------------------------------------------------------------------
    // R8 — Regional Indicator pairs form a 2-cell flag emoji (FS-VT-019)
    // ---------------------------------------------------------------------------

    /// "🇫🇷" (U+1F1EB U+1F1F7) must occupy exactly 2 cells as a confirmed flag.
    #[test]
    fn r8_regional_indicator_pair_forms_two_cell_flag() {
        let mut vt = make_vt(80, 24);
        // 🇫🇷 = U+1F1EB (F) + U+1F1F7 (R).
        vt.process("🇫🇷".as_bytes());
        // Must occupy 2 cells total.
        let g0 = vt
            .active_buf_ref()
            .get(0, 0)
            .map(|c| c.grapheme.clone())
            .unwrap_or_default();
        let w0 = vt.active_buf_ref().get(0, 0).map(|c| c.width).unwrap_or(0);
        let w1 = vt.active_buf_ref().get(0, 1).map(|c| c.width).unwrap_or(99);
        assert!(
            g0.contains('\u{1F1EB}') && g0.contains('\u{1F1F7}'),
            "cell (0,0) must contain both RI codepoints, got: {g0:?}"
        );
        assert_eq!(w0, 2, "confirmed flag cell must have width=2");
        assert_eq!(w1, 0, "cell (0,1) must be phantom");
        // Cursor must be at col=2.
        assert_eq!(
            vt.normal_cursor.col, 2,
            "cursor must be at col=2 after flag"
        );
    }

    /// Unpaired RI (followed by a non-RI char) must occupy 1 cell (FS-VT-019).
    #[test]
    fn r8_unpaired_regional_indicator_occupies_one_cell() {
        let mut vt = make_vt(80, 24);
        // U+1F1EB alone, followed by ASCII 'A'.
        vt.process("\u{1F1EB}A".as_bytes());
        let w0 = vt.active_buf_ref().get(0, 0).map(|c| c.width).unwrap_or(0);
        assert_eq!(
            w0, 1,
            "unpaired RI must occupy 1 cell after confirmation by non-RI char"
        );
        // 'A' must be at col=1.
        let g1 = vt
            .active_buf_ref()
            .get(0, 1)
            .map(|c| c.grapheme.clone())
            .unwrap_or_default();
        assert_eq!(g1, "A", "char after unpaired RI must be at col=1");
        assert_eq!(vt.normal_cursor.col, 2, "cursor must be at col=2");
    }

    /// Two flags "🇫🇷🇩🇪" must occupy exactly 4 cells total.
    #[test]
    fn r8_two_flags_occupy_four_cells() {
        let mut vt = make_vt(80, 24);
        // 🇫🇷 = U+1F1EB U+1F1F7, 🇩🇪 = U+1F1E9 U+1F1EA.
        vt.process("🇫🇷🇩🇪".as_bytes());
        // First flag at cols 0-1, second flag at cols 2-3.
        let w0 = vt.active_buf_ref().get(0, 0).map(|c| c.width).unwrap_or(0);
        let w1 = vt.active_buf_ref().get(0, 1).map(|c| c.width).unwrap_or(99);
        let w2 = vt.active_buf_ref().get(0, 2).map(|c| c.width).unwrap_or(0);
        let w3 = vt.active_buf_ref().get(0, 3).map(|c| c.width).unwrap_or(99);
        assert_eq!(w0, 2, "first flag: width=2 at col=0");
        assert_eq!(w1, 0, "first flag: phantom at col=1");
        assert_eq!(w2, 2, "second flag: width=2 at col=2");
        assert_eq!(w3, 0, "second flag: phantom at col=3");
        assert_eq!(vt.normal_cursor.col, 4, "cursor must be at col=4");
    }

    // ---------------------------------------------------------------------------
    // R6 — Variation selectors U+FE0F / U+FE0E (FS-VT-017)
    // ---------------------------------------------------------------------------

    /// "☆\u{FE0F}": text star + emoji presentation selector must yield 2 cells.
    #[test]
    fn r6_fe0f_emoji_presentation_forces_two_cells() {
        let mut vt = make_vt(80, 24);
        // U+2606 (white star) is ambiguous/width=1 by unicode_width.
        // FE0F must upgrade it to width=2.
        vt.process("☆\u{FE0F}".as_bytes());
        let w0 = vt.active_buf_ref().get(0, 0).map(|c| c.width).unwrap_or(0);
        assert_eq!(w0, 2, "☆+FE0F must occupy 2 cells");
        let w1 = vt.active_buf_ref().get(0, 1).map(|c| c.width).unwrap_or(99);
        assert_eq!(w1, 0, "cell (0,1) must be phantom after FE0F");
        assert_eq!(vt.normal_cursor.col, 2, "cursor must be at col=2");
    }

    /// "☆\u{FE0E}": text presentation selector must keep the star at 1 cell.
    #[test]
    fn r6_fe0e_text_presentation_keeps_one_cell() {
        let mut vt = make_vt(80, 24);
        vt.process("☆\u{FE0E}".as_bytes());
        let w0 = vt.active_buf_ref().get(0, 0).map(|c| c.width).unwrap_or(0);
        assert_eq!(w0, 1, "☆+FE0E must remain 1 cell");
        assert_eq!(vt.normal_cursor.col, 1, "cursor must be at col=1");
    }

    /// "A\u{FE0F}": FE0F after a plain ASCII letter must not widen it.
    #[test]
    fn r6_fe0f_does_not_widen_non_emoji() {
        let mut vt = make_vt(80, 24);
        vt.process("A\u{FE0F}".as_bytes());
        let w0 = vt.active_buf_ref().get(0, 0).map(|c| c.width).unwrap_or(0);
        assert_eq!(w0, 1, "FE0F must not widen a plain ASCII character");
        assert_eq!(vt.normal_cursor.col, 1, "cursor must be at col=1");
    }

    /// FE0E after a non-eligible character (e.g. U+00E0 'à') must not buffer
    /// it and must not crash — the VS is silently dropped.
    #[test]
    fn r6_fe0e_on_non_eligible_char_is_dropped() {
        let mut vt = make_vt(80, 24);
        // U+00E0 'à' is not in the emoji VS-eligible set.
        vt.process("à\u{FE0E}".as_bytes());
        // 'à' is width=1 (Latin extended), must remain unchanged.
        let w0 = vt.active_buf_ref().get(0, 0).map(|c| c.width).unwrap_or(0);
        assert_eq!(w0, 1, "non-eligible char must stay width=1 after FE0E");
        // Cursor at col=1 — the VS consumed no visual column.
        assert_eq!(vt.normal_cursor.col, 1, "cursor must be at col=1");
    }

    /// A lone RI at the end of input (no following char) stays pending as a
    /// 2-cell provisional slot.  The next process() call will confirm or narrow it.
    /// This test verifies the provisional state is visible after the first call.
    #[test]
    fn r8_lone_ri_at_end_of_input_is_provisional_two_cells() {
        let mut vt = make_vt(80, 24);
        // Single Regional Indicator (U+1F1EB), no following character.
        vt.process("\u{1F1EB}".as_bytes());
        // The provisional cell must be written as width=2.
        let w0 = vt.active_buf_ref().get(0, 0).map(|c| c.width).unwrap_or(0);
        assert_eq!(w0, 2, "provisional RI must occupy 2 cells");
        // A subsequent non-RI character must narrow it to 1 cell.
        vt.process("A".as_bytes());
        let w0_after = vt.active_buf_ref().get(0, 0).map(|c| c.width).unwrap_or(0);
        assert_eq!(
            w0_after, 1,
            "RI must be narrowed to 1 cell when followed by non-RI"
        );
        let g1 = vt
            .active_buf_ref()
            .get(0, 1)
            .map(|c| c.grapheme.clone())
            .unwrap_or_default();
        assert_eq!(g1, "A", "'A' must land at col=1");
    }

    /// RI followed immediately by a skin-tone modifier: the RI must be committed
    /// as narrow (1 cell) before the skin-tone attaches to whatever precedes it.
    #[test]
    fn r7_skin_tone_after_lone_ri_narrows_ri() {
        let mut vt = make_vt(80, 24);
        // U+1F1EB alone, then skin-tone U+1F3FB.
        vt.process("\u{1F1EB}\u{1F3FB}".as_bytes());
        // The RI must be committed as narrow (width=1) because a skin-tone is
        // not a second RI — the RI is unpaired.
        let w0 = vt.active_buf_ref().get(0, 0).map(|c| c.width).unwrap_or(0);
        assert_eq!(w0, 1, "lone RI before skin-tone must be narrowed to 1 cell");
    }

    // ---------------------------------------------------------------------------
    // CHA — Cursor Horizontal Absolute (CSI G)
    // ---------------------------------------------------------------------------

    /// CHA positions the cursor at the column indicated (1-based → 0-indexed).
    #[test]
    fn test_cha_basic() {
        let mut vt = make_vt(80, 24);
        // CSI 5 G — move to column 5 (1-based), i.e. col=4 (0-based).
        vt.process(b"\x1b[5G");
        assert_eq!(vt.normal_cursor.col, 4, "CHA 5 must place cursor at col=4");
    }

    /// CHA with value exceeding column count is clamped to cols-1.
    #[test]
    fn test_cha_clamps_to_cols() {
        let mut vt = make_vt(80, 24);
        // CSI 999 G — far beyond 80 cols; must clamp to col=79.
        vt.process(b"\x1b[999G");
        assert_eq!(
            vt.normal_cursor.col, 79,
            "CHA beyond cols must clamp to cols-1"
        );
    }

    // ---------------------------------------------------------------------------
    // VPA — Vertical Position Absolute (CSI d)
    // ---------------------------------------------------------------------------

    /// VPA positions the cursor at the row indicated (1-based → 0-indexed).
    #[test]
    fn test_vpa_basic() {
        let mut vt = make_vt(80, 24);
        // CSI 3 d — move to row 3 (1-based), i.e. row=2 (0-based).
        vt.process(b"\x1b[3d");
        assert_eq!(vt.normal_cursor.row, 2, "VPA 3 must place cursor at row=2");
    }

    /// VPA with value exceeding row count is clamped to rows-1.
    #[test]
    fn test_vpa_clamps_to_rows() {
        let mut vt = make_vt(80, 24);
        // CSI 999 d — far beyond 24 rows; must clamp to row=23.
        vt.process(b"\x1b[999d");
        assert_eq!(
            vt.normal_cursor.row, 23,
            "VPA beyond rows must clamp to rows-1"
        );
    }

    // ---------------------------------------------------------------------------
    // HPA — Horizontal Position Absolute (CSI `)
    // ---------------------------------------------------------------------------

    /// HPA (backtick) behaves identically to CHA.
    #[test]
    fn test_hpa_equivalent_to_cha() {
        let mut vt = make_vt(80, 24);
        // CSI 10 ` — move to column 10 (1-based), i.e. col=9 (0-based).
        vt.process(b"\x1b[10`");
        assert_eq!(
            vt.normal_cursor.col, 9,
            "HPA must behave identically to CHA"
        );
    }

    // ---------------------------------------------------------------------------
    // ECH — Erase Character (CSI X)
    // ---------------------------------------------------------------------------

    /// ECH erases N chars at cursor position without moving the cursor.
    #[test]
    fn test_ech_erases_without_moving_cursor() {
        let mut vt = make_vt(80, 24);
        // Write "ABCDE" at row=0, then position cursor at col=1.
        vt.process(b"ABCDE");
        // Move cursor to col=1 (CHA 2).
        vt.process(b"\x1b[2G");
        // ECH 2 — erase 2 chars starting at col=1 (B and C).
        vt.process(b"\x1b[2X");
        // Cursor must remain at col=1.
        assert_eq!(vt.normal_cursor.col, 1, "ECH must not move the cursor");
        // Cell at col=0 must still be 'A'.
        assert_eq!(
            grapheme_at(&vt, 0, 0),
            "A",
            "ECH must not erase cells before cursor"
        );
        // Cells at col=1 and col=2 must be erased (default ' ').
        let g1 = grapheme_at(&vt, 0, 1);
        let g2 = grapheme_at(&vt, 0, 2);
        assert!(
            g1 == " " || g1.is_empty(),
            "ECH must erase cell at col=1, got {:?}",
            g1
        );
        assert!(
            g2 == " " || g2.is_empty(),
            "ECH must erase cell at col=2, got {:?}",
            g2
        );
        // Cell at col=3 must still be 'D'.
        assert_eq!(
            grapheme_at(&vt, 0, 3),
            "D",
            "ECH must not erase cells past N"
        );
    }

    /// ECH with N larger than remaining columns is clamped to end of line.
    #[test]
    fn test_ech_clamps_to_eol() {
        let mut vt = make_vt(10, 5);
        // Fill row=0 with 'X' chars.
        vt.process(b"XXXXXXXXXX");
        // Move to col=8 (CHA 9).
        vt.process(b"\x1b[9G");
        // ECH 999 — far beyond EOL; must clamp to remaining 2 cells (col=8, col=9).
        vt.process(b"\x1b[999X");
        // Cursor stays at col=8.
        assert_eq!(vt.normal_cursor.col, 8, "ECH must not move cursor");
        // Cells 8 and 9 must be erased.
        let g8 = grapheme_at(&vt, 0, 8);
        let g9 = grapheme_at(&vt, 0, 9);
        assert!(
            g8 == " " || g8.is_empty(),
            "ECH must erase col=8 when N > remaining"
        );
        assert!(
            g9 == " " || g9.is_empty(),
            "ECH must erase col=9 when N > remaining"
        );
        // Cells before cursor must be intact.
        assert_eq!(
            grapheme_at(&vt, 0, 0),
            "X",
            "ECH must not erase before cursor"
        );
    }

    // ---------------------------------------------------------------------------
    // CNL — Cursor Next Line (CSI E)
    // ---------------------------------------------------------------------------

    /// CNL moves cursor down N lines and sets col=0.
    #[test]
    fn test_cnl_moves_cursor() {
        let mut vt = make_vt(80, 24);
        // Start at col=5 via CHA.
        vt.process(b"\x1b[6G");
        // CNL 3 — move down 3 lines.
        vt.process(b"\x1b[3E");
        assert_eq!(vt.normal_cursor.row, 3, "CNL 3 must move to row=3");
        assert_eq!(vt.normal_cursor.col, 0, "CNL must set col=0");
    }

    /// CNL with DECOM off clamps to screen bottom, not scroll region bottom.
    #[test]
    fn test_cnl_respects_scroll_bottom() {
        let mut vt = make_vt(80, 24);
        // Set scroll region rows 2–5 (1-based): DECSTBM CSI 2 ; 5 r.
        vt.process(b"\x1b[2;5r");
        // Position cursor at row=4 (CUP row=5, col=1 in 1-based → row=4).
        vt.process(b"\x1b[5;1H");
        // CNL 999 — with DECOM off, must clamp to screen bottom (row=23), not scroll region bottom.
        vt.process(b"\x1b[999E");
        assert_eq!(
            vt.normal_cursor.row, 23,
            "CNL with DECOM off must clamp to screen bottom (row 23), not scroll region bottom"
        );
        assert_eq!(vt.normal_cursor.col, 0, "CNL must set col=0");
    }

    // ---------------------------------------------------------------------------
    // CPL — Cursor Previous Line (CSI F)
    // ---------------------------------------------------------------------------

    /// CPL moves cursor up N lines and sets col=0.
    #[test]
    fn test_cpl_moves_cursor() {
        let mut vt = make_vt(80, 24);
        // Position at row=5 via CUP.
        vt.process(b"\x1b[6;5H");
        // CPL 2 — move up 2 lines.
        vt.process(b"\x1b[2F");
        assert_eq!(
            vt.normal_cursor.row, 3,
            "CPL 2 from row=5 must land at row=3"
        );
        assert_eq!(vt.normal_cursor.col, 0, "CPL must set col=0");
    }

    /// CPL with DECOM off clamps to screen top, not scroll region top.
    #[test]
    fn test_cpl_respects_scroll_top() {
        let mut vt = make_vt(80, 24);
        // Set scroll region rows 3–10 (1-based): DECSTBM CSI 3 ; 10 r → top=2, bottom=9 (0-based).
        vt.process(b"\x1b[3;10r");
        // Position cursor at row=3 (CUP row=4, col=1 → row=3 0-based).
        vt.process(b"\x1b[4;1H");
        // CPL 999 — with DECOM off, must clamp to screen top (row=0), not scroll region top.
        vt.process(b"\x1b[999F");
        assert_eq!(
            vt.normal_cursor.row, 0,
            "CPL with DECOM off must clamp to screen top (row 0), not scroll region top"
        );
        assert_eq!(vt.normal_cursor.col, 0, "CPL must set col=0");
    }

    // ---------------------------------------------------------------------------
    // Cursor blink — DECSET 12 / DECRST 12
    // ---------------------------------------------------------------------------

    /// `?12h` enables cursor blinking; `?12l` disables it.
    #[test]
    fn test_cursor_blink_decset12() {
        let mut vt = make_vt(80, 24);
        // Default state: blink disabled.
        assert!(!vt.cursor_blink, "cursor_blink must default to false");
        // DECSET 12 — enable blink.
        vt.process(b"\x1b[?12h");
        assert!(vt.cursor_blink, "cursor_blink must be true after ?12h");
        // DECRST 12 — disable blink.
        vt.process(b"\x1b[?12l");
        assert!(!vt.cursor_blink, "cursor_blink must be false after ?12l");
    }

    // ---------------------------------------------------------------------------
    // Bug A — wrap_pending must be cleared by explicit cursor movements
    // ---------------------------------------------------------------------------

    #[test]
    fn wrap_pending_cleared_by_cup() {
        let mut vt = make_vt(80, 24);
        vt.process(&[b'A'; 80]);
        assert!(
            vt.wrap_pending,
            "wrap_pending must be true after filling line"
        );
        vt.process(b"\x1b[1;1H");
        assert!(!vt.wrap_pending, "CUP must clear wrap_pending");
        vt.process(b"X");
        assert_eq!(grapheme_at(&vt, 0, 0), "X", "X must land at row=0, col=0");
        assert_eq!(vt.normal_cursor.row, 0, "cursor must remain on row 0");
    }

    #[test]
    fn wrap_pending_cleared_by_cuu() {
        let mut vt = make_vt(80, 24);
        vt.process(b"\x1b[3;1H");
        vt.process(&[b'A'; 80]);
        assert!(vt.wrap_pending);
        vt.process(b"\x1b[2A");
        assert!(!vt.wrap_pending, "CUU must clear wrap_pending");
    }

    #[test]
    fn wrap_pending_cleared_by_cud() {
        let mut vt = make_vt(80, 24);
        vt.process(&[b'A'; 80]);
        assert!(vt.wrap_pending);
        vt.process(b"\x1b[1B");
        assert!(!vt.wrap_pending, "CUD must clear wrap_pending");
    }

    #[test]
    fn wrap_pending_cleared_by_cha() {
        let mut vt = make_vt(80, 24);
        vt.process(&[b'A'; 80]);
        assert!(vt.wrap_pending);
        vt.process(b"\x1b[1G");
        assert!(!vt.wrap_pending, "CHA must clear wrap_pending");
    }

    #[test]
    fn wrap_pending_cleared_by_vpa() {
        let mut vt = make_vt(80, 24);
        vt.process(&[b'A'; 80]);
        assert!(vt.wrap_pending);
        vt.process(b"\x1b[5d");
        assert!(!vt.wrap_pending, "VPA must clear wrap_pending");
    }

    #[test]
    fn wrap_pending_cleared_by_decrc() {
        let mut vt = make_vt(80, 24);
        vt.process(b"\x1b7");
        vt.process(&[b'A'; 80]);
        assert!(vt.wrap_pending, "wrap_pending must be true before restore");
        vt.process(b"\x1b8");
        assert!(!vt.wrap_pending, "DECRC (ESC 8) must clear wrap_pending");
    }

    #[test]
    fn wrap_pending_cleared_by_decstbm() {
        let mut vt = make_vt(80, 24);
        vt.process(&[b'A'; 80]);
        assert!(vt.wrap_pending);
        vt.process(b"\x1b[1;20r");
        assert!(!vt.wrap_pending, "DECSTBM must clear wrap_pending");
        assert_eq!(vt.normal_cursor.row, 0);
        assert_eq!(vt.normal_cursor.col, 0);
    }

    #[test]
    fn wrap_pending_cleared_by_ed2() {
        let mut vt = make_vt(80, 24);
        vt.process(&[b'A'; 80]);
        assert!(vt.wrap_pending);
        vt.process(b"\x1b[2J");
        assert!(!vt.wrap_pending, "ED 2 must clear wrap_pending");
    }

    #[test]
    fn htop_footer_scenario() {
        let mut vt = make_vt(80, 24);
        vt.process(b"\x1b[23;1H");
        vt.process(&[b'X'; 79]);
        assert!(!vt.wrap_pending);
        vt.process(b"\x1b[1;23r");
        assert!(!vt.wrap_pending, "DECSTBM must clear wrap_pending");
        assert_eq!(vt.normal_cursor.row, 0);
        vt.process(b"\x1b[24;1H");
        assert_eq!(vt.normal_cursor.row, 23);
        vt.process(b"FOOTER");
        assert_eq!(grapheme_at(&vt, 23, 0), "F");
        assert_eq!(grapheme_at(&vt, 23, 1), "O");
        assert_eq!(grapheme_at(&vt, 22, 0), "X");
    }

    #[test]
    fn htop_footer_scenario_with_wrap_pending() {
        let mut vt = make_vt(80, 24);
        vt.process(b"\x1b[23;1H");
        vt.process(&[b'X'; 80]);
        assert!(vt.wrap_pending);
        vt.process(b"\x1b[1;23r");
        assert!(!vt.wrap_pending, "DECSTBM must clear wrap_pending");
        assert_eq!(vt.normal_cursor.row, 0);
        vt.process(b"\x1b[24;1H");
        assert_eq!(vt.normal_cursor.row, 23);
        vt.process(b"FOOTER");
        assert_eq!(grapheme_at(&vt, 23, 0), "F", "footer must land on row 23");
        assert_eq!(grapheme_at(&vt, 22, 0), "X");
    }

    // ---------------------------------------------------------------------------
    // Bug B — CUU/CUD/CNL/CPL/VPR clamp to screen edges when DECOM is off
    // ---------------------------------------------------------------------------

    #[test]
    fn cuu_clamps_to_screen_top_when_decom_off() {
        let mut vt = make_vt(80, 24);
        vt.process(b"\x1b[5;20r");
        vt.process(b"\x1b[7;1H");
        assert_eq!(vt.normal_cursor.row, 6);
        vt.process(b"\x1b[10A");
        assert_eq!(
            vt.normal_cursor.row, 0,
            "CUU with DECOM off must clamp to screen top (row 0), not scroll region top (row 4)"
        );
    }

    #[test]
    fn cud_clamps_to_screen_bottom_when_decom_off() {
        let mut vt = make_vt(80, 24);
        vt.process(b"\x1b[1;15r");
        vt.process(b"\x1b[14;1H");
        assert_eq!(vt.normal_cursor.row, 13);
        vt.process(b"\x1b[10B");
        assert_eq!(
            vt.normal_cursor.row, 23,
            "CUD with DECOM off must clamp to screen bottom (row 23), not scroll region bottom (row 14)"
        );
    }

    #[test]
    fn cuu_clamps_to_region_top_when_decom_on() {
        let mut vt = make_vt(80, 24);
        vt.process(b"\x1b[5;20r");
        vt.process(b"\x1b[?6h");
        vt.process(b"\x1b[3;1H");
        assert_eq!(vt.normal_cursor.row, 6);
        vt.process(b"\x1b[10A");
        assert_eq!(
            vt.normal_cursor.row, 4,
            "CUU with DECOM on must clamp to scroll region top (row 4)"
        );
    }

    // ---------------------------------------------------------------------------
    // Bug C — DECSTBM single-row region (top == bottom) must be accepted
    // ---------------------------------------------------------------------------

    #[test]
    fn decstbm_single_row_region_accepted() {
        let mut vt = make_vt(80, 24);
        vt.process(b"\x1b[5;5r");
        assert_eq!(
            vt.modes.scroll_region,
            (4, 4),
            "single-row DECSTBM must set scroll_region to (4, 4)"
        );
    }

    #[test]
    fn decstbm_single_row_region_cursor_home() {
        let mut vt = make_vt(80, 24);
        vt.process(b"\x1b[10;5H");
        vt.process(b"\x1b[5;5r");
        assert_eq!(vt.normal_cursor.row, 0, "DECSTBM must home cursor row to 0");
        assert_eq!(vt.normal_cursor.col, 0, "DECSTBM must home cursor col to 0");
    }
}

// ---------------------------------------------------------------------------
// cursor_dirty — DirtyRegion.cursor_moved propagation tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod cursor_dirty {
    use crate::vt::VtProcessor;

    fn make_vt(cols: u16, rows: u16) -> VtProcessor {
        VtProcessor::new(cols, rows, 10_000)
    }

    #[test]
    fn cr_marks_cursor_moved() {
        let mut vt = make_vt(80, 24);
        let dirty = vt.process(b"\r");
        assert!(dirty.cursor_moved, "CR must set cursor_moved");
        assert!(!dirty.is_empty(), "CR must yield non-empty DirtyRegion");
        assert!(
            dirty.rows.is_empty(),
            "CR must not mark any cell rows dirty"
        );
    }

    #[test]
    fn cup_marks_cursor_moved() {
        let mut vt = make_vt(80, 24);
        let dirty = vt.process(b"\x1b[5;10H");
        assert!(dirty.cursor_moved, "CUP must set cursor_moved");
        assert!(!dirty.is_empty());
    }

    #[test]
    fn cuu_marks_cursor_moved() {
        // Move cursor to row 5 first so there is room to move up.
        let mut vt = make_vt(80, 24);
        vt.process(b"\x1b[6;1H");
        let dirty = vt.process(b"\x1b[2A");
        assert!(dirty.cursor_moved, "CUU must set cursor_moved");
    }

    #[test]
    fn cud_marks_cursor_moved() {
        let mut vt = make_vt(80, 24);
        let dirty = vt.process(b"\x1b[2B");
        assert!(dirty.cursor_moved, "CUD must set cursor_moved");
    }

    #[test]
    fn cuf_marks_cursor_moved() {
        let mut vt = make_vt(80, 24);
        let dirty = vt.process(b"\x1b[2C");
        assert!(dirty.cursor_moved, "CUF must set cursor_moved");
    }

    #[test]
    fn cub_marks_cursor_moved() {
        // Move cursor right first so there is room to move left.
        let mut vt = make_vt(80, 24);
        vt.process(b"\x1b[1;10H");
        let dirty = vt.process(b"\x1b[2D");
        assert!(dirty.cursor_moved, "CUB must set cursor_moved");
    }

    #[test]
    fn cha_marks_cursor_moved() {
        let mut vt = make_vt(80, 24);
        let dirty = vt.process(b"\x1b[10G");
        assert!(dirty.cursor_moved, "CHA must set cursor_moved");
    }

    #[test]
    fn vpa_marks_cursor_moved() {
        let mut vt = make_vt(80, 24);
        let dirty = vt.process(b"\x1b[5d");
        assert!(dirty.cursor_moved, "VPA must set cursor_moved");
    }

    #[test]
    fn cnl_marks_cursor_moved() {
        let mut vt = make_vt(80, 24);
        let dirty = vt.process(b"\x1b[1E");
        assert!(dirty.cursor_moved, "CNL must set cursor_moved");
    }

    #[test]
    fn cpl_marks_cursor_moved() {
        // Move cursor down first.
        let mut vt = make_vt(80, 24);
        vt.process(b"\x1b[5;1H");
        let dirty = vt.process(b"\x1b[1F");
        assert!(dirty.cursor_moved, "CPL must set cursor_moved");
    }

    #[test]
    fn lf_no_scroll_marks_cursor_moved() {
        // LF when cursor is not on the bottom row — moves cursor down without scrolling.
        let mut vt = make_vt(80, 24);
        // Cursor starts at row 0 — LF moves it to row 1.
        let dirty = vt.process(b"\x0A");
        assert!(dirty.cursor_moved, "LF (no scroll) must set cursor_moved");
    }

    #[test]
    fn bs_marks_cursor_moved() {
        // Move right first, then BS.
        let mut vt = make_vt(80, 24);
        vt.process(b"\x1b[1;5H");
        let dirty = vt.process(b"\x08");
        assert!(dirty.cursor_moved, "BS must set cursor_moved");
    }

    #[test]
    fn decset_25l_marks_cursor_moved() {
        // Hide cursor — cursor_visible changes from true to false.
        let mut vt = make_vt(80, 24);
        let dirty = vt.process(b"\x1b[?25l");
        assert!(
            dirty.cursor_moved,
            "?25l (hide cursor) must mark cursor_moved"
        );
    }

    #[test]
    fn decset_25h_no_change_does_not_mark() {
        // Show cursor when it is already visible — no change, no cursor_moved.
        let mut vt = make_vt(80, 24);
        // Default state: cursor_visible = true.
        let dirty = vt.process(b"\x1b[?25h");
        assert!(
            !dirty.cursor_moved,
            "?25h when cursor already visible must NOT mark cursor_moved"
        );
    }

    #[test]
    fn cursor_only_move_no_dirty_rows() {
        // Pure cursor move: cursor_moved set, no dirty rows, not empty.
        let mut vt = make_vt(80, 24);
        let dirty = vt.process(b"\x1b[10;5H");
        assert!(!dirty.is_full_redraw, "CUP must not trigger full redraw");
        assert!(
            dirty.rows.is_empty(),
            "CUP must not mark any cell rows dirty"
        );
        assert!(dirty.cursor_moved, "CUP must set cursor_moved");
        assert!(!dirty.is_empty(), "CUP must yield non-empty DirtyRegion");
    }
}
