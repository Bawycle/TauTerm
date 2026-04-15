// SPDX-License-Identifier: MPL-2.0

use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Language — MUST be an enum, never a free String (FS-I18N-006)
// ---------------------------------------------------------------------------

/// Supported UI languages. Extend this enum when adding new locales.
/// This type is used across IPC — it MUST NOT be replaced with a plain `String`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default, specta::Type)]
#[serde(rename_all = "camelCase")]
pub enum Language {
    #[default]
    En,
    Fr,
}
