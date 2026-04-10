// SPDX-License-Identifier: MPL-2.0

//! macOS PTY backend stub — not supported in v1 (§1.3 of ARCHITECTURE.md).

use crate::error::PtyError;
use crate::platform::{PtyBackend, PtySession};

#[derive(Default)]
pub struct MacOsPtyBackend {}

impl MacOsPtyBackend {
    pub fn new() -> Self {
        Self {}
    }
}

impl PtyBackend for MacOsPtyBackend {
    fn open_session(
        &self,
        _cols: u16,
        _rows: u16,
        _pixel_width: u16,
        _pixel_height: u16,
        _command: &str,
        _args: &[&str],
        _env: &[(&str, &str)],
        _working_directory: Option<&std::path::Path>,
    ) -> Result<Box<dyn PtySession>, PtyError> {
        unimplemented!("macOS PTY backend not supported in v1")
    }
}
