// SPDX-License-Identifier: MPL-2.0

//! Windows PTY backend stub — not supported in v1.

use crate::error::PtyError;
use crate::platform::{PtyBackend, PtySession};

pub struct WindowsPtyBackend {}

impl WindowsPtyBackend {
    pub fn new() -> Self {
        Self {}
    }
}

impl PtyBackend for WindowsPtyBackend {
    fn open_session(
        &self,
        _cols: u16,
        _rows: u16,
        _command: &str,
        _args: &[&str],
        _env: &[(&str, &str)],
    ) -> Result<Box<dyn PtySession>, PtyError> {
        unimplemented!("Windows PTY backend not supported in v1")
    }
}
