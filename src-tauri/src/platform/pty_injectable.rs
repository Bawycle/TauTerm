// SPDX-License-Identifier: MPL-2.0

//! Injectable PTY backend — E2E testing only (`e2e-testing` feature).
//!
//! Provides `InjectablePtyBackend` and `InjectablePtySession`, which replace
//! the real `LinuxPtyBackend` in E2E test builds. Instead of spawning a real
//! shell, the backend creates an in-process mpsc channel. Test code pushes
//! synthetic bytes into the channel via `InjectableRegistry::send`, and the
//! existing PTY read task (`spawn_pty_read_task`) processes them through the
//! VT pipeline exactly as it would with real PTY output.
//!
//! See ADR-0015-e2e-injectable-pty.md for rationale and
//! ADR-0015-implementation-notes.md for the full design.

use std::io::{self, Read};
use std::sync::{Arc, Mutex};

use dashmap::DashMap;
use tokio::sync::mpsc;

use crate::error::PtyError;
use crate::platform::{PtyBackend, PtySession};
use crate::session::ids::PaneId;

// ---------------------------------------------------------------------------
// InjectableRegistry
// ---------------------------------------------------------------------------

/// Pane-to-sender map shared between the backend and the `inject_pty_output`
/// Tauri command. Managed as Tauri state in e2e-testing builds.
///
/// `DashMap` provides interior mutability with `Send + Sync`, so this struct
/// can be wrapped in `Arc` and shared across threads without an outer `Mutex`.
pub struct InjectableRegistry {
    senders: DashMap<PaneId, mpsc::UnboundedSender<Vec<u8>>>,
}

impl InjectableRegistry {
    /// Create an empty registry.
    pub fn new() -> Self {
        Self {
            senders: DashMap::new(),
        }
    }

    /// Register the sender for a pane. Replaces any existing entry.
    ///
    /// Called by `SessionRegistry::create_tab` / `split_pane` after the real
    /// pane ID is assigned (the injectable sender is extracted from the session
    /// via `PtySession::injectable_sender()` before the session is moved).
    pub fn register(&self, pane_id: PaneId, tx: mpsc::UnboundedSender<Vec<u8>>) {
        self.senders.insert(pane_id, tx);
    }

    /// Push bytes into the pane's VT pipeline.
    ///
    /// Returns `Err` if the pane ID is not found or the channel has closed
    /// (receiver dropped — pane's read task has already exited).
    pub fn send(&self, pane_id: &PaneId, data: Vec<u8>) -> Result<(), String> {
        match self.senders.get(pane_id) {
            None => Err(format!("pane not found: {pane_id}")),
            Some(tx) => tx
                .send(data)
                .map_err(|_| format!("channel closed for pane {pane_id}")),
        }
    }

    /// Remove the sender for a closed pane.
    ///
    /// Called by `SessionRegistry::close_pane`. Dropping the `UnboundedSender`
    /// causes `blocking_recv()` in `MpscReaderAdapter` to return `None` (EOF),
    /// which in turn causes the PTY read task to exit cleanly.
    pub fn remove(&self, pane_id: &PaneId) {
        self.senders.remove(pane_id);
    }
}

impl Default for InjectableRegistry {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// MpscReaderAdapter
// ---------------------------------------------------------------------------

/// Bridges a `tokio::sync::mpsc::UnboundedReceiver<Vec<u8>>` into
/// `std::io::Read` so the existing `spawn_pty_read_task` can consume
/// injectable bytes without modification.
///
/// This adapter is used exclusively inside `tokio::task::spawn_blocking`,
/// where blocking is safe. `blocking_recv()` is called to block the OS thread
/// until bytes are available or the sender is dropped (EOF).
///
/// The `leftover` buffer handles the case where a received chunk is larger
/// than the `buf` slice provided to a single `read()` call.
struct MpscReaderAdapter {
    rx: mpsc::UnboundedReceiver<Vec<u8>>,
    /// Bytes buffered from the last received chunk not yet consumed by `read()`.
    leftover: Vec<u8>,
}

impl MpscReaderAdapter {
    fn new(rx: mpsc::UnboundedReceiver<Vec<u8>>) -> Self {
        Self {
            rx,
            leftover: Vec::new(),
        }
    }
}

impl Read for MpscReaderAdapter {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        // 1. Drain leftover bytes from a previous oversized chunk.
        if !self.leftover.is_empty() {
            let n = buf.len().min(self.leftover.len());
            buf[..n].copy_from_slice(&self.leftover[..n]);
            self.leftover.drain(..n);
            return Ok(n);
        }

        // 2. Block until the next chunk arrives or the sender is dropped.
        //
        // `blocking_recv()` is correct here because this method is only ever
        // called from a `spawn_blocking` thread (see `spawn_pty_read_task`).
        // Do not use `try_recv` (busy-spin) or create a new Tokio runtime.
        match self.rx.blocking_recv() {
            None => {
                // Sender dropped — EOF signal to the read task loop.
                // The read task checks `Ok(0)` and returns cleanly (pty_task.rs:77).
                Ok(0)
            }
            Some(chunk) => {
                let n = buf.len().min(chunk.len());
                buf[..n].copy_from_slice(&chunk[..n]);
                if chunk.len() > n {
                    // Store the remainder for the next read() call.
                    self.leftover.extend_from_slice(&chunk[n..]);
                }
                Ok(n)
            }
        }
    }
}

// ---------------------------------------------------------------------------
// InjectablePtySession
// ---------------------------------------------------------------------------

/// An injectable PTY session for E2E testing.
///
/// Holds the channel sender (for cloning in `injectable_sender()`) and the
/// reader adapter wrapped behind `Arc<Mutex<...>>` (for `reader_handle()`).
/// The `write` method discards input; `resize` is a no-op; `close` is a no-op
/// (deregistration is handled by `SessionRegistry::close_pane`).
pub struct InjectablePtySession {
    /// Sender end — kept alive so the receiver does not see EOF prematurely.
    /// `SessionRegistry` extracts a clone via `injectable_sender()` and stores
    /// it in `InjectableRegistry` before this field is moved.
    tx: mpsc::UnboundedSender<Vec<u8>>,
    /// Shared reader handle passed to the PTY read task.
    reader: Arc<Mutex<Box<dyn Read + Send>>>,
}

impl PtySession for InjectablePtySession {
    fn reader_handle(&self) -> Option<Arc<Mutex<Box<dyn Read + Send>>>> {
        Some(self.reader.clone())
    }

    fn write(&mut self, _data: &[u8]) -> Result<(), PtyError> {
        // E2E sessions ignore keyboard input — tests inject output directly
        // via `inject_pty_output`. Silently discard.
        Ok(())
    }

    fn resize(
        &mut self,
        _cols: u16,
        _rows: u16,
        _pixel_width: u16,
        _pixel_height: u16,
    ) -> Result<(), PtyError> {
        // No real PTY to resize.
        Ok(())
    }

    fn close(self: Box<Self>) {
        // Nothing to do: deregistration (and therefore sender drop → EOF) is
        // handled by `SessionRegistry::close_pane` calling
        // `InjectableRegistry::remove`. Dropping `self` here drops `self.tx`
        // and `self.reader`; the Arc refcount on the reader may still be held
        // by the read task, which is correct and mirrors `LinuxPtySession`.
    }

    #[cfg(feature = "e2e-testing")]
    fn injectable_sender(&self) -> Option<mpsc::UnboundedSender<Vec<u8>>> {
        Some(self.tx.clone())
    }
}

// ---------------------------------------------------------------------------
// InjectablePtyBackend
// ---------------------------------------------------------------------------

/// PTY backend for E2E testing — creates injectable channel sessions instead
/// of real shell processes.
pub struct InjectablePtyBackend {
    registry: Arc<InjectableRegistry>,
}

impl PtyBackend for InjectablePtyBackend {
    fn open_session(
        &self,
        _cols: u16,
        _rows: u16,
        _command: &str,
        _args: &[&str],
        _env: &[(&str, &str)],
    ) -> Result<Box<dyn PtySession>, PtyError> {
        // Create the mpsc channel that bridges inject_pty_output → read task.
        let (tx, rx) = mpsc::unbounded_channel::<Vec<u8>>();

        // Wrap the receiver in the adapter, then box-erase the concrete type.
        // `MpscReaderAdapter` is `Send` because `UnboundedReceiver<Vec<u8>>` is
        // `Send` (T=Vec<u8> is Send) per tokio's guarantees.
        let adapter = MpscReaderAdapter::new(rx);
        let reader: Arc<Mutex<Box<dyn Read + Send>>> = Arc::new(Mutex::new(Box::new(adapter)));

        // Note: we do NOT pre-register the sender in `InjectableRegistry` here.
        // `open_session` does not receive the real PaneId (it is generated by
        // `SessionRegistry::create_tab` after this call returns). Instead, the
        // sender is exposed via `injectable_sender()` and extracted by
        // `SessionRegistry` before the session is moved into the pane.
        // See ADR-0015-implementation-notes.md §2.5 (Option F).
        let _ = &self.registry; // registry is used by InjectableRegistry::remove in close_pane

        Ok(Box::new(InjectablePtySession { tx, reader }))
    }
}

// ---------------------------------------------------------------------------
// Factory function
// ---------------------------------------------------------------------------

/// Create an `InjectablePtyBackend` sharing the given registry.
///
/// `lib.rs` calls this in e2e-testing builds, passing the same `Arc` that is
/// registered as Tauri state so `inject_pty_output` can reach the registry.
pub fn create_injectable_pty_backend(registry: Arc<InjectableRegistry>) -> InjectablePtyBackend {
    InjectablePtyBackend { registry }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Read;

    /// Verify that bytes pushed into the channel are readable via the adapter.
    #[test]
    fn mpsc_reader_adapter_delivers_bytes() {
        let (tx, rx) = mpsc::unbounded_channel::<Vec<u8>>();
        let mut adapter = MpscReaderAdapter::new(rx);

        tx.send(b"hello".to_vec()).unwrap();
        drop(tx); // EOF after first message

        let mut buf = [0u8; 16];
        let n = adapter.read(&mut buf).unwrap();
        assert_eq!(&buf[..n], b"hello");

        // Next read should return EOF
        let n2 = adapter.read(&mut buf).unwrap();
        assert_eq!(n2, 0, "dropped sender must yield EOF");
    }

    /// Verify that a chunk larger than buf is split correctly across two reads.
    #[test]
    fn mpsc_reader_adapter_handles_leftover() {
        let (tx, rx) = mpsc::unbounded_channel::<Vec<u8>>();
        let mut adapter = MpscReaderAdapter::new(rx);

        tx.send(b"abcdef".to_vec()).unwrap();
        drop(tx);

        let mut buf = [0u8; 3];
        let n1 = adapter.read(&mut buf).unwrap();
        assert_eq!(&buf[..n1], b"abc");

        let n2 = adapter.read(&mut buf).unwrap();
        assert_eq!(&buf[..n2], b"def");

        let n3 = adapter.read(&mut buf).unwrap();
        assert_eq!(n3, 0, "EOF after sender drop");
    }

    /// Verify that `open_session` returns a session with a working reader_handle.
    #[test]
    fn injectable_backend_open_session_returns_reader() {
        let registry = Arc::new(InjectableRegistry::new());
        let backend = create_injectable_pty_backend(registry);
        let session = backend
            .open_session(80, 24, "/bin/sh", &[], &[])
            .expect("open_session must succeed");
        assert!(
            session.reader_handle().is_some(),
            "injectable session must return Some(reader_handle)"
        );
    }

    /// Verify InjectableRegistry send/remove behaviour.
    #[test]
    fn injectable_registry_send_and_remove() {
        let registry = InjectableRegistry::new();
        let pane_id = PaneId::new();
        let (tx, mut rx) = mpsc::unbounded_channel::<Vec<u8>>();

        registry.register(pane_id.clone(), tx);
        registry
            .send(&pane_id, b"test".to_vec())
            .expect("send must succeed after register");

        let received = rx.try_recv().expect("must receive sent data");
        assert_eq!(received, b"test");

        registry.remove(&pane_id);
        let err = registry.send(&pane_id, b"x".to_vec());
        assert!(err.is_err(), "send after remove must return Err");
    }
}
