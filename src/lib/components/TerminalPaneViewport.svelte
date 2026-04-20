<!-- SPDX-License-Identifier: MPL-2.0 -->
<!--
  TerminalPaneViewport — character cell grid, cursor overlay, and input event bindings.

  Renders the terminal screen as DOM rows of spans.

  Input architecture (Linux/WebKitGTK):
  - The hidden <textarea> (terminal-pane__input) receives keyboard focus, keydown events,
    and GTK IM composition commits (dead keys, AltGr, IBus/Fcitx) via the `input` event.
    WebKit only delivers GTK IM commits to editable elements; a plain <div> silently drops
    them. The textarea is invisible (position:fixed, opacity:0, clip-path) but focusable.
  - The visible <div> hosts the cell grid and handles mouse events. It is tabindex=-1
    (never receives tab/keyboard focus) and delegates mousedown focus to the textarea.

  Security: cell content uses Svelte text interpolation — no {@html} (TUITC-SEC-010).

  Props received from TerminalPane:
    tp         — composable instance (useTerminalPane return value)
    active     — whether this pane is focused
    lineHeight — optional line-height override
    onkeydown  — keyboard handler (defined in TerminalPane to access tp.decckm etc.)
    oninput    — GTK IM / IME composition commit handler
-->
<script lang="ts">
  import type { useTerminalPane } from '$lib/composables/useTerminalPane.svelte';
  import * as m from '$lib/paraglide/messages';

  interface Props {
    tp: ReturnType<typeof useTerminalPane>;
    active: boolean;
    lineHeight?: number;
    /** When true, applies cursor:none to hide the mouse pointer while the user types. */
    cursorHidden?: boolean;
    /** Handles GTK IM / IME composition commits (dead keys, AltGr via IBus, etc.). */
    oninput: (event: Event & { currentTarget: EventTarget & HTMLTextAreaElement }) => void;
    /** Handles compositionend — delivers the final composed character (dead keys, IME). */
    oncompositionend: (event: CompositionEvent) => void;
    onmousemove?: (event: MouseEvent) => void;
  }

  let {
    tp,
    active,
    lineHeight,
    cursorHidden = false,
    oninput,
    oncompositionend,
    onmousemove,
  }: Props = $props();
</script>

<!--
  Hidden textarea — true keyboard focus receptor and GTK IM input sink.
  Receives keydown, input (GTK IM commits), focus, and blur events.
  Invisible via position:fixed + opacity:0 + clip-path, but focusable
  (display:none or visibility:hidden would prevent focus entirely).
  aria-hidden: true — screen readers use the visible div[role=textbox].

  tabindex={-1}: excluded from sequential focus navigation. Focus is
  always programmatic (tp.inputEl?.focus() from mousedown, onviewportactive,
  and onWindowFocus). With tabindex=-1 here AND on all other interactive
  elements (TabBarItems, buttons), there are zero elements in the tab order.
  WebKitGTK cannot consume Tab for focus navigation because there are no
  sequential-focus candidates. The capture-phase keydown handler in
  TerminalPane intercepts Tab and sends HT (0x09) to the PTY for shell
  completion.
-->
<textarea
  bind:this={tp.inputEl}
  class="terminal-pane__input"
  tabindex={-1}
  aria-hidden="true"
  autocomplete="off"
  autocapitalize="off"
  spellcheck={false}
  rows={1}
  {oninput}
  {oncompositionend}
  onfocus={tp.handleFocus}
  onblur={tp.handleBlur}
></textarea>

<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
<div
  bind:this={tp.viewportEl}
  class="terminal-pane__viewport terminal-grid"
  class:terminal-pane__viewport--cursor-hidden={cursorHidden}
  data-screen-generation={tp.screenGeneration}
  style={lineHeight != null ? `--line-height-terminal: ${lineHeight}` : undefined}
  tabindex={-1}
  role="textbox"
  aria-multiline="true"
  aria-label={m.terminal_output_aria_label()}
  aria-readonly="false"
  onmousedown={(e) => {
    // Redirect keyboard focus to the hidden textarea — the true input receptor.
    // Must happen on mousedown (before click) so focus arrives before any handler.
    tp.inputEl?.focus({ preventScroll: true });
    tp.handleMousedown(e);
  }}
  onmousemove={(e) => {
    tp.handleMousemove(e);
    onmousemove?.(e);
  }}
  onmouseup={tp.handleMouseup}
  onwheel={tp.handleWheel}
>
  <!-- Cell grid: rows × cells — SECURITY: text via interpolation, never {@html} -->
  {#each tp.gridRows as row, rowIdx (rowIdx)}
    <div class="terminal-pane__row">
      {#each row as cell, colIdx}
        {#if cell.width !== 0}
          {@const selected = tp.hasSelectionRange && tp.isSelected(rowIdx, colIdx)}
          <span
            class="terminal-pane__cell"
            class:terminal-pane__cell--wide={cell.width === 2}
            class:terminal-pane__cell--hyperlink={cell.hyperlink != null}
            class:terminal-pane__cell--blink={cell.blink}
            class:terminal-pane__cell--strikethrough={cell.strikethrough}
            class:terminal-pane__cell--selected={selected && active && !tp.selectionFlashing}
            class:terminal-pane__cell--selected-flash={selected && active && tp.selectionFlashing}
            class:terminal-pane__cell--selected-inactive={selected && !active}
            class:terminal-pane__cell--search-active={tp.hasSearchMatches &&
              tp.activeSearchMatchSet.has(rowIdx * tp.cols + colIdx)}
            class:terminal-pane__cell--search-match={tp.hasSearchMatches &&
              !tp.activeSearchMatchSet.has(rowIdx * tp.cols + colIdx) &&
              tp.searchMatchSet.has(rowIdx * tp.cols + colIdx)}
            style={cell.style}>{cell.content === '' ? '\u00a0' : cell.content}</span
          >
        {/if}
      {/each}
    </div>
  {/each}

  <!-- Cursor overlay (TUITC-FN-001 to 006, TUITC-UX-050 to 053) -->
  <!-- F7: data-char carries the glyph under the block cursor so the CSS
       pseudo-element can re-render it in var(--term-cursor-fg) without
       mix-blend-mode tricks. -->
  {#if tp.cursor.visible && (tp.cursorVisible || !tp.currentCursorBlinks)}
    <div
      class="terminal-pane__cursor"
      class:terminal-pane__cursor--block={tp.currentCursorShape === 'block'}
      class:terminal-pane__cursor--underline={tp.currentCursorShape === 'underline'}
      class:terminal-pane__cursor--bar={tp.currentCursorShape === 'bar'}
      class:terminal-pane__cursor--unfocused={!active}
      style="--cursor-top:{tp.cursor.row}lh; top:var(--cursor-top); left:{tp.cursor.col}ch"
      data-char={tp.gridRows[tp.cursor.row]?.[tp.cursor.col]?.content || ' '}
      aria-hidden="true"
    ></div>
  {/if}
</div>
