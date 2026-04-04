// SPDX-License-Identifier: MPL-2.0

//! Tauri command handlers — re-exports all handler functions for `generate_handler![]`.
//!
//! Every `#[tauri::command]` function is defined in a sub-module and
//! re-exported here so `lib.rs` can register them all in one place.

pub mod connection_cmds;
pub mod input_cmds;
pub mod preferences_cmds;
pub mod session_cmds;
pub mod ssh_cmds;
pub mod ssh_prompt_cmds;
pub mod system_cmds;

pub use connection_cmds::*;
pub use input_cmds::*;
pub use preferences_cmds::*;
pub use session_cmds::*;
pub use ssh_cmds::*;
pub use ssh_prompt_cmds::*;
pub use system_cmds::*;
