<!-- SPDX-License-Identifier: MPL-2.0 -->

# ADR-0009 — Pane layout structure: flat list with split metadata vs. recursive tree

**Date:** 2026-04-04
**Status:** Accepted

## Context

TauTerm supports splitting a tab into multiple panes (FS-PANE-001 through FS-PANE-006). Each split produces two panes sharing the space of the original, either horizontally or vertically. Splits can be nested: a pane can be split again, producing a tree of arbitrary depth.

The IPC contract must carry pane layout information in both directions: `split_pane` and `close_pane` return the updated layout; `get_session_state` returns a full snapshot; `session-state-changed` events carry the updated tab layout. The representation of this layout in the IPC payload is a core design decision.

Two representations were evaluated:

**Option A: Flat list with split metadata per pane**
`TabState` contains a flat `PaneState[]` array. Each `PaneState` carries fields such as `splitDirection: 'horizontal' | 'vertical' | null` and `splitRatio: number` alongside a reference to its sibling or parent. This is the model initially sketched in UXD §15.1.

**Option B: Recursive tree (PaneNode)**
`TabState.layout` is a `PaneNode` union type: either a `leaf` (a terminal pane) or a `split` node containing two `PaneNode` children and a direction and ratio. The tree structure directly encodes the topology.

## Decision

Use **Option B — a recursive `PaneNode` tree** as the IPC layout representation.

The Rust type is:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum PaneNode {
    Leaf { pane_id: PaneId, state: PaneState },
    Split {
        direction: SplitDirection,
        ratio: f32,
        first: Box<PaneNode>,
        second: Box<PaneNode>,
    },
}
```

The TypeScript mirror is:

```typescript
type PaneNode =
  | { type: 'leaf'; paneId: PaneId; state: PaneState }
  | { type: 'split'; direction: 'horizontal' | 'vertical'; ratio: number;
      first: PaneNode; second: PaneNode };
```

`TabState.layout: PaneNode` replaces the `panes: PaneState[]` field from UXD §15.1.

The frontend `split-tree.ts` module consumes this type directly: `buildFromPaneNode()` reconstructs the layout tree; `findLeaf(id)` locates panes by ID; `updateRatio()` updates split ratios after drag-resize.

## Alternatives considered

**Flat list with parent references**
Each `PaneState` includes a `parentId: PaneId | null` and `siblingId: PaneId | null`. The tree is reconstructed by the frontend from these references. Problem: the order of entries in the flat list is ambiguous when a parent has two children — which is `first` and which is `second`? An index field is needed. Then, reconstructing the tree requires a traversal of the flat list with a map lookup per node, which is O(n log n) and fragile: any inconsistency in the parent/sibling references produces an unrecoverable layout. Not chosen: the flat list cannot represent a split tree of depth > 1 unambiguously without essentially encoding the tree structure anyway.

**Flat list with ordering convention**
Entries are ordered depth-first, left-to-right. The consumer reconstructs the tree by parsing the ordering. This is implicit and brittle: a reordering or insertion at the wrong index corrupts the layout silently. Not chosen.

**A layout engine type (e.g., Golden Layout or Mosaic)**
Importing an external layout model would introduce a large dependency and a non-trivial serialization contract. For TauTerm's v1 use case (binary splits only, no floating panes, no tabs-within-panes), the recursive tree is sufficient and self-contained. Not chosen.

## Consequences

**Positive:**
- The tree structure directly encodes the topology. There is no ambiguity: the `first` and `second` children of a `Split` node are unambiguous; their direction and ratio are co-located.
- The TypeScript `split-tree.ts` module operates on the tree natively — no reconstruction step needed.
- Serialization via `serde` with `#[serde(tag = "type")]` is straightforward and produces clean JSON readable by humans.
- Nested splits of arbitrary depth are representable without schema changes.

**Negative / risks:**
- The recursive type requires `Box<PaneNode>` in Rust to avoid an infinite-size type. This is idiomatic in Rust for recursive data structures.
- The UXD §15.1 flat `panes: PaneState[]` definition is superseded. Any code or documentation referencing the flat model must be updated (ARCHITECTURE.md §4.5.1, §4.6).
- Drag-and-drop pane reordering (moving a pane to an arbitrary position in the tree) requires a tree mutation that is more complex than reordering a flat array. This feature is out of scope for v1 (acceptable v1 limitation).

## Notes

This decision supersedes UXD §15.1 (`TabState.panes: PaneState[]`). The flat model is not supported in the IPC contract. Implementers must use `TabState.layout: PaneNode`. See ARCHITECTURE.md §4.5.1 and §4.6 for the full supersession table.
