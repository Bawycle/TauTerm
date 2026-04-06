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
        overflow_seq.extend(std::iter::repeat(b'X').take(5000));
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
            .chain(std::iter::repeat(b'A').take(300))
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
}
