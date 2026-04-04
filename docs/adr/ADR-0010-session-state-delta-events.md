<!-- SPDX-License-Identifier: MPL-2.0 -->

# ADR-0010 — `session-state-changed` event: complete TabState vs. partial diff

**Date:** 2026-04-04
**Status:** Accepted

## Context

The backend emits `session-state-changed` events to notify the frontend of topology changes: tab creation, tab closure, tab rename, active tab change, pane process exit, OSC-driven title change, etc. The frontend maintains a replica of `SessionState` for rendering the tab bar and pane tree. When a `session-state-changed` event arrives, the frontend must update its replica.

The event payload design affects both serialization cost and frontend implementation complexity. Three options were considered:

**Option A: Complete `TabState` of the affected tab**
The event carries the full `TabState` — including the complete `PaneNode` tree — of the one tab that changed. The frontend atomically replaces its replica of that tab.

**Option B: RFC 6902 JSON Patch**
The event carries a JSON Patch document (`[{ op, path, value }]`) describing the minimal diff from the previous state to the new state. The frontend applies the patch to its replica.

**Option C: Full `SessionState` snapshot**
The event carries the entire `SessionState` (all tabs, all panes). The frontend replaces its entire replica.

## Decision

Use **Option A — the complete `TabState` of the affected tab**.

The `SessionStateChanged` event payload is:

```typescript
interface SessionStateChanged {
  changeType: 'tab-created' | 'tab-closed' | 'tab-reordered'
    | 'active-tab-changed' | 'active-pane-changed'
    | 'pane-metadata-changed';
  // Present for all changeTypes except 'tab-closed'.
  // Contains the complete, updated TabState of the affected tab.
  tab?: TabState;
  // Present when changeType is 'active-tab-changed' or 'tab-closed'.
  activeTabId?: string;
}
```

On receiving the event, the frontend:
- For `tab-closed`: removes the tab with `activeTabId` from its replica.
- For all other types: atomically replaces its replica of `tab.id` with the received `tab`.

## Alternatives considered

**Option B: RFC 6902 JSON Patch**
JSON Patch is a well-specified standard for partial updates. However, applying a JSON Patch to a TypeScript union type (`PaneNode` is a recursive union) requires a general-purpose deep-merge implementation that understands union type semantics. The ambiguity problem: when a patch operation `replace` targets a path into the `PaneNode` tree (e.g., `/layout/first/state/title`), the frontend must correctly navigate the tree despite the union type structure. This is non-trivial to implement correctly and is a surface for subtle bugs. Rejected: implementation complexity exceeds the benefit at v1 scale.

**Option C: Full `SessionState` snapshot**
Sending the full `SessionState` on every topology change is semantically clear and trivially correct. The problem: it couples event frequency to total session size. With 20 open tabs and 40 panes, every tab title change (which is frequent — shells set the title on every directory change) triggers a full serialization of all 20 tabs. This grows linearly with session size and unboundedly with long-running sessions. Rejected: unbounded payload growth is architecturally unacceptable.

## Consequences

**Positive:**
- The frontend update logic is simple: locate the tab by ID in the replica, replace it atomically. No deep merge, no patch application, no conflict resolution.
- The payload is semantically unambiguous. A `TabState` with its full `PaneNode` tree defines the topology completely; there is no question of what a missing field means.
- The serialization cost is bounded and predictable: a tab with 1–8 panes serializes to ≤ 2 KB of JSON. This is negligible at IPC event rates.
- The event is not emitted for `split_pane` or `close_pane` — those commands return the updated `TabState` directly (see ARCHITECTURE.md §4.5.2). This avoids redundant events for the most frequent topology mutations.

**Negative / risks:**
- The payload is larger than a targeted diff for small changes (e.g., title change in a tab with many panes). In practice, the overhead is < 2 KB per event, which is inconsequential on any modern system.
- The frontend must ensure that its replica update is atomic: it must not render an intermediate state between receiving the event and applying the full replacement. Svelte 5 runes (`$state`) provide this guarantee when the update is performed in a single synchronous assignment.

## Notes

`session-state-changed` is emitted only for topology changes that originate asynchronously or outside a direct user command. It is not emitted for `split_pane` or `close_pane` (those return `TabState | null` in their command response). See ARCHITECTURE.md §4.5.2 for the full emit conditions.
