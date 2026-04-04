// SPDX-License-Identifier: MPL-2.0

//! Resize debouncing for pane viewport changes.
//!
//! The frontend may fire many `resize_pane` IPC calls rapidly (e.g., during
//! window drag). A Tokio timer debounces these by 16–33ms; `TIOCSWINSZ` and
//! SIGWINCH are issued only once after the timer fires with the final size
//! (§6.5 of ARCHITECTURE.md, FS-PTY-010).

use std::time::Duration;

/// Debounce delay for resize events (§6.5).
pub const RESIZE_DEBOUNCE_MS: u64 = 16;

/// A pending resize operation.
#[derive(Debug, Clone, Copy)]
pub struct PendingResize {
    pub cols: u16,
    pub rows: u16,
    pub pixel_width: u16,
    pub pixel_height: u16,
}

/// A resize debouncer for a single pane.
///
/// Accepts resize requests and fires the callback after the debounce
/// interval, with only the most recent size.
pub struct ResizeDebouncer {
    sender: tokio::sync::watch::Sender<Option<PendingResize>>,
}

impl ResizeDebouncer {
    /// Create a new debouncer. `on_resize` is called after each debounce window.
    pub fn new<F>(on_resize: F) -> Self
    where
        F: Fn(PendingResize) + Send + 'static,
    {
        let (tx, mut rx) = tokio::sync::watch::channel(None::<PendingResize>);

        tokio::spawn(async move {
            loop {
                // Wait for a value to arrive.
                if rx.changed().await.is_err() {
                    break;
                }
                let resize = match *rx.borrow() {
                    Some(r) => r,
                    None => continue,
                };
                // Debounce: wait and see if a newer value arrives.
                tokio::time::sleep(Duration::from_millis(RESIZE_DEBOUNCE_MS)).await;
                // Re-read the latest value after debounce.
                let latest = match *rx.borrow() {
                    Some(r) => r,
                    None => resize,
                };
                on_resize(latest);
            }
        });

        Self { sender: tx }
    }

    /// Schedule a resize. Only the last value within the debounce window is applied.
    pub fn schedule(&self, resize: PendingResize) {
        let _ = self.sender.send(Some(resize));
    }
}
