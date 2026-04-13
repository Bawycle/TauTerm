// SPDX-License-Identifier: MPL-2.0

//! Layout tree manipulation helpers: split, title update, and pane removal.

use crate::session::{
    ids::PaneId,
    tab::{PaneNode, SplitDirection},
};

// ---------------------------------------------------------------------------
// Layout tree helpers
// ---------------------------------------------------------------------------

/// Replace the leaf node for `target_id` with a split containing
/// the existing pane (first) and a new pane (second).
pub(super) fn replace_leaf_with_split(
    node: PaneNode,
    target_id: &PaneId,
    new_id: PaneId,
    new_state: crate::session::pane::PaneState,
    existing_state: crate::session::pane::PaneState,
    direction: SplitDirection,
) -> PaneNode {
    match node {
        PaneNode::Leaf { pane_id, .. } if &pane_id == target_id => PaneNode::Split {
            direction,
            ratio: 0.5,
            first: Box::new(PaneNode::Leaf {
                pane_id: pane_id.clone(),
                state: existing_state,
            }),
            second: Box::new(PaneNode::Leaf {
                pane_id: new_id,
                state: new_state,
            }),
        },
        PaneNode::Leaf { .. } => node,
        PaneNode::Split {
            direction: d,
            ratio,
            first,
            second,
        } => PaneNode::Split {
            direction: d,
            ratio,
            first: Box::new(replace_leaf_with_split(
                *first,
                target_id,
                new_id.clone(),
                new_state.clone(),
                existing_state.clone(),
                direction,
            )),
            second: Box::new(replace_leaf_with_split(
                *second,
                target_id,
                new_id,
                new_state,
                existing_state,
                direction,
            )),
        },
    }
}

/// Update the `PaneState.title` for a specific pane in the layout tree in-place.
pub(super) fn update_pane_title_in_tree(node: &mut PaneNode, target_id: &PaneId, title: &str) {
    match node {
        PaneNode::Leaf { pane_id, state } if pane_id == target_id => {
            state.title = Some(title.to_string());
        }
        PaneNode::Leaf { .. } => {}
        PaneNode::Split { first, second, .. } => {
            update_pane_title_in_tree(first, target_id, title);
            update_pane_title_in_tree(second, target_id, title);
        }
    }
}

/// Update the `PaneState.cwd` for a specific pane in the layout tree in-place.
pub(super) fn update_pane_cwd_in_tree(node: &mut PaneNode, target_id: &PaneId, cwd: &str) {
    match node {
        PaneNode::Leaf { pane_id, state } if pane_id == target_id => {
            state.cwd = Some(cwd.to_string());
        }
        PaneNode::Leaf { .. } => {}
        PaneNode::Split { first, second, .. } => {
            update_pane_cwd_in_tree(first, target_id, cwd);
            update_pane_cwd_in_tree(second, target_id, cwd);
        }
    }
}

/// Update the `PaneState.label` for a specific pane in the layout tree in-place.
pub(super) fn update_pane_label_in_tree(
    node: &mut PaneNode,
    target_id: &PaneId,
    label: Option<String>,
) {
    match node {
        PaneNode::Leaf { pane_id, state } if pane_id == target_id => {
            state.label = label;
        }
        PaneNode::Leaf { .. } => {}
        PaneNode::Split { first, second, .. } => {
            update_pane_label_in_tree(first, target_id, label.clone());
            update_pane_label_in_tree(second, target_id, label);
        }
    }
}

/// Remove the leaf for `target_id`, collapsing its sibling upward.
pub(super) fn remove_pane_from_tree(node: PaneNode, target_id: &PaneId) -> PaneNode {
    match node {
        PaneNode::Leaf { ref pane_id, .. } if pane_id == target_id => {
            // Caller ensures there is at least one other pane — this case
            // should not be reached at the top level.
            node
        }
        PaneNode::Leaf { .. } => node,
        PaneNode::Split {
            direction,
            ratio,
            first,
            second,
        } => {
            let first_ids = first.pane_ids();
            let second_ids = second.pane_ids();

            if first_ids.contains(target_id) && first_ids.len() == 1 {
                // First child is the sole target — collapse to second.
                *second
            } else if second_ids.contains(target_id) && second_ids.len() == 1 {
                // Second child is the sole target — collapse to first.
                *first
            } else {
                PaneNode::Split {
                    direction,
                    ratio,
                    first: Box::new(remove_pane_from_tree(*first, target_id)),
                    second: Box::new(remove_pane_from_tree(*second, target_id)),
                }
            }
        }
    }
}
