// SPDX-License-Identifier: MPL-2.0

use super::{open_sh_session, read_until_timeout};
use crate::platform::PtyBackend;

use super::super::backend::LinuxPtyBackend;

// --- FPL-W-001: write small payload succeeds ---
#[test]
fn fpl_w_001_write_small_payload_succeeds() {
    let mut session = open_sh_session(80, 24).expect("open session");
    let result = session.write(b"ls\n");
    assert!(
        result.is_ok(),
        "write small payload must succeed: {:?}",
        result.err()
    );
}

// --- FPL-W-002: write 64 KiB payload succeeds ---
#[test]
fn fpl_w_002_write_max_payload_succeeds() {
    let mut session = open_sh_session(80, 24).expect("open session");
    let payload = vec![b'a'; 65_536];
    let result = session.write(&payload);
    assert!(
        result.is_ok(),
        "write 64 KiB must succeed: {:?}",
        result.err()
    );
}

// --- FPL-R-001: resize returns Ok ---
#[test]
fn fpl_r_001_resize_succeeds() {
    let mut session = open_sh_session(80, 24).expect("open session");
    let result = session.resize(120, 40, 0, 0);
    assert!(result.is_ok(), "resize must succeed: {:?}", result.err());
}

// --- FPL-R-002: resize with pixel dimensions succeeds ---
#[test]
fn fpl_r_002_resize_with_pixel_dims_succeeds() {
    let mut session = open_sh_session(80, 24).expect("open session");
    let result = session.resize(80, 24, 960, 480);
    assert!(
        result.is_ok(),
        "resize with pixel dims must succeed: {:?}",
        result.err()
    );
}

// --- FPL-R-003: degenerate resize (0,0) succeeds without panic ---
#[test]
fn fpl_r_003_resize_degenerate_zero_does_not_panic() {
    let mut session = open_sh_session(80, 24).expect("open session");
    // Degenerate size — result may be Ok or Err depending on kernel,
    // but must not panic.
    let _ = session.resize(0, 0, 0, 0);
}

// -----------------------------------------------------------------------
// FPL-W-003 — Write after session close returns an error
// -----------------------------------------------------------------------

/// FPL-W-003: Closing the PTY session (via Drop) must not panic, and
/// subsequent read on the master reader must return EOF or an error (no data).
///
/// Background: On Linux PTY, writing to the master fd after the child exits
/// does not reliably return EIO — the kernel may buffer the write in the
/// character device ring. The observable "dead fd" condition on a PTY master is
/// read-side: once the slave fd is closed (child exited), reading the master
/// returns EIO. We test this read-side behaviour here, not write-side.
#[test]
fn fpl_w_003_read_after_child_exit_returns_eof_or_error() {
    let backend = LinuxPtyBackend::new();
    let session = backend
        .open_session(
            80,
            24,
            "/bin/sh",
            &["-c", "exit 0"], // shell exits immediately
            &[("TERM", "xterm-256color")],
        )
        .expect("open session");

    let reader = session
        .reader_handle()
        .expect("LinuxPtySession must have a reader");

    // The reader must eventually return EOF or EIO after the child exits.
    // We poll until we get a 0-byte read or an error.
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    let mut got_eof = false;
    {
        let mut r = reader.lock().expect("lock");
        let mut buf = [0u8; 64];
        while std::time::Instant::now() < deadline {
            match r.read(&mut buf) {
                Ok(0) | Err(_) => {
                    got_eof = true;
                    break;
                }
                Ok(_) => {}
            }
        }
    }

    assert!(
        got_eof,
        "FPL-W-003: reading from the PTY master after child exit must return EOF or error"
    );

    // Dropping the session must not panic.
    drop(session);
}

// -----------------------------------------------------------------------
// FPL-W-004 — Master write is readable via reader_handle (round-trip)
// -----------------------------------------------------------------------

/// FPL-W-004: Bytes written to the PTY (simulating keyboard input `echo`) are
/// echoed back through the master reader, validating the full round-trip path
/// used by the production PTY read task.
///
/// We use `echo` via the shell to produce predictable output on the master.
#[test]
fn fpl_w_004_write_master_readable_via_reader_handle() {
    let backend = LinuxPtyBackend::new();
    let session = backend
        .open_session(
            80,
            24,
            "/bin/sh",
            &["-c", "echo FPL_W_004_MARKER; sleep 5"],
            &[("TERM", "xterm-256color")],
        )
        .expect("open session");

    let reader = session
        .reader_handle()
        .expect("LinuxPtySession must have a reader");

    let output = read_until_timeout(
        reader,
        "FPL_W_004_MARKER",
        std::time::Duration::from_secs(5),
    );
    assert!(
        output.is_some(),
        "FPL-W-004: 'FPL_W_004_MARKER' must be readable via reader_handle after shell echo"
    );
}

// -----------------------------------------------------------------------
// FPL-R-005 — resize() delivers SIGWINCH to the child process
// -----------------------------------------------------------------------

/// FPL-R-005: Resizing the PTY must deliver SIGWINCH to the foreground process
/// group so that the child can update its layout (FS-PTY-009).
///
/// Strategy: spawn a shell that traps SIGWINCH and prints a marker, then resize
/// and verify the marker appears in the output.
///
/// The `while true; do sleep 1; done` loop keeps the shell as the foreground
/// process so that TIOCSWINSZ delivers SIGWINCH to the shell (not to a `sleep`
/// exec-optimised into the shell's place).
#[test]
fn fpl_r_005_resize_delivers_sigwinch_to_child() {
    let backend = LinuxPtyBackend::new();
    let mut session = backend
        .open_session(
            80,
            24,
            "/bin/sh",
            &[
                "-c",
                "trap 'echo SIGWINCH_RECEIVED' WINCH; echo READY; while true; do sleep 1; done",
            ],
            &[("TERM", "xterm-256color")],
        )
        .expect("open session");

    let reader = session
        .reader_handle()
        .expect("LinuxPtySession must have a reader");

    // Wait for the child to signal it's ready (trap is installed).
    let ready = read_until_timeout(reader.clone(), "READY", std::time::Duration::from_secs(5));
    assert!(
        ready.is_some(),
        "FPL-R-005: child must print READY before resize"
    );

    // Brief pause to ensure the shell has started its wait loop before we resize.
    std::thread::sleep(std::time::Duration::from_millis(100));

    // Resize the PTY — TIOCSWINSZ delivers SIGWINCH to the foreground process group.
    session.resize(120, 40, 0, 0).expect("resize must succeed");

    // The child's WINCH trap handler should print SIGWINCH_RECEIVED.
    let output = read_until_timeout(
        reader,
        "SIGWINCH_RECEIVED",
        std::time::Duration::from_secs(5),
    );
    assert!(
        output.is_some(),
        "FPL-R-005: child must print SIGWINCH_RECEIVED after PTY resize"
    );
}
