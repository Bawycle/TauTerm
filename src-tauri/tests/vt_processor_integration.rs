// SPDX-License-Identifier: MPL-2.0

//! Integration tests for `VtProcessor` using in-process pipe-pair injection.
//!
//! These tests exercise the full VT parsing pipeline (bytes → `VtProcessor::process`
//! → `ScreenBuffer` state) without a real PTY. A pipe pair or direct byte slice
//! substitutes for the PTY output stream, giving fully deterministic behaviour.
//!
//! Coverage (§14.3 of TESTING.md):
//! - Multi-chunk sequences: VT sequences split across multiple `process()` calls
//! - Large block (> 4096 bytes): parser handles oversize input without truncation
//! - Resize mid-stream: grid dimensions update, cursors clamped, no data loss
//! - Dirty region: full-redraw flag and per-row dirty tracking

use tau_term_lib::vt::VtProcessor;

// ---------------------------------------------------------------------------
// 1. Multi-chunk sequences — ESC split across two reads
// ---------------------------------------------------------------------------

/// A CSI sequence (cursor position) whose ESC byte is delivered in chunk 1 and
/// the rest in chunk 2. The parser must reassemble across the boundary.
#[test]
fn multi_chunk_csi_split_across_reads() {
    let mut vt = VtProcessor::new(80, 24, 1_000, 0, false);

    // Write "AB" so we have visible content.
    vt.process(b"AB");

    // Send ESC alone (partial CSI).
    vt.process(b"\x1b");
    // Complete the CSI: move cursor to row 5, col 10 (1-based in VT, 0-based in buffer).
    // ESC [ 5 ; 10 H  → CUP (cursor position)
    vt.process(b"[5;10H");

    // Cursor should now be at row 4, col 9 (0-based).
    let snap = vt.get_snapshot();
    assert_eq!(
        snap.cursor_row, 4,
        "cursor_row must be 4 after split CUP (got {})",
        snap.cursor_row
    );
    assert_eq!(
        snap.cursor_col, 9,
        "cursor_col must be 9 after split CUP (got {})",
        snap.cursor_col
    );
}

/// A multi-byte OSC title sequence split at the OSC terminator (BEL).
#[test]
fn multi_chunk_osc_title_split_before_terminator() {
    let mut vt = VtProcessor::new(80, 24, 1_000, 0, false);

    // OSC 0 ; "Hello" without the BEL terminator.
    vt.process(b"\x1b]0;Hello");
    // No title set yet — incomplete sequence.
    // Deliver the BEL terminator in a second chunk.
    vt.process(b"\x07");

    assert_eq!(
        vt.title, "Hello",
        "Title must be set after OSC completed across two chunks"
    );
}

/// An SGR sequence split so the parameter bytes land in separate reads.
/// ESC [ 1 ; in chunk 1, 3 m in chunk 2 → bold + italic.
#[test]
fn multi_chunk_sgr_split_inside_params() {
    let mut vt = VtProcessor::new(80, 24, 1_000, 0, false);

    vt.process(b"\x1b[1;");
    vt.process(b"3m");
    // Write a character with these attributes active.
    vt.process(b"X");

    let snap = vt.get_snapshot();
    let cell = snap.cells.first().expect("cell at (0,0)");
    assert!(cell.bold, "cell must be bold after split SGR 1;3m");
    assert!(cell.italic, "cell must be italic after split SGR 1;3m");
}

// ---------------------------------------------------------------------------
// 2. Large block > 4096 bytes — no truncation, no partial-sequence artifacts
// ---------------------------------------------------------------------------

/// Write 5000 'A' bytes in a single `process()` call. All must appear in the
/// screen buffer — none silently dropped by an internal buffer size limit.
#[test]
fn large_block_no_truncation_plain_text() {
    // 80 cols × 24 rows = 1920 cells — enough rows to hold 5000 chars with wrapping.
    // Use a very large terminal to avoid scrollback truncation of visible content.
    let mut vt = VtProcessor::new(200, 100, 50_000, 0, false);

    let input: Vec<u8> = b"A".repeat(5_000);
    vt.process(&input);

    let snap = vt.get_snapshot();
    let total_a: usize = snap.cells.iter().filter(|c| c.content == "A").count();
    // All 5000 'A's must be present (the screen is 200×100 = 20000 cells).
    assert_eq!(
        total_a, 5_000,
        "All 5000 'A' bytes must be present after large-block write (got {total_a})"
    );
}

/// A 6000-byte block that interleaves printable text with CSI sequences.
/// The parser must not drop or corrupt the interleaved sequences.
#[test]
fn large_block_with_interspersed_csi_sequences() {
    let mut vt = VtProcessor::new(80, 60, 10_000, 0, false);

    // Build: 50 repetitions of "hello\r\n" (~350 bytes total) followed by a
    // single CUP sequence, then 5000 'B' bytes.
    let mut data: Vec<u8> = Vec::with_capacity(6_000);
    for _ in 0..50 {
        data.extend_from_slice(b"hello\r\n");
    }
    // CUP: move to row 1, col 1 (0-based: row 0, col 0).
    data.extend_from_slice(b"\x1b[1;1H");
    data.extend_from_slice(&b"B".repeat(5_000));

    vt.process(&data);

    // After CUP + 5000 'B', the first cell must be 'B'.
    let snap = vt.get_snapshot();
    let first = snap.cells.first().map(|c| c.content.as_str()).unwrap_or("");
    assert_eq!(first, "B", "First cell must be 'B' after CUP + large write");
}

// ---------------------------------------------------------------------------
// 3. Resize mid-stream — grid dimensions update, cursors clamped
// ---------------------------------------------------------------------------

/// Process some content, resize to a smaller terminal, then process more content.
/// Grid dimensions must reflect the new size; cursors must be clamped.
#[test]
fn resize_mid_stream_updates_dimensions() {
    let mut vt = VtProcessor::new(80, 24, 1_000, 0, false);

    // Write a character at the bottom-right corner.
    // CUP 24;80 (1-based) → row 23, col 79 (0-based).
    vt.process(b"\x1b[24;80HX");
    let snap = vt.get_snapshot();
    assert_eq!(snap.cols, 80);
    assert_eq!(snap.rows, 24);

    // Resize down to 40×12.
    vt.resize(40, 12);
    let snap = vt.get_snapshot();
    assert_eq!(snap.cols, 40, "cols must be 40 after resize");
    assert_eq!(snap.rows, 12, "rows must be 12 after resize");

    // Cursor must have been clamped to the new bounds (max row 11, max col 39).
    assert!(
        snap.cursor_row <= 11,
        "cursor_row must be clamped to 11 after resize to 12 rows (got {})",
        snap.cursor_row
    );
    assert!(
        snap.cursor_col <= 39,
        "cursor_col must be clamped to 39 after resize to 40 cols (got {})",
        snap.cursor_col
    );
}

/// Content written before resize must not be corrupted after resize.
#[test]
fn resize_mid_stream_preserves_content() {
    let mut vt = VtProcessor::new(80, 24, 1_000, 0, false);

    // Write "Hi" at (0,0).
    vt.process(b"Hi");

    // Resize to 40×12.
    vt.resize(40, 12);

    // "Hi" at row 0 must still be there.
    let snap = vt.get_snapshot();
    let h = snap.cells.first().map(|c| c.content.as_str()).unwrap_or("");
    let i = snap.cells.get(1).map(|c| c.content.as_str()).unwrap_or("");
    assert_eq!(h, "H", "Cell(0,0) must still be 'H' after resize");
    assert_eq!(i, "i", "Cell(0,1) must still be 'i' after resize");
}

/// Resize to a larger terminal — grid expands without panic.
#[test]
fn resize_mid_stream_expand() {
    let mut vt = VtProcessor::new(40, 12, 1_000, 0, false);

    vt.process(b"expand");

    vt.resize(80, 24);
    let snap = vt.get_snapshot();
    assert_eq!(snap.cols, 80);
    assert_eq!(snap.rows, 24);
    // Original content still readable.
    let e = snap.cells.first().map(|c| c.content.as_str()).unwrap_or("");
    assert_eq!(e, "e", "Cell(0,0) must be 'e' after expand resize");
}

/// Multiple sequential resizes must not panic or corrupt state.
#[test]
fn resize_mid_stream_multiple_sequential() {
    let mut vt = VtProcessor::new(80, 24, 1_000, 0, false);
    vt.process(b"A");
    vt.resize(40, 12);
    vt.process(b"B");
    vt.resize(120, 40);
    vt.process(b"C");
    vt.resize(80, 24);

    // No panic is the primary assertion; snap must be valid.
    let snap = vt.get_snapshot();
    assert_eq!(snap.cols, 80);
    assert_eq!(snap.rows, 24);
}

// ---------------------------------------------------------------------------
// 4. Dirty region — tracking and coalescing
// ---------------------------------------------------------------------------

/// A single character write must mark exactly that row dirty.
#[test]
fn dirty_region_single_char_marks_row() {
    let mut vt = VtProcessor::new(80, 24, 1_000, 0, false);
    let dirty = vt.process(b"X");
    assert!(
        dirty.rows.contains(0u16) || dirty.is_full_redraw,
        "Row 0 must be dirty after writing 'X'"
    );
}

/// Alternate screen switch must produce a full-redraw dirty region.
#[test]
fn dirty_region_alt_screen_switch_marks_full_redraw() {
    let mut vt = VtProcessor::new(80, 24, 1_000, 0, false);
    // Enter alternate screen (DECSET 1049).
    let dirty = vt.process(b"\x1b[?1049h");
    assert!(
        dirty.is_full_redraw,
        "Entering alternate screen must mark a full redraw"
    );

    // Leave alternate screen (DECRST 1049).
    let dirty = vt.process(b"\x1b[?1049l");
    assert!(
        dirty.is_full_redraw,
        "Leaving alternate screen must mark a full redraw"
    );
}

/// Two sequences processed in one `process()` call produce a merged dirty region.
#[test]
fn dirty_region_two_rows_written_both_dirty() {
    let mut vt = VtProcessor::new(80, 24, 1_000, 0, false);
    // Write to row 0, then move to row 1 and write there — all in one process call.
    let dirty = vt.process(b"Row0\r\nRow1");
    // Row 0 and row 1 must both be in the dirty set (or full redraw).
    let has_row0 = dirty.rows.contains(0u16) || dirty.is_full_redraw;
    let has_row1 = dirty.rows.contains(1u16) || dirty.is_full_redraw;
    assert!(has_row0, "Row 0 must be dirty after writing 'Row0'");
    assert!(has_row1, "Row 1 must be dirty after writing 'Row1'");
}

// ---------------------------------------------------------------------------
// 5. Cursor movement sequences
// ---------------------------------------------------------------------------

/// CUP (CSI row ; col H) positions the cursor correctly.
#[test]
fn cursor_cup_positions_correctly() {
    let mut vt = VtProcessor::new(80, 24, 1_000, 0, false);
    // CUP 3;5 → 0-based row 2, col 4.
    vt.process(b"\x1b[3;5H");
    let snap = vt.get_snapshot();
    assert_eq!(snap.cursor_row, 2);
    assert_eq!(snap.cursor_col, 4);
}

/// CUF (cursor forward) advances the cursor column.
#[test]
fn cursor_cuf_advances_column() {
    let mut vt = VtProcessor::new(80, 24, 1_000, 0, false);
    vt.process(b"\x1b[5C"); // Move right 5.
    let snap = vt.get_snapshot();
    assert_eq!(snap.cursor_col, 5);
}

/// CR (0x0D) returns cursor to column 0.
#[test]
fn cr_returns_to_column_zero() {
    let mut vt = VtProcessor::new(80, 24, 1_000, 0, false);
    vt.process(b"ABCDE\r");
    let snap = vt.get_snapshot();
    assert_eq!(snap.cursor_col, 0, "CR must move cursor to column 0");
}

/// LF (0x0A) advances the cursor row by one.
#[test]
fn lf_advances_row() {
    let mut vt = VtProcessor::new(80, 24, 1_000, 0, false);
    vt.process(b"\n");
    let snap = vt.get_snapshot();
    assert_eq!(snap.cursor_row, 1, "LF must advance cursor to row 1");
}

// ---------------------------------------------------------------------------
// 5b. Partial snapshot parity — C1 (P3)
// ---------------------------------------------------------------------------

/// Verifies that accessing dirty rows directly via `active_buf_ref().get_row()`
/// produces cell data bit-identical to what `get_snapshot()` returns for the same rows.
///
/// This is the safety net for C1 (P3): the partial-update path must be
/// equivalent to the full-snapshot path for every dirty row.
#[test]
fn partial_snapshot_parity_with_full_snapshot() {
    let mut proc = tau_term_lib::vt::VtProcessor::new(80, 24, 1_000, 0, false);

    // Row 0: colored text (bold red foreground).
    proc.process(b"\x1b[1;31mHello\x1b[0m");
    // Row 1: wide char U+4E2D (中, width=2) followed by normal text.
    proc.process(b"\r\n\xe4\xb8\xad ok");
    // Row 2: move to row 3 (skip row 2), write inverse video text.
    proc.process(b"\x1b[4;1H\x1b[7mInverse\x1b[0m");

    // Dirty rows after processing: 0, 1, 3 (row 2 was skipped by CUP).
    let dirty_rows: Vec<u16> = vec![0, 1, 3];

    // Reference path: full snapshot.
    let snap = proc.get_snapshot();
    let cols = snap.cols as usize;

    // New path: direct row access via active_buf_ref().get_row().
    // For each dirty row, compare cell-by-cell against the snapshot.
    let buf = proc.active_buf_ref();

    for &row in &dirty_rows {
        let snap_start = row as usize * cols;
        let snap_end = snap_start + cols;
        let snap_row = &snap.cells[snap_start..snap_end];

        let buf_row = buf
            .get_row(row)
            .unwrap_or_else(|| panic!("get_row({row}) must return Some for a valid row"));

        assert_eq!(
            buf_row.len(),
            snap_row.len(),
            "row {row}: cell count must match between direct access and snapshot"
        );

        for (col, (buf_cell, snap_cell)) in buf_row.iter().zip(snap_row.iter()).enumerate() {
            assert_eq!(
                buf_cell.grapheme, snap_cell.content,
                "row {row} col {col}: grapheme mismatch"
            );
            assert_eq!(
                buf_cell.width, snap_cell.width,
                "row {row} col {col}: width mismatch"
            );
            assert_eq!(
                buf_cell.attrs.bold, snap_cell.bold,
                "row {row} col {col}: bold mismatch"
            );
            assert_eq!(
                buf_cell.attrs.italic, snap_cell.italic,
                "row {row} col {col}: italic mismatch"
            );
            assert_eq!(
                buf_cell.attrs.fg, snap_cell.fg,
                "row {row} col {col}: fg mismatch"
            );
            assert_eq!(
                buf_cell.attrs.bg, snap_cell.bg,
                "row {row} col {col}: bg mismatch"
            );
            assert_eq!(
                buf_cell.attrs.inverse, snap_cell.inverse,
                "row {row} col {col}: inverse mismatch"
            );
        }
    }
}

// ---------------------------------------------------------------------------
// 6. Scrollback accumulation
// ---------------------------------------------------------------------------

/// Lines scrolled off the top must enter the scrollback ring.
#[test]
fn scrollback_accumulates_on_scroll() {
    let mut vt = VtProcessor::new(80, 24, 1_000, 0, false);

    // Fill all 24 rows with labelled lines and then push one more to force scroll.
    // Use "Row NN\r\n" — 25 lines causes at least one scroll event.
    for i in 0..25u32 {
        let line = format!("Line{i:02}\r\n");
        vt.process(line.as_bytes());
    }

    // Scrollback must have at least one entry.
    let snap = vt.get_snapshot();
    assert!(
        snap.scrollback_lines >= 1,
        "At least one scrollback line expected after filling screen (got {})",
        snap.scrollback_lines
    );
}
