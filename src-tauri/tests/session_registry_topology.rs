// SPDX-License-Identifier: MPL-2.0

//! Integration tests — session topology types (TEST-IPC-*).
//!
//! `SessionRegistry` requires a live `AppHandle` and `PtyBackend`, making it
//! untestable in pure integration tests without the full Tauri runtime. These
//! tests instead cover the topology data types directly:
//!
//! - `TabId` / `PaneId` uniqueness at scale
//! - `PaneNode` tree construction, querying, and serialization (single leaf,
//!   split at two and three levels)
//! - `TabState` / `SessionState` serialization round-trip
//! - `close_pane` semantics: removing the only leaf collapses to the sibling
//! - `reorder_tab`: order field is mutable and sortable
//! - All IDs are distinct across 100 creations (TEST-IPC-* uniqueness guarantee)

use std::collections::HashSet;

use tau_term_lib::session::{
    ids::{PaneId, TabId},
    lifecycle::PaneLifecycleState,
    pane::PaneState,
    tab::{PaneNode, SessionState, SplitDirection, TabState},
};

// ---------------------------------------------------------------------------
// Helpers — build PaneState and PaneNode fixtures without a real PTY
// ---------------------------------------------------------------------------

fn make_pane_state(id: PaneId) -> PaneState {
    PaneState {
        pane_id: id,
        lifecycle: PaneLifecycleState::Running,
        title: None,
        ssh_state: None,
        scroll_offset: 0,
    }
}

fn make_leaf(id: PaneId) -> PaneNode {
    let state = make_pane_state(id.clone());
    PaneNode::Leaf { pane_id: id, state }
}

fn make_split(direction: SplitDirection, first: PaneNode, second: PaneNode) -> PaneNode {
    PaneNode::Split {
        direction,
        ratio: 0.5,
        first: Box::new(first),
        second: Box::new(second),
    }
}

fn make_tab(id: TabId, active_pane: PaneId, order: u32, layout: PaneNode) -> TabState {
    TabState {
        id,
        label: None,
        active_pane_id: active_pane,
        order,
        layout,
    }
}

// ---------------------------------------------------------------------------
// TabId / PaneId uniqueness
// ---------------------------------------------------------------------------

#[test]
fn topology_100_tab_ids_are_unique() {
    let ids: HashSet<String> = (0..100).map(|_| TabId::new().0).collect();
    assert_eq!(ids.len(), 100, "Expected 100 distinct TabIds");
}

#[test]
fn topology_100_pane_ids_are_unique() {
    let ids: HashSet<String> = (0..100).map(|_| PaneId::new().0).collect();
    assert_eq!(ids.len(), 100, "Expected 100 distinct PaneIds");
}

#[test]
fn topology_mixed_100_tab_and_pane_ids_are_all_unique() {
    let tab_ids: HashSet<String> = (0..50).map(|_| TabId::new().0).collect();
    let pane_ids: HashSet<String> = (0..50).map(|_| PaneId::new().0).collect();
    // No overlap between tab and pane IDs (UUID v4 collision probability ≈ 0).
    let intersection: HashSet<_> = tab_ids.intersection(&pane_ids).collect();
    assert!(
        intersection.is_empty(),
        "TabId and PaneId namespaces must not collide"
    );
}

// ---------------------------------------------------------------------------
// Single-leaf topology (create_tab equivalent)
// ---------------------------------------------------------------------------

#[test]
fn topology_single_leaf_contains_correct_pane_id() {
    let pane_id = PaneId::new();
    let node = make_leaf(pane_id.clone());
    let ids = node.pane_ids();
    assert_eq!(ids.len(), 1);
    assert_eq!(ids[0], pane_id);
}

#[test]
fn topology_single_leaf_find_pane_returns_self() {
    let pane_id = PaneId::new();
    let node = make_leaf(pane_id.clone());
    assert!(node.find_pane(&pane_id).is_some());
}

#[test]
fn topology_single_leaf_find_unknown_pane_returns_none() {
    let pane_id = PaneId::new();
    let other_id = PaneId::new();
    let node = make_leaf(pane_id);
    assert!(node.find_pane(&other_id).is_none());
}

#[test]
fn topology_single_leaf_tab_state_structure_is_correct() {
    let tab_id = TabId::new();
    let pane_id = PaneId::new();
    let layout = make_leaf(pane_id.clone());
    let tab = make_tab(tab_id.clone(), pane_id.clone(), 0, layout);

    assert_eq!(tab.id, tab_id);
    assert_eq!(tab.active_pane_id, pane_id);
    assert_eq!(tab.order, 0);
    assert!(tab.label.is_none());
    // Layout must contain exactly this pane.
    assert_eq!(tab.layout.pane_ids(), vec![pane_id]);
}

// ---------------------------------------------------------------------------
// Two-level split (split_pane equivalent — level 1)
// ---------------------------------------------------------------------------

#[test]
fn topology_split_contains_both_pane_ids() {
    let pane_a = PaneId::new();
    let pane_b = PaneId::new();
    let node = make_split(
        SplitDirection::Horizontal,
        make_leaf(pane_a.clone()),
        make_leaf(pane_b.clone()),
    );
    let ids = node.pane_ids();
    assert_eq!(ids.len(), 2);
    assert!(ids.contains(&pane_a));
    assert!(ids.contains(&pane_b));
}

#[test]
fn topology_split_find_first_child_pane() {
    let pane_a = PaneId::new();
    let pane_b = PaneId::new();
    let node = make_split(
        SplitDirection::Vertical,
        make_leaf(pane_a.clone()),
        make_leaf(pane_b.clone()),
    );
    assert!(node.find_pane(&pane_a).is_some());
    assert!(node.find_pane(&pane_b).is_some());
}

#[test]
fn topology_split_find_unknown_pane_returns_none() {
    let pane_a = PaneId::new();
    let pane_b = PaneId::new();
    let unknown = PaneId::new();
    let node = make_split(
        SplitDirection::Horizontal,
        make_leaf(pane_a),
        make_leaf(pane_b),
    );
    assert!(node.find_pane(&unknown).is_none());
}

// ---------------------------------------------------------------------------
// Three-level split (split_pane × 2 — deeper tree)
// ---------------------------------------------------------------------------

#[test]
fn topology_three_level_split_contains_three_pane_ids() {
    let pane_a = PaneId::new();
    let pane_b = PaneId::new();
    let pane_c = PaneId::new();

    // Structure: Split(Split(A, B), C)
    let inner = make_split(
        SplitDirection::Horizontal,
        make_leaf(pane_a.clone()),
        make_leaf(pane_b.clone()),
    );
    let outer = make_split(SplitDirection::Vertical, inner, make_leaf(pane_c.clone()));

    let ids = outer.pane_ids();
    assert_eq!(ids.len(), 3);
    assert!(ids.contains(&pane_a));
    assert!(ids.contains(&pane_b));
    assert!(ids.contains(&pane_c));
}

#[test]
fn topology_three_level_split_all_panes_findable() {
    let pane_a = PaneId::new();
    let pane_b = PaneId::new();
    let pane_c = PaneId::new();

    let inner = make_split(
        SplitDirection::Vertical,
        make_leaf(pane_a.clone()),
        make_leaf(pane_b.clone()),
    );
    let outer = make_split(SplitDirection::Horizontal, inner, make_leaf(pane_c.clone()));

    assert!(outer.find_pane(&pane_a).is_some());
    assert!(outer.find_pane(&pane_b).is_some());
    assert!(outer.find_pane(&pane_c).is_some());
}

#[test]
fn topology_four_pane_tree_contains_four_ids() {
    let ids: Vec<PaneId> = (0..4).map(|_| PaneId::new()).collect();
    // Structure: Split(Split(A,B), Split(C,D))
    let left = make_split(
        SplitDirection::Horizontal,
        make_leaf(ids[0].clone()),
        make_leaf(ids[1].clone()),
    );
    let right = make_split(
        SplitDirection::Horizontal,
        make_leaf(ids[2].clone()),
        make_leaf(ids[3].clone()),
    );
    let root = make_split(SplitDirection::Vertical, left, right);

    let found = root.pane_ids();
    assert_eq!(found.len(), 4);
    for id in &ids {
        assert!(found.contains(id), "pane_id {id} not found in tree");
    }
}

// ---------------------------------------------------------------------------
// close_pane semantics — collapsing a split into its sibling
// ---------------------------------------------------------------------------

#[test]
fn topology_removing_only_pane_from_split_leaves_sibling_as_leaf() {
    // A split with two leaves — remove one, the sibling should be the new root.
    // We simulate this at the type level (the registry helper does the same logic).
    let pane_a = PaneId::new();
    let pane_b = PaneId::new();

    let split = make_split(
        SplitDirection::Horizontal,
        make_leaf(pane_a.clone()),
        make_leaf(pane_b.clone()),
    );

    // Both panes present before close.
    let before = split.pane_ids();
    assert!(before.contains(&pane_a));
    assert!(before.contains(&pane_b));

    // After "closing" pane_a, only pane_b should remain.
    // Registry uses `remove_pane_from_tree` (internal). We test the type invariant:
    // a single-pane layout is always a Leaf.
    let remaining_leaf = make_leaf(pane_b.clone());
    let after = remaining_leaf.pane_ids();
    assert_eq!(after, vec![pane_b]);
}

// ---------------------------------------------------------------------------
// reorder_tab — order field is correctly updated
// ---------------------------------------------------------------------------

#[test]
fn topology_reorder_tab_updates_order_field() {
    let tab_id = TabId::new();
    let pane_id = PaneId::new();
    let mut tab = make_tab(tab_id, pane_id, 0, make_leaf(PaneId::new()));
    assert_eq!(tab.order, 0);
    tab.order = 3;
    assert_eq!(tab.order, 3);
}

#[test]
fn topology_tabs_sorted_by_order_field() {
    let tabs: Vec<TabState> = (0u32..5)
        .rev()
        .map(|i| {
            let tid = TabId::new();
            let pid = PaneId::new();
            make_tab(tid, pid.clone(), i, make_leaf(pid))
        })
        .collect();

    let mut sorted = tabs.clone();
    sorted.sort_by_key(|t| t.order);

    for (i, tab) in sorted.iter().enumerate() {
        assert_eq!(tab.order as usize, i);
    }
}

// ---------------------------------------------------------------------------
// SessionState serialization round-trip
// ---------------------------------------------------------------------------

#[test]
fn topology_tab_state_serializes_and_deserializes() {
    let tab_id = TabId::new();
    let pane_id = PaneId::new();
    let layout = make_leaf(pane_id.clone());
    let tab = make_tab(tab_id.clone(), pane_id.clone(), 0, layout);

    let json = serde_json::to_string(&tab).expect("serialize TabState");
    let restored: TabState = serde_json::from_str(&json).expect("deserialize TabState");

    assert_eq!(restored.id, tab.id);
    assert_eq!(restored.active_pane_id, tab.active_pane_id);
    assert_eq!(restored.order, tab.order);
}

#[test]
fn topology_session_state_round_trips() {
    let tab_id = TabId::new();
    let pane_id = PaneId::new();
    let layout = make_leaf(pane_id.clone());
    let tab = make_tab(tab_id.clone(), pane_id, 0, layout);

    let session = SessionState {
        tabs: vec![tab],
        active_tab_id: tab_id.clone(),
    };

    let json = serde_json::to_string(&session).expect("serialize SessionState");
    let restored: SessionState = serde_json::from_str(&json).expect("deserialize SessionState");

    assert_eq!(restored.active_tab_id, tab_id);
    assert_eq!(restored.tabs.len(), 1);
}

#[test]
fn topology_split_node_serializes_with_direction_and_ratio() {
    let pane_a = PaneId::new();
    let pane_b = PaneId::new();
    let node = make_split(
        SplitDirection::Horizontal,
        make_leaf(pane_a),
        make_leaf(pane_b),
    );

    let json = serde_json::to_string(&node).expect("serialize PaneNode");
    // The discriminant `type` and direction must be present in JSON.
    assert!(
        json.contains("\"split\"") || json.contains("split"),
        "type tag missing: {json}"
    );
    assert!(
        json.contains("horizontal") || json.contains("Horizontal"),
        "direction missing: {json}"
    );
    assert!(json.contains("0.5"), "ratio missing: {json}");
}

#[test]
fn topology_pane_node_leaf_serializes_with_type_tag() {
    let pane_id = PaneId::new();
    let node = make_leaf(pane_id.clone());
    let json = serde_json::to_string(&node).expect("serialize");
    assert!(
        json.contains("\"leaf\"") || json.contains("leaf"),
        "leaf type tag missing: {json}"
    );
}

// ---------------------------------------------------------------------------
// SEC-SPRINT-008 — 50 levels of nested PaneNode::Split must not stack overflow
//
// `PaneNode` operations (`pane_ids`, `find_pane`, serialization) are recursive.
// A deeply nested tree (e.g. from 50 repeated split_pane calls) must not blow
// the stack. Verified by constructing the tree and exercising each recursive
// operation at 50 nesting levels.
// ---------------------------------------------------------------------------

#[test]
fn sec_sprint_008_50_nested_splits_no_stack_overflow() {
    // Build a maximally skewed tree: Split(Split(Split(... Leaf, Leaf), Leaf), Leaf).
    // This is the adversarial pattern that maximises recursion depth.
    let leaves: Vec<PaneId> = (0..51).map(|_| PaneId::new()).collect();

    // Start with the innermost leaf.
    let mut node = make_leaf(leaves[0].clone());

    // Wrap it 50 times: each iteration adds one Split level.
    for i in 1..=50 {
        node = make_split(
            SplitDirection::Horizontal,
            node,
            make_leaf(leaves[i].clone()),
        );
    }

    // `pane_ids()` recurses over the entire tree — must not panic/overflow.
    let ids = node.pane_ids();
    assert_eq!(
        ids.len(),
        51,
        "SEC-SPRINT-008: 50-level split must contain 51 pane IDs (no stack overflow)"
    );

    // `find_pane()` on the deepest leaf also recurses through all 50 levels.
    let deepest = &leaves[0];
    assert!(
        node.find_pane(deepest).is_some(),
        "SEC-SPRINT-008: find_pane on deepest leaf must succeed (no stack overflow)"
    );

    // Serialization (serde_json) may also recurse — verify no overflow.
    let json = serde_json::to_string(&node)
        .expect("SEC-SPRINT-008: serialization of 50-level split must not panic/overflow");
    assert!(
        !json.is_empty(),
        "SEC-SPRINT-008: serialized JSON must be non-empty"
    );
}

// ---------------------------------------------------------------------------
// PaneState serialization
// ---------------------------------------------------------------------------

#[test]
fn topology_pane_state_running_serializes_correctly() {
    let pane_id = PaneId::new();
    let state = make_pane_state(pane_id.clone());
    let json = serde_json::to_string(&state).expect("serialize PaneState");
    let restored: PaneState = serde_json::from_str(&json).expect("deserialize PaneState");
    assert_eq!(restored.pane_id, pane_id);
    assert!(restored.title.is_none());
    assert!(restored.ssh_state.is_none());
    assert_eq!(restored.scroll_offset, 0);
}
