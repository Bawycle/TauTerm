// SPDX-License-Identifier: MPL-2.0

//! Session module — tab and pane lifecycle management.
//!
//! Re-exports the public API types used by command handlers and the event system.
//! The `SessionRegistry` is the single source of truth for session topology,
//! injected as `State<Arc<SessionRegistry>>` by Tauri.

pub mod ids;
pub mod lifecycle;
pub mod output;
pub mod pane;
pub mod pty_task;
pub mod registry;
pub mod resize;
pub mod ssh_task;
pub mod tab;

#[cfg(feature = "e2e-testing")]
pub mod ssh_injectable;

pub use ids::{ConnectionId, PaneId, TabId};
pub use lifecycle::PaneLifecycleState;
pub use pane::{PaneSession, PaneState};
pub use registry::{CreateTabConfig, ScrollPositionState, SessionRegistry};
pub use tab::{PaneNode, SessionState, SplitDirection, TabState};
