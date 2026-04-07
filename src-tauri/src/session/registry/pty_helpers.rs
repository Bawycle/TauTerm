// SPDX-License-Identifier: MPL-2.0

//! PTY reader/writer handle extraction helpers.

use crate::platform::PtySession;

// ---------------------------------------------------------------------------
// PTY reader/writer extraction
// ---------------------------------------------------------------------------

/// Extract a reader handle from a `Box<dyn PtySession>` for the read task.
///
/// Delegates to the `PtySession::reader_handle()` trait method, which each
/// platform backend implements. No unsafe downcast needed.
pub(super) fn get_reader_handle(
    pty: &dyn PtySession,
) -> Option<std::sync::Arc<std::sync::Mutex<Box<dyn std::io::Read + Send>>>> {
    pty.reader_handle()
}

/// Extract a writer handle from a `Box<dyn PtySession>` for the read task.
///
/// Used by Task 1 to write DSR/DA/CPR responses back to the PTY master after
/// releasing the `VtProcessor` write-lock. Sessions that do not support
/// back-writes (e.g. injectable E2E sessions) return `None`.
pub(super) fn get_writer_handle(
    pty: &dyn PtySession,
) -> Option<std::sync::Arc<std::sync::Mutex<Box<dyn std::io::Write + Send>>>> {
    pty.writer_handle()
}
