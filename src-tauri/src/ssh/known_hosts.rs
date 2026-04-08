// SPDX-License-Identifier: MPL-2.0

//! TauTerm known-hosts file management (module entry).
//!
//! See [`store`] for the full implementation.

mod store;

#[cfg(test)]
mod tests;

pub use store::{KnownHostEntry, KnownHostLookup, KnownHostsStore};
