<!-- SPDX-License-Identifier: MPL-2.0 -->

# TauTerm — Screen Buffer Data Model, Scrollback Structures, and Resize Constraints

> Part of the [Architecture](README.md).
> See also: [ADR-0011](../adr/ADR-0011-scrollback-rust-ring-buffer.md), [ADR-0017](../adr/ADR-0017-scrollback-memory-estimation.md).

---

## 14. Screen Buffer Data Model

This section specifies the in-memory layout of the terminal cell grid and scrollback ring, documents the memory estimation formula used by the preferences UI (FS-SB-002), defines the soft/hard wrap representation and its consequences for search and selection, and specifies the minimum terminal size constraint and where it is enforced in the resize pipeline.

### 14.1 Cell Structure Layout

The authoritative Rust definition is in `vt/cell.rs`. The table below documents the memory contribution of each field at compile time and at runtime.

#### `Cell` (per terminal cell)

| Field | Type | Struct size (bytes) | Heap allocation |
|---|---|---|---|
| `grapheme` | `String` | 24 (ptr + len + cap) | 1–4 bytes per ASCII/UTF-8 codepoint; minimum allocator granularity: typically 8–16 bytes on Linux glibc |
| `attrs` | `CellAttrs` | 16 (see below) | none — `Copy` type, inline |
| `width` | `u8` | 1 | none |
| `hyperlink` | `Option<Arc<str>>` | 8 (discriminant + pointer) | shared — counted once per hyperlink run, not per cell |
| alignment padding | — | 7 | none |
| **Total struct** | | **56 bytes** | |

`Cell` is not `Copy`. Each cell owns a `String` allocation. This is the dominant memory cost.

#### `CellAttrs` (SGR attribute set, inline in `Cell`)

`CellAttrs` is `Copy + Clone + PartialEq + Default`. All fields are inline.

| Field | Type | Size |
|---|---|---|
| `fg` | `Option<Color>` | 4 bytes (discriminant + 3-byte payload for `Rgb`, packed with `Ansi`/`Ansi256`) |
| `bg` | `Option<Color>` | 4 bytes |
| `underline_color` | `Option<Color>` | 4 bytes |
| `bold`, `dim`, `italic`, `blink`, `inverse`, `hidden`, `strikethrough` | 7 × `bool` | 7 bytes |
| `underline` | `u8` | 1 byte |
| alignment padding | — | variable |
| **Total (repr Rust)** | | **16 bytes** |

#### `Color` (inline in `CellAttrs`)

`Color` is an enum with three variants: `Ansi { index: u8 }`, `Ansi256 { index: u8 }`, `Rgb { r, g, b: u8 }`. The maximum payload is 3 bytes (`Rgb`). With a 1-byte discriminant, the enum occupies 4 bytes.

#### `ScrollbackLine` (per scrollback entry)

| Field | Type | Struct size (bytes) | Heap allocation |
|---|---|---|---|
| `cells` | `Vec<Cell>` | 24 (ptr + len + cap) | `cols × sizeof(Cell)` on heap + individual `String` allocations per cell |
| `soft_wrapped` | `bool` | 1 | none |
| alignment padding | — | 7 | none |
| **Total struct** | | **32 bytes** | |

---

### 14.2 Memory Estimation Formula

#### Context

FS-SB-002 requires the preferences UI to display an estimated memory consumption for the configured scrollback limit. UXD specifies the format: `~{N} MB per pane`. This section defines the formula used by `PreferencesPanel.svelte` (`scrollbackEstimateMb`) and documents its basis and precision class.

#### Derivation

For a terminal at a representative width of **80 columns**, the per-line memory cost is:

```
struct overhead (ScrollbackLine)  :    32 bytes
struct overhead (Vec<Cell>)       :    24 bytes (already counted in ScrollbackLine.cells)
80 × Cell struct                  : 4,480 bytes (80 × 56)
80 × String heap allocations      :   640 bytes (80 × ~8 bytes minimum on glibc)
──────────────────────────────────────────────
Total per line at 80 cols         : ~5,152 bytes
```

**Rounding and upper-bound choice:** The formula must be an **upper bound**, not an average, for three reasons:

1. The UX intent is to inform the user of worst-case memory usage for capacity planning. Underestimating leads users to configure a scrollback limit that exhausts memory in practice.
2. Pane widths vary. A pane at 200 columns costs 2.5× more than a pane at 80 columns. The estimate must not assume a narrow terminal.
3. String allocations on modern allocators (jemalloc, glibc) have alignment granularity of at least 16 bytes; using 8 bytes is already conservative.

**Decision:** Use **5 500 bytes per line** as the per-line coefficient in the UI estimate. This value is derived from the measured `Cell` struct layout (see §14.1 and ADR-0017): 56 bytes struct + ~16 bytes heap per `String` grapheme allocation × 80 columns + 32 bytes `ScrollbackLine` overhead ≈ 5 800 bytes/line worst case, conservatively rounded to 5 500.

This supersedes the earlier 500 bytes/line figure, which underestimated by a factor of ~11 by ignoring heap allocations entirely.

At the stated `MAX_SCROLLBACK_LINES` ceiling of 100,000:
- 100,000 × 5 500 bytes ≈ **524 MB** (estimated upper bound per pane)

At the default of 10,000 lines:
- 10,000 × 5 500 bytes ≈ **52 MB** (estimated upper bound per pane)

#### Formula

```
estimated_mb = round(scrollback_lines × 5_500 / (1_024 × 1_024), 1)
```

This is a pure frontend computation in `PreferencesPanel.svelte` with no IPC call required. It is reactive: it updates in real time as the user adjusts the input.

#### Precision label

The UXD format `~{N} MB per pane` deliberately includes the tilde prefix `~` to communicate that this is an estimate, not a guarantee. The estimate is intentionally pessimistic; the actual runtime usage depends on pane width, grapheme cluster length (multi-codepoint sequences are longer than ASCII), and allocator behavior.

#### Migration note

`PreferencesPanel.svelte` was updated in Wave 1 from `* 200` (initial placeholder, 200 bytes/line) to `* 5500` (correct coefficient per ADR-0017). The associated `PreferencesPanel.test.ts` was updated accordingly. No further migration is needed.

---

### 14.3 Soft Wrap and Hard Newline Representation

#### Storage

The `soft_wrapped: bool` flag is stored on `ScrollbackLine`, the struct that wraps each line as it enters the scrollback ring.

- `soft_wrapped = true`: the line ended because the terminal width was exhausted — the cursor reached the last column after a printable character and DECAWM (mode ?7) was active. The next character caused an implicit LF+CR, scrolling this line into the scrollback with the soft-wrap flag set.
- `soft_wrapped = false`: the line ended because a hard newline (`\n`, LF, VT, or FF) was received, or the line was evicted by a scroll command not triggered by auto-wrap.

**Where the flag is set:** `ScreenBuffer::scroll_up()` receives a `soft_wrapped: bool` parameter. The VT processor passes `soft_wrapped = true` when the scroll is triggered by the delayed-wrap mechanism in `write_char()`. It passes `soft_wrapped = false` for LF-triggered scrolls and for explicit scroll operations (CSI S, CSI T). Within a single `scroll_up(count > 1)` call, only the first evicted line may be soft-wrapped; subsequent lines in the same call are always hard-newline lines.

**Where the flag is NOT stored:** the flag is a property of the `ScrollbackLine`, not of any individual cell. There is no per-cell wrap marker. The visible screen buffer (`ScreenBuffer.cells`) does not carry wrap state: lines on the visible screen have not yet been committed to the scrollback, and their wrap state is tracked implicitly by the `VtProcessor.wrap_pending` flag on the current row.

#### Impact on search (FS-SEARCH-002)

`search.rs` groups consecutive scrollback lines into **logical lines** before matching:

- A group starts with any line.
- A group continues as long as each line has `soft_wrapped = true`.
- A group ends at the first line with `soft_wrapped = false`, or at the last line in the scrollback.

The group is flattened into a single string for matching. A word split across a soft-wrap boundary is therefore found as a single match (the search does not see the boundary). Match positions (`scrollback_row`, `col_start`, `col_end`) are reported relative to the individual scrollback row, not the logical line — the caller can reconstruct the correct visual position.

This behavior is intentional and correct: soft wraps are an artifact of terminal width, not of the program's output. Search must be transparent to them.

#### Impact on text selection and copy

When the user selects and copies text that spans soft-wrapped lines, the copy operation must NOT insert a newline character at soft-wrap boundaries — the output must reconstruct the original program output without artificial line breaks.

Concretely: if the user selects from column 0 of scrollback row N (soft-wrapped) through column 3 of scrollback row N+1, the copied string must join those two rows without a `\n` in between. A `\n` is inserted only at hard-newline boundaries.

**Implementation responsibility:** the frontend selection-to-clipboard pipeline (in `selection.svelte.ts` or equivalent) must consult the `soft_wrapped` flag when assembling the text to copy. This requires the `get_scrollback_line` API to expose the `soft_wrapped` flag, or a dedicated copy-text IPC command that performs the join server-side. For v1, the frontend approach is acceptable; the `get_scrollback_line` return type must be extended to include the flag if it is not already present.

**Current status:** `VtProcessor::get_scrollback_line` currently returns `Option<Vec<Cell>>` (stripping the `soft_wrapped` flag). This is a known gap. See §14.3.1 below.

#### 14.3.1 Required API change: expose `soft_wrapped` in `get_scrollback_line`

The current public API on `VtProcessor`:

```rust
pub fn get_scrollback_line(&self, index: usize) -> Option<Vec<Cell>>
```

does not expose `soft_wrapped`. This is insufficient for correct selection-to-clipboard behavior (§14.3 above). The API must be changed to:

```rust
pub fn get_scrollback_line(&self, index: usize) -> Option<(Vec<Cell>, bool)>
// Returns: (cells, soft_wrapped)
```

or a dedicated `ScrollbackLineRef` type:

```rust
pub struct ScrollbackLineRef {
    pub cells: Vec<Cell>,
    pub soft_wrapped: bool,
}

pub fn get_scrollback_line(&self, index: usize) -> Option<ScrollbackLineRef>
```

The second form is preferred (named field, no tuple positional confusion). The `frontend-dev` and `rust-dev` must coordinate this change before implementing text selection.

#### Impact on serialization (session persistence)

Session persistence is out of scope for v1 (see §12.1). When it is implemented, the `ScrollbackLine` struct is already `Clone` and can be made `Serialize`/`Deserialize` straightforwardly. The `soft_wrapped` flag must be included in the serialized form: it is semantic data, not a rendering hint, and must be restored accurately to preserve search and selection correctness after a session is restored.

---

### 14.4 Minimum Terminal Size Constraint

#### Requirement

VT conventions and most interactive applications require a minimum terminal size to render correctly. A PTY of 1×1 or 0×0 is pathological and must be rejected. The minimum is:

- **Columns (cols):** 20
- **Rows (rows):** 5

These values are chosen to match the practical minimum for most interactive TUI programs (editors, pagers, shells with prompts) while being small enough to not interfere with any realistic layout. The values align with what iTerm2 and gnome-terminal enforce.

#### Enforcement layer: `SessionRegistry::resize_pane`

The minimum is enforced in the Rust backend at `SessionRegistry::resize_pane`, before the value reaches `PaneSession::resize` and `VtProcessor::resize`. This is the single point of enforcement for all resize paths:

- **Viewport-driven resize** (ResizeObserver in the frontend → `resize_pane` IPC command → `SessionRegistry::resize_pane`): enforced.
- **Programmatic resize** (any future code path that calls `SessionRegistry::resize_pane` directly): enforced because it goes through the same function.

```rust
// In SessionRegistry::resize_pane, before calling pane.resize():
const MIN_COLS: u16 = 20;
const MIN_ROWS: u16 = 5;
let cols = cols.max(MIN_COLS);
let rows = rows.max(MIN_ROWS);
```

The clamped values are silently applied: no error is returned. The frontend does not receive a rejection; it receives a SIGWINCH for the clamped size when the terminal is ready. This is correct behavior: the frontend cannot usefully react to a `col_too_small` error from a resize operation — the correct response is always to apply the minimum.

#### Why not enforce in the frontend?

Enforcing the minimum in the frontend alone would be insufficient: any other code path calling `resize_pane` (including test injection, future plugin commands) would bypass it. Frontend validation is a UX improvement (prevents sending a known-too-small resize) but cannot be the sole enforcement layer.

#### Why not enforce in `VtProcessor::resize`?

`VtProcessor::resize` is a low-level primitive that does not own session or PTY state. Enforcing there would hide the constraint from the layer that also owns `TIOCSWINSZ`. Enforcement at `SessionRegistry::resize_pane` keeps the invariant at the layer that owns the full resize operation (VtProcessor + PTY).

#### Why not enforce in `PaneSession::resize`?

`PaneSession::resize` is also acceptable as an enforcement point and is closer to the PTY call. However, `SessionRegistry::resize_pane` is the public API that command handlers call; enforcing there provides the widest coverage. `PaneSession::resize` does not add a second enforcement layer — a single point is correct.

#### Pixel dimensions

Pixel dimensions (`pixel_width`, `pixel_height`) are passed through to `TIOCSWINSZ` unchanged. The minimum constraint applies only to character dimensions (cols, rows). There is no minimum pixel size constraint: pixel dimensions are informational hints for applications that query them (e.g., sixel graphics size calculations) and zero is a valid value indicating "unknown".

#### Initial size

When a pane is created, the initial size is determined by the frontend viewport measurement and passed to `VtProcessor::new` and the PTY spawn call. The minimum is applied at spawn time via the same clamping in `SessionRegistry`. `VtProcessor::new` does not enforce a minimum internally; it is always called with an already-clamped size.

---

*This section is maintained by the TauTerm software architect. Any change to the cell struct layout, the memory estimate formula, the soft-wrap flag semantics, or the minimum terminal size requires updating this document.*
