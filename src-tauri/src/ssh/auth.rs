// SPDX-License-Identifier: MPL-2.0

//! SSH authentication sequence.
//!
//! Tries authentication methods in order: public-key → keyboard-interactive → password.
//! Credential prompts are sent to the frontend via `credential-prompt` events.
//!
//! Full implementation requires `russh` client integration.

// Stub — authentication logic will be implemented in the SSH integration pass.
