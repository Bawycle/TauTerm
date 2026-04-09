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
        let mut vt = VtProcessor::new(80, 24, 10_000, 0, false);
        // Set a title. Note: the `vte` parser splits the OSC payload on each ';',
        // so "injected;ls -la" is delivered as params[1]="injected", params[2]="ls -la".
        // With the new params-based API, only params[1] is used as the title.
        vt.process(b"\x1b]0;injected\x07");
        assert_eq!(vt.title, "injected");

        // Send CSI 21t (window title read request) — must be silently ignored.
        let _dirty = vt.process(b"\x1b[21t");
        // No panic and no dedicated response buffer exists — the sequence is a no-op.
    }

    /// SEC-PTY-001: CSI 21t after a title containing a shell injection payload.
    #[test]
    fn sec_pty_001_csi_21t_after_shell_injection_title_no_effect() {
        let mut vt = VtProcessor::new(80, 24, 10_000, 0, false);
        let _dirty = vt.process(b"\x1b]0;$(id)\x07\x1b[21t");
        // No panic, no crash, no observable injection.
    }

    // -----------------------------------------------------------------------
    // SEC-PTY-002 — OSC query sequences discarded (no echo-back)
    // -----------------------------------------------------------------------

    /// SEC-PTY-002: OSC 10;? (foreground color query) must be silently discarded.
    #[test]
    fn sec_pty_002_osc_color_query_no_response() {
        let mut vt = VtProcessor::new(80, 24, 10_000, 0, false);
        // OSC 10 ; ? BEL
        let _dirty = vt.process(b"\x1b]10;?\x07");
        // No panic. VtProcessor has no response buffer — confirms no echo-back.
    }

    /// SEC-PTY-002: DECRQSS (ESC P $ q ... ESC \) must be silently discarded.
    #[test]
    fn sec_pty_002_decrqss_ignored() {
        let mut vt = VtProcessor::new(80, 24, 10_000, 0, false);
        // DECRQSS sequence: ESC P $ q " p ESC \
        let _dirty = vt.process(b"\x1bP$q\"p\x1b\\");
        // No panic, no observable response.
    }

    /// SEC-PTY-002: CSI ? 1 $ p (DECRPM) must be silently discarded.
    #[test]
    fn sec_pty_002_decrpm_mode_query_ignored() {
        let mut vt = VtProcessor::new(80, 24, 10_000, 0, false);
        let _dirty = vt.process(b"\x1b[?1$p");
        // No panic, no mode response injected.
    }

    // -----------------------------------------------------------------------
    // SEC-PTY-003 — OSC sequence with large payload does not panic or OOM
    // -----------------------------------------------------------------------

    /// SEC-PTY-003: Large OSC 0 title payload must be processed without panic.
    #[test]
    fn sec_pty_003_large_osc_title_no_panic() {
        let mut vt = VtProcessor::new(80, 24, 10_000, 0, false);
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
        let mut vt = VtProcessor::new(80, 24, 10_000, 0, false);
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
        let mut vt = VtProcessor::new(80, 24, 10_000, 0, false);
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

    // -----------------------------------------------------------------------
    // SEC-PTY-008 — OSC 22 title stack is bounded (DoS prevention)
    // -----------------------------------------------------------------------

    /// SEC-PTY-008: OSC 22 title stack is bounded — cannot grow indefinitely.
    /// Prevents DoS via unbounded memory allocation (CVE-2022-24130 pattern).
    #[test]
    fn sec_pty_008_title_stack_bounded() {
        let mut vt = VtProcessor::new(80, 24, 10_000, 0, false);
        // Set a known initial title.
        vt.process(b"\x1b]0;base\x07");
        // Push 1000 titles — must silently cap at TITLE_STACK_MAX (16).
        for i in 0..1000u32 {
            let osc = format!("\x1b]22;title{i}\x07");
            vt.process(osc.as_bytes());
        }
        // Key assertion: no panic, no OOM, process completes.
        // Observable check: pop more than TITLE_STACK_MAX times — the title must
        // eventually stop changing (stack exhausted) rather than unwinding 1000 frames.
        let mut pop_count_with_change = 0u32;
        let mut prev_title = vt.title.clone();
        for _ in 0..20 {
            vt.process(b"\x1b]23;\x07");
            if vt.title != prev_title {
                pop_count_with_change += 1;
                prev_title = vt.title.clone();
            }
        }
        assert!(
            pop_count_with_change <= 16,
            "Title stack must be capped at TITLE_STACK_MAX=16; got {pop_count_with_change} restorations after 20 pops"
        );
    }

    // -----------------------------------------------------------------------
    // SEC-PTY-009 — OSC 23 pop on empty stack does not panic
    // -----------------------------------------------------------------------

    /// SEC-PTY-009: OSC 23 (PopTitle) on empty stack must not panic.
    #[test]
    fn sec_pty_009_title_stack_pop_on_empty_no_panic() {
        let mut vt = VtProcessor::new(80, 24, 10_000, 0, false);
        // Pop without any prior push — must not panic.
        for _ in 0..10 {
            vt.process(b"\x1b]23;\x07");
        }
        // Still functional after empty pops.
        vt.process(b"A");
        // No assertions needed — the test passes if it doesn't panic.
    }

    // -----------------------------------------------------------------------
    // sec_osc_oversized_ignored — D1 (P1) guard: OSC total_len > 8192
    // -----------------------------------------------------------------------

    /// D1: OSC sequence whose total field length exceeds 8192 bytes must be
    /// silently dropped — the terminal title must remain unchanged.
    #[test]
    fn sec_osc_oversized_ignored() {
        let mut vt = VtProcessor::new(80, 24, 10_000, 0, false);
        // Set a known initial title.
        vt.process(b"\x1b]0;initial\x07");
        assert_eq!(vt.title, "initial");

        // Build an OSC 0 sequence whose payload field alone is > 8192 bytes.
        // total_len = len("0") + 1 + len(payload) + 1 > 8192
        // So payload must be >= 8191 bytes.
        let big_payload: Vec<u8> = b"A".repeat(8200);
        let mut seq = b"\x1b]0;".to_vec();
        seq.extend_from_slice(&big_payload);
        seq.push(b'\x07'); // BEL terminator

        vt.process(&seq);

        // The oversized sequence must be silently ignored — title unchanged.
        assert_eq!(
            vt.title, "initial",
            "Oversized OSC sequence must not change the title (D1 guard)"
        );
    }

    /// SEC-PTY-007: Valid characters surrounding invalid UTF-8 must render correctly.
    #[test]
    fn sec_pty_007_valid_chars_unaffected_by_invalid_utf8() {
        use crate::vt::screen_buffer::SnapshotCell;
        let mut vt = VtProcessor::new(80, 24, 10_000, 0, false);
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
