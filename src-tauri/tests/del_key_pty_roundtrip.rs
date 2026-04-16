// SPDX-License-Identifier: MPL-2.0

//! Regression: Del key (CSI 3~) PTY round-trip with real bash.
//!
//! Reproduces a user-reported bug where pressing the Del key in an active
//! TauTerm pane caused the pane to become permanently unresponsive (no
//! output appears, but cursor blinks and new panes work).
//!
//! Diagnostic steps already performed (console instrumentation):
//! - Frontend keydown/input events fire correctly before and after Del
//! - `send_input` IPC returns Ok after the freeze
//! - `get_pane_screen_snapshot` shows the VT state is updated (Task 1 alive)
//! - But screen-update events stop flowing to the frontend
//!
//! This test isolates the RAW PTY + bash layer: if bash correctly echoes
//! characters sent before and after the Del sequence `\x1b[3~`, then the
//! freeze is NOT caused by bash/kernel PTY behavior, and the bug must be
//! elsewhere (VT parser, event emitter, IPC ack cycle).

#![allow(dead_code)] // helpers kept for future tests

use std::io::Read;
use std::sync::mpsc::{Receiver, RecvTimeoutError, channel};
use std::time::{Duration, Instant};

use tau_term_lib::platform::PtySession;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn open_bash_session(cols: u16, rows: u16) -> Box<dyn PtySession> {
    // Locate a bash binary. Prefer /usr/bin/bash, then /bin/bash.
    let bash_path = ["/usr/bin/bash", "/bin/bash"]
        .iter()
        .find(|p| std::path::Path::new(p).exists())
        .copied()
        .unwrap_or("/bin/bash");

    let backend = tau_term_lib::platform::create_pty_backend();
    let rows_str = rows.to_string();
    let cols_str = cols.to_string();
    // --noprofile + --rcfile /dev/null suppress startup scripts so PS1 is
    // set purely from the environment variable below. -i = interactive.
    backend
        .open_session(
            cols,
            rows,
            0,
            0,
            bash_path,
            // --norc skips /etc/bash.bashrc AND ~/.bashrc (distro-agnostic).
            // --noprofile skips login profile files.
            // -i forces interactive mode so readline is active (Del keys etc.).
            &["--norc", "--noprofile", "-i"],
            &[
                ("TERM", "xterm-256color"),
                ("COLORTERM", "truecolor"),
                ("LINES", rows_str.as_str()),
                ("COLUMNS", cols_str.as_str()),
                ("PS1", "PROMPT>>> "),
                ("HOME", "/tmp"),
            ],
            None,
        )
        .expect("open bash session")
}

/// Drain the channel into a String until `expected` appears or `timeout`
/// elapses.
fn read_until(
    rx: &Receiver<Vec<u8>>,
    accumulator: &mut String,
    expected: &str,
    timeout: Duration,
) -> bool {
    let deadline = Instant::now() + timeout;
    loop {
        if accumulator.contains(expected) {
            return true;
        }
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            return false;
        }
        match rx.recv_timeout(remaining) {
            Ok(chunk) => accumulator.push_str(&String::from_utf8_lossy(&chunk)),
            Err(RecvTimeoutError::Timeout) => return false,
            Err(RecvTimeoutError::Disconnected) => return false,
        }
    }
}

/// Drain any pending chunks in the channel into `accumulator` for
/// `quiet_window` of silence — i.e. until no new chunk has arrived for at
/// least `quiet_window`. Returns the text drained during this call.
fn drain_quiet(rx: &Receiver<Vec<u8>>, accumulator: &mut String, quiet_window: Duration) -> String {
    let mut drained = String::new();
    loop {
        match rx.recv_timeout(quiet_window) {
            Ok(chunk) => {
                let s = String::from_utf8_lossy(&chunk).to_string();
                accumulator.push_str(&s);
                drained.push_str(&s);
            }
            Err(_) => return drained, // Timeout or disconnected = quiet
        }
    }
}

/// Take the reader out of the session and wrap it in a dedicated thread +
/// channel so we can do timeout-based reads without holding any shared
/// mutex during blocking I/O.
fn session_with_reader_channel(session: &mut Box<dyn PtySession>) -> Receiver<Vec<u8>> {
    let reader_arc = session
        .reader_handle()
        .expect("PtySession must expose a reader");
    // Move ownership of the reader out of the Arc<Mutex<...>> by taking
    // the mutex and swapping in a dummy reader. The cleanest way is to
    // not unwrap at all — clone the Arc and spawn a reader thread that
    // locks on each iteration. That's what we'll do.
    // Pattern B: cloned Arc, reader thread loops on lock().
    let (tx, rx) = channel::<Vec<u8>>();
    std::thread::spawn(move || {
        let mut buf = [0u8; 4096];
        loop {
            let n = {
                let mut r = match reader_arc.lock() {
                    Ok(g) => g,
                    Err(_) => break,
                };
                match r.read(&mut buf) {
                    Ok(0) => break,
                    Ok(n) => n,
                    Err(_) => break,
                }
            };
            if tx.send(buf[..n].to_vec()).is_err() {
                break;
            }
        }
    });
    rx
}

// ---------------------------------------------------------------------------
// DIAGNOSTIC: capture whatever bash prints on startup so we can tune PS1 /
// readline detection.
// ---------------------------------------------------------------------------

#[test]
fn del_pty_000_diagnostic_capture_bash_startup() {
    let mut session = open_bash_session(80, 24);
    let rx = session_with_reader_channel(&mut session);
    let mut acc = String::new();
    drain_quiet(&rx, &mut acc, Duration::from_millis(1500));
    eprintln!(
        "DEL-PTY-000: bash startup output ({} bytes): {:?}",
        acc.len(),
        acc
    );
    eprintln!(
        "DEL-PTY-000: visible: {:?}",
        acc.replace('\x1b', "<ESC>")
            .replace('\r', "<CR>")
            .replace('\n', "<LF>")
    );
    // Drop session explicitly so bash exits and the orphaned reader thread
    // unblocks via EOF.
    drop(session);
}

// ---------------------------------------------------------------------------
// TEST: Del key does NOT break bash's ability to echo subsequent input.
// ---------------------------------------------------------------------------

/// DEL-PTY-001 — Send "abc" + Del (CSI 3~) + "xyz" to real bash and verify
/// that bash keeps echoing characters after Del.
///
/// Expected: bash with TERM=xterm-256color has `\e[3~: delete-char` bound
/// via readline; pressing Del at end of line is a no-op (possibly bell).
/// Subsequent typed chars MUST still be echoed.
///
/// Failure mode: if bash stops echoing after CSI 3~, this test will time
/// out waiting for "xyz" — indicating a bash/kernel bug (not TauTerm's).
#[test]
fn del_pty_001_bash_echoes_after_del_keypress() {
    let mut session = open_bash_session(80, 24);
    let rx = session_with_reader_channel(&mut session);
    let mut acc = String::new();

    // Step 1: wait for PROMPT
    assert!(
        read_until(&rx, &mut acc, "PROMPT>>>", Duration::from_secs(5)),
        "bash must print PROMPT>>> within 5s — got: {acc:?}"
    );

    // Step 2: type "abc"
    session.write(b"abc").expect("write abc");
    assert!(
        read_until(&rx, &mut acc, "abc", Duration::from_secs(2)),
        "bash must echo 'abc' — got: {acc:?}"
    );

    // Step 3: send Del (CSI 3~) — no-op at end of line.
    session.write(b"\x1b[3~").expect("write Del");
    let _drained = drain_quiet(&rx, &mut acc, Duration::from_millis(200));

    // Step 4: type "xyz" — MUST echo.
    session.write(b"xyz").expect("write xyz");
    assert!(
        read_until(&rx, &mut acc, "xyz", Duration::from_secs(3)),
        "DEL-PTY-001: bash must echo 'xyz' AFTER Del keypress — \
         freeze at PTY/bash layer would manifest here.\n\
         Accumulated output: {acc:?}"
    );

    // Step 5: \n must return to prompt.
    session.write(b"\n").expect("write newline");
    assert!(
        read_until(&rx, &mut acc, "PROMPT>>>", Duration::from_secs(3)),
        "DEL-PTY-001: bash must print a new PROMPT after Enter — \
         line editor may be stuck. Accumulated: {acc:?}"
    );
    drop(session);
}

/// DEL-PTY-002 — Same as 001 but Del is pressed in the MIDDLE of text.
#[test]
fn del_pty_002_del_mid_line_preserves_echo() {
    let mut session = open_bash_session(80, 24);
    let rx = session_with_reader_channel(&mut session);
    let mut acc = String::new();

    assert!(
        read_until(&rx, &mut acc, "PROMPT>>>", Duration::from_secs(5)),
        "bash PROMPT within 5s"
    );

    session.write(b"abcdef").expect("write abcdef");
    assert!(
        read_until(&rx, &mut acc, "abcdef", Duration::from_secs(2)),
        "echo abcdef"
    );

    session.write(b"\x1b[3D").expect("CUB 3"); // cursor left 3
    let _ = drain_quiet(&rx, &mut acc, Duration::from_millis(100));

    session.write(b"\x1b[3~").expect("Del mid-line");
    let _ = drain_quiet(&rx, &mut acc, Duration::from_millis(200));

    session.write(b"XYZ").expect("write XYZ");
    assert!(
        read_until(&rx, &mut acc, "XYZ", Duration::from_secs(3)),
        "DEL-PTY-002: bash must echo 'XYZ' after mid-line Del. \
         Acc: {acc:?}"
    );

    session.write(b"\x03").expect("Ctrl-C");
    drop(session);
}

/// DEL-PTY-003 — Capture the exact bytes bash emits in response to Del
/// so we can diagnose whether the VT parser has trouble with them.
#[test]
fn del_pty_003_capture_bash_del_response_bytes() {
    let mut session = open_bash_session(80, 24);
    let rx = session_with_reader_channel(&mut session);
    let mut acc = String::new();

    assert!(
        read_until(&rx, &mut acc, "PROMPT>>>", Duration::from_secs(5)),
        "bash PROMPT within 5s"
    );

    session.write(b"HELLO").expect("write HELLO");
    assert!(
        read_until(&rx, &mut acc, "HELLO", Duration::from_secs(2)),
        "echo HELLO"
    );

    session.write(b"\x1b[2D").expect("CUB 2");
    let _ = drain_quiet(&rx, &mut acc, Duration::from_millis(200));

    let t0 = Instant::now();
    session.write(b"\x1b[3~").expect("Del");
    let del_response = drain_quiet(&rx, &mut acc, Duration::from_millis(300));
    let elapsed = t0.elapsed();

    eprintln!(
        "DEL-PTY-003: bash responded to Del in {elapsed:?} with {} bytes",
        del_response.len()
    );
    eprintln!("DEL-PTY-003: raw bytes: {:?}", del_response.as_bytes());
    eprintln!(
        "DEL-PTY-003: visible: {}",
        del_response
            .replace('\x1b', "<ESC>")
            .replace('\r', "<CR>")
            .replace('\n', "<LF>")
            .replace('\x07', "<BEL>")
    );

    assert!(
        !del_response.is_empty(),
        "DEL-PTY-003: bash produced NO bytes in response to mid-line Del — \
         unexpected, readline should redraw the line"
    );

    session.write(b"\x03").expect("Ctrl-C");
    drop(session);
}
