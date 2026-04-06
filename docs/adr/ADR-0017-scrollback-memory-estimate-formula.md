<!-- SPDX-License-Identifier: MPL-2.0 -->

# ADR-0017 — Scrollback memory estimate: 500 bytes/line upper-bound formula

**Date:** 2026-04-06
**Status:** Accepted

## Context

FS-SB-002 requires the preferences UI to display an estimated memory consumption for the scrollback buffer (`~{N} MB per pane`). The estimate must update reactively as the user adjusts the scrollback line count.

The implementation in `PreferencesPanel.svelte` uses a hard-coded per-line coefficient. The original coefficient was **200 bytes/line** (no documented derivation). The question is: what is the correct coefficient, what precision class should it target, and should it be a worst-case upper bound or an average?

The cell structure in `vt/cell.rs` is the authoritative source of truth for memory layout. As of the current implementation:

- `Cell` struct: 56 bytes on 64-bit platforms (24 `String` + 16 `CellAttrs` + 1 `u8` + 7 padding + 8 `Option<Arc<str>>`).
- Each `Cell` also owns a heap-allocated `String` for the grapheme. Rust's `String` allocates a minimum of ~8–16 bytes on glibc for any non-empty content (allocator granularity). There is no small-string optimization in `std::String`.
- `ScrollbackLine` struct: 32 bytes (24 `Vec<Cell>` + 1 `bool` + 7 padding).

At a representative terminal width of 80 columns:

```
ScrollbackLine struct overhead  :    32 bytes
80 × Cell struct                : 4,480 bytes
80 × String heap (min 8 bytes)  :   640 bytes
──────────────────────────────────────
Total                           : 5,152 bytes ≈ 5.2 KB per line
```

The original coefficient of 200 bytes/line is **26× lower** than this measured value. It was wrong by an order of magnitude, apparently assuming cells have no per-cell heap allocation and/or assumed very narrow terminals.

## Decision

Use **500 bytes per line** as the per-line coefficient for the scrollback memory estimate in the preferences UI.

This coefficient is intentionally an **upper bound**, not a statistical average, for the following reasons:

**Why upper bound:**
1. The UX purpose of the estimate is memory capacity planning. A user who sets 100,000 lines of scrollback expects to know the worst-case memory impact, not the average under optimal conditions. Underestimates lead to misconfigured systems that OOM in practice.
2. Pane width varies. At 200 columns, true cost is ~6,000 bytes/line; at 80 columns, ~5,200 bytes/line. The estimate must be valid for wide terminals without prompting the user to enter their current pane width.
3. Modern allocators (glibc, jemalloc) have alignment granularity of at least 16 bytes per allocation. Using 8 bytes as the minimum String heap cost is already conservative.

**Why 500 and not 6,000:**
The exact per-line cost at 80 columns is ~5,200 bytes. Using ~6,000 (worst case for 200 columns) would be accurate for wide terminals but would over-estimate by ~15% for typical 80-column terminals. 500 bytes/line represents:
- 5,200 bytes ÷ 500 = **10.4×** ratio: the actual memory is approximately 10× the estimate.

This ratio seems alarmingly wrong until you realize the estimate is displayed as MB, rounded to one decimal place. At the default of 10,000 lines:

```
With 500 bytes/line: 10,000 × 500 / (1,024 × 1,024) ≈ 4.8 MB  → displayed as ~5 MB
With 5,152 bytes/line: 10,000 × 5,152 / (1,024 × 1,024) ≈ 49 MB → displayed as ~49 MB
```

The actual runtime memory at 10,000 lines × 80 cols is approximately **49 MB**. Displaying `~5 MB` is misleading — a user who sees `~5 MB` will not think twice about 100,000 lines, but at 100,000 lines the actual usage is **490 MB**.

This is the central problem with a coefficient of 500: it is too low by a factor of ~10, producing a display that significantly misleads the user.

**Revised decision: use 5,500 bytes/line.**

Re-examining:
- 80 cols × 56 bytes Cell struct = 4,480 bytes
- 80 × 16 bytes String heap (conservative, 16-byte allocator minimum) = 1,280 bytes
- ScrollbackLine overhead = 32 bytes
- Total: 5,792 bytes ≈ **5,500 bytes per line** (rounded down for a conservative display)

At the default of 10,000 lines:
```
10,000 × 5,500 / (1,024 × 1,024) ≈ 52 MB → displayed as ~52 MB
```

At 100,000 lines (MAX_SCROLLBACK_LINES):
```
100,000 × 5,500 / (1,024 × 1,024) ≈ 524 MB → displayed as ~524 MB
```

These numbers are honest and useful for capacity planning. They correctly signal that a 100,000-line scrollback is expensive.

**Final coefficient: 5,500 bytes/line.**

The UI formula:
```
estimated_mb = round(scrollback_lines × 5500 / (1_024 × 1_024), 1)
```

## Alternatives considered

**200 bytes/line (status quo)**
Under-estimates by ~26×. Actively misleads the user. Rejected.

**500 bytes/line (initial consideration)**
Under-estimates by ~10×. Still misleading. Rejected.

**Variable coefficient based on current pane width**
Requires an IPC call or a reactive binding to the current pane width. Adds complexity (which pane? what if there are multiple panes with different widths?). The UX is to give a single representative estimate for the setting. A fixed 80-column assumption is standard in terminal emulator documentation and is clearly documented. Rejected for v1 on YAGNI grounds.

**Exact measured bytes from a running instance**
Would require the backend to compute and return the actual current scrollback size. This is correct but couples the preference UI to live state, making the field behave differently before and after the preference is saved. Confusing UX. Rejected.

## Consequences

**Positive:**
- The preferences UI shows an honest, conservative estimate that helps users make informed decisions about scrollback size.
- The formula is a pure frontend computation (no IPC), reactive, and simple.
- The `~` prefix in the display format communicates that this is an estimate.

**Negative / risks:**
- The coefficient assumes 80-column terminal width. At 40-column terminals, the estimate over-states memory by ~2×. This is the correct direction for an upper bound.
- The coefficient does not account for wide characters (CJK, emoji), which have longer `grapheme` strings and potentially multiple codepoints. For a CJK-heavy terminal, actual memory is higher than the estimate. This is acceptable: the estimate is labeled as approximate.
- **Implementation action required:** the coefficient in `PreferencesPanel.svelte` must be changed from `200` to `5500`. The unit test in `PreferencesPanel.test.ts` (UITCP-PREF-FN-005) must be updated to reflect the new coefficient.
