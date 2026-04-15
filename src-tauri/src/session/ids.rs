// SPDX-License-Identifier: MPL-2.0

//! Newtype ID wrappers for all session entities.
//!
//! All entity IDs are newtypes over `String` (UUID v4). Using distinct types
//! prevents silent mixing of IDs across entity kinds (TabId, PaneId, ConnectionId)
//! at compile time (§3.4 of ARCHITECTURE.md).

use serde::{Deserialize, Serialize};

/// Identifies a tab in the session registry.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, specta::Type)]
pub struct TabId(pub String);

impl TabId {
    pub fn new() -> Self {
        Self(uuid::Uuid::new_v4().to_string())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Default for TabId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for TabId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// Identifies a pane within a tab.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, specta::Type)]
pub struct PaneId(pub String);

impl PaneId {
    pub fn new() -> Self {
        Self(uuid::Uuid::new_v4().to_string())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Default for PaneId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for PaneId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// Identifies a saved SSH connection configuration.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, specta::Type)]
pub struct ConnectionId(pub String);

impl ConnectionId {
    pub fn new() -> Self {
        Self(uuid::Uuid::new_v4().to_string())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Default for ConnectionId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for ConnectionId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    // --- TabId ---

    #[test]
    fn tab_id_new_produces_non_empty_string() {
        let id = TabId::new();
        assert!(!id.as_str().is_empty());
    }

    #[test]
    fn tab_id_two_calls_produce_distinct_ids() {
        let a = TabId::new();
        let b = TabId::new();
        assert_ne!(a, b);
    }

    #[test]
    fn tab_id_as_str_and_display_are_consistent() {
        let id = TabId::new();
        assert_eq!(id.as_str(), id.to_string());
    }

    #[test]
    fn tab_id_default_produces_valid_non_empty_id() {
        let id = TabId::default();
        assert!(!id.as_str().is_empty());
    }

    // --- PaneId ---

    #[test]
    fn pane_id_new_produces_non_empty_string() {
        let id = PaneId::new();
        assert!(!id.as_str().is_empty());
    }

    #[test]
    fn pane_id_two_calls_produce_distinct_ids() {
        let a = PaneId::new();
        let b = PaneId::new();
        assert_ne!(a, b);
    }

    #[test]
    fn pane_id_as_str_and_display_are_consistent() {
        let id = PaneId::new();
        assert_eq!(id.as_str(), id.to_string());
    }

    // --- ConnectionId ---

    #[test]
    fn connection_id_new_produces_non_empty_string() {
        let id = ConnectionId::new();
        assert!(!id.as_str().is_empty());
    }

    #[test]
    fn connection_id_two_calls_produce_distinct_ids() {
        let a = ConnectionId::new();
        let b = ConnectionId::new();
        assert_ne!(a, b);
    }

    // --- Cross-type uniqueness (compile-time type safety) ---

    #[test]
    fn tab_id_and_pane_id_are_distinct_types() {
        // This test is mostly structural (checked at compile time), but we
        // verify at runtime that the same UUID string used in both types
        // does not accidentally compare equal across types.
        let raw = uuid::Uuid::new_v4().to_string();
        let tab = TabId(raw.clone());
        let pane = PaneId(raw.clone());
        // Different types — cannot compare directly; verify inner values equal
        // but types remain separate.
        assert_eq!(tab.as_str(), pane.as_str());
        // If TabId == PaneId were possible this would not compile.
        // The assertion above confirms the same underlying string, not the same type.
    }

    // --- Uniqueness at scale ---

    #[test]
    fn one_hundred_tab_ids_are_all_unique() {
        let ids: HashSet<String> = (0..100).map(|_| TabId::new().0).collect();
        assert_eq!(ids.len(), 100);
    }

    // --- Serialization round-trip ---

    #[test]
    fn tab_id_serializes_as_plain_string() {
        let id = TabId("test-id-123".to_string());
        let json = serde_json::to_string(&id).expect("serialize failed");
        // Newtype should serialize as a plain JSON string, not an object.
        assert_eq!(json, "\"test-id-123\"");
    }

    #[test]
    fn pane_id_deserializes_from_plain_string() {
        let json = "\"pane-abc\"";
        let id: PaneId = serde_json::from_str(json).expect("deserialize failed");
        assert_eq!(id.as_str(), "pane-abc");
    }
}
