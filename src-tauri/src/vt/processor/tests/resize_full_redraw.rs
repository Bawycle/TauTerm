// SPDX-License-Identifier: MPL-2.0

use crate::vt::VtProcessor;

/// After resize() + process(shell response) + process(single char),
/// the third process() call must NOT carry full_redraw=true.
///
/// Regression test for the "phantom full_redraw" bug: the first process()
/// after resize correctly drains the full_redraw flag from the ScreenBuffer,
/// but the second process() must see a clean dirty region.
#[test]
fn full_redraw_does_not_leak_after_resize_and_process() {
    let mut vt = VtProcessor::new(80, 24, 1_000);

    // Simulate initial shell output (prompt).
    let _ = vt.process(b"user@host:~$ ");

    // Simulate a resize (frontend triggers this via IPC).
    vt.resize(94, 33);

    // Process #4 equivalent: shell redraws after SIGWINCH.
    // A typical shell sends CR + erase-line + prompt.
    let dirty1 = vt.process(b"\r\x1b[K\x1b[?2004huser@host:~$ ");
    assert!(
        dirty1.is_full_redraw,
        "first process() after resize must carry full_redraw"
    );

    // Process #5 equivalent: user types 'l' (~3s later, echoed by shell).
    let dirty2 = vt.process(b"l");
    assert!(
        !dirty2.is_full_redraw,
        "second process() after resize must NOT carry full_redraw (got full_redraw=true with dirty_rows={:?})",
        dirty2.rows,
    );
    assert!(
        dirty2.rows.contains(&0) || !dirty2.rows.is_empty(),
        "typing a character must dirty at least the cursor row"
    );
}

/// Resize followed by two process() calls with no data in between.
/// The first drains full_redraw, the second must be clean.
#[test]
fn full_redraw_drains_on_first_process_after_resize() {
    let mut vt = VtProcessor::new(80, 24, 1_000);
    vt.resize(100, 30);

    let dirty1 = vt.process(b"x");
    assert!(
        dirty1.is_full_redraw,
        "first process after resize must be full_redraw"
    );

    let dirty2 = vt.process(b"y");
    assert!(
        !dirty2.is_full_redraw,
        "second process after resize must NOT be full_redraw"
    );
}
