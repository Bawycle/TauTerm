<!-- SPDX-License-Identifier: MPL-2.0 -->

# ADR-0008 — Terminal rendering strategy: DOM-based with row virtualization

**Date:** 2026-04-04
**Status:** Accepted

## Context

The terminal pane must render a grid of cells, each potentially having:
- A Unicode character (including wide/CJK, combining, emoji)
- Foreground and background colors (ANSI 16, 256-color, truecolor)
- Text attributes (bold, italic, underline, blink, inverse, strikethrough)
- Cursor overlay (block, underline, bar; blinking; focused/unfocused)
- Selection highlight (with X11 PRIMARY selection on Linux — FS-CLIP-004)
- Search match highlight
- Hyperlink underline

The screen can be up to several hundred columns and several dozen rows. At high terminal output rates, the backend emits `screen-update` events containing cell diffs. The renderer must apply diffs and repaint efficiently.

The target WebView on Linux is **WebKitGTK**, not Chromium. This is a binding constraint on any approach that assumes Chromium-level Canvas 2D or WebGL performance.

Two fundamentally different rendering strategies were evaluated:

**Option A: HTML/CSS DOM-based rendering**
Visible terminal rows are rendered as DOM elements. Each run of cells with identical attributes is collapsed into a `<span>`. CSS handles colors and text attributes. Row virtualization limits the live DOM to the visible viewport only. The browser's layout engine handles font metrics, glyph rendering, wide character width, and ligatures.

**Option B: Canvas-based rendering**
All terminal content is drawn onto an HTML `<canvas>` using the Canvas 2D API (or WebGL). Font measurement, glyph placement, cell coloring, and text selection are handled entirely in TypeScript.

## Decision

Use **Option A — DOM-based rendering with row virtualization and attribute-run merging**.

Each visible row is a DOM element. Adjacent cells with identical attributes are merged into a single `<span>` (attribute-run merging), reducing node count substantially. Only the rows currently visible in the pane viewport are in the live DOM (row virtualization). Updates from `screen-update` events are applied as targeted DOM mutations, not full redraws. `requestAnimationFrame` batching ensures at most one repaint per frame.

The `<TerminalPane />` Svelte component is the implementation boundary. It owns the row virtualization logic, the dirty-row tracking, and the cursor overlay. All other components (tab bar, status bar, preferences panel) are unaffected by this choice.

## Alternatives considered

### Canvas 2D

**Argument for:** Predictable rendering performance at high output rates; no DOM layout cost.

**Why rejected:**

1. **PRIMARY selection (X11/Wayland) — blocking constraint (FS-CLIP-004).** FS-CLIP-004 requires that selecting text in the terminal copies to the X11 PRIMARY selection (middle-click paste). DOM: this is native behavior via WebKitGTK at zero implementation cost. Canvas: PRIMARY selection must be driven by an `invoke('copy_to_clipboard', ...)` call on every `mouseup` after a drag — this requires a round-trip through the IPC boundary on each selection event. It cannot use the native PRIMARY selection mechanism because the selection is not in the DOM. This is architecturally incorrect and introduces latency on a latency-sensitive interaction.

2. **WebGL unavailable on v1 ARM32 and RISC-V targets.** Canvas 2D without WebGL acceleration does not offer a significant rendering advantage over DOM + `requestAnimationFrame` batching on WebKitGTK. Benchmarks cited in favor of canvas performance are measured on Chromium; WebKitGTK's `fillText()` is historically slower. The performance argument for canvas depends on a platform assumption that does not hold for all v1 targets.

3. **Ligature support is absent in Canvas 2D.** `CanvasRenderingContext2D.fillText()` bypasses the text shaping engine and does not render ligatures. Terminal fonts chosen by developers (Fira Code, Cascadia Code, Iosevka, JetBrains Mono) are selected specifically for their ligature support. DOM + CSS renders ligatures correctly via WebKitGTK's text shaping pipeline.

4. **Wide character / CJK width drift.** Canvas 2D must measure character widths via `measureText()` and maintain a JS-side unicode width table synchronized with the Rust backend's `unicode-width` crate. Any divergence between the two tables causes cursor positioning errors on CJK output. DOM delegates width computation to WebKitGTK's layout engine, which uses the same Unicode standard tables. No synchronization risk.

5. **Implementation cost: 8–12 weeks vs 3–5 weeks.** A correct canvas renderer for a terminal emulator is a standalone imperative rendering system (~800–1500 lines of TypeScript) entirely outside Svelte's component model. It requires custom hit-testing for selection, explicit cursor blink timers, canvas-to-font metric caching, and ARIA live region supplementation for accessibility. The DOM approach is a Svelte component that uses the browser's existing capabilities for all of the above.

6. **Design token integration.** DOM: CSS custom properties (`var(--term-bg)`, `var(--term-color-1)`, etc.) propagate instantly to all panes when a theme changes. FS-THEME-006 (live theme switching without restart) is free. Canvas: each token change requires explicit cache invalidation and a full repaint per pane. This is a significant implementation burden for a Must-level requirement.

### WebGL

Rejected on the same grounds as Canvas 2D, with the additional constraint that WebGL is not reliably available on ARM32 and RISC-V targets (Mesa + DRM + full OpenGL ES 2.0 stack required). WebGL would also require a custom glyph atlas, shader programs, and a dedicated rendering pipeline — estimated 12–16 weeks for a correct implementation.

### xterm.js (as a library)

Not applicable. xterm.js expects a raw PTY byte stream as input and performs VT parsing internally. TauTerm's architecture places the VT parser in the Rust backend (ADR-0003, `vte` crate) and sends pre-parsed cell diffs to the frontend via IPC (`screen-update` events). Integrating xterm.js would require bypassing the Rust VT parser, violating ADR-0003, and sending raw bytes across the IPC boundary — which is both architecturally incorrect and a performance regression. xterm.js is incompatible with TauTerm's IPC contract.

## Consequences

**Positive:**
- X11 PRIMARY selection (FS-CLIP-004) and Wayland `wp_primary_selection_v1` (FS-CLIP-007) work natively via WebKitGTK's text selection model. No IPC round-trip per selection event.
- Ligatures render correctly on all developer fonts via WebKitGTK's text shaping engine.
- Wide characters (CJK, emoji) and combining characters are handled by WebKitGTK's layout engine, consistent with the Rust backend's `unicode-width` usage.
- Design token CSS custom properties propagate to all pane renderers instantly on theme change (FS-THEME-006).
- The `<TerminalPane />` Svelte component integrates with the existing component architecture. State is component-local Svelte 5 runes.
- Implementation estimate: 3–5 weeks for a correct, virtualized DOM renderer.
- IME composition (FS-KBD-011) is handled natively by the browser for a focused DOM element.

**Negative / risks:**
- At very high terminal output rates, DOM mutations may cause frame drops. Mitigation: row virtualization + attribute-run merging + `requestAnimationFrame` batching reduce the per-frame mutation budget to the visible viewport only. This is sufficient for expected usage patterns.
- Blink attribute (SGR 5) requires a JavaScript timer toggling a CSS class on blinking cells. Rate is throttled to the configured cursor blink rate. The number of simultaneously blinking cells in practice is very low (typically 0 or 1 — the cursor).
- Hyperlink ranges (FS-VT-070) must be mapped to `<a>` elements across potentially multiple `<span>` runs. This requires splitting attribute runs at hyperlink boundaries, which adds complexity to the run-merging algorithm.

## Revision clause

Revisit this decision if profiling reveals sustained frame drops exceeding 16ms during high-volume terminal output (e.g., `cat large_file.txt` in a standard pane configuration) on a mid-range Linux system with WebKitGTK. Migration path if triggered: Canvas 2D with a glyph cache → WebGL only if Canvas 2D is also insufficient. The IPC contract (`screen-update` cell diff model) is renderer-agnostic and does not need to change for either migration path.
