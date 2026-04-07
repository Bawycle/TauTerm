<!-- SPDX-License-Identifier: MPL-2.0 -->
<!--
  TerminalPaneViewport — character cell grid, cursor overlay, and input event bindings.

  Renders the terminal screen as DOM rows of spans. Binds keyboard, mouse, wheel,
  focus, and blur events to the composable handlers.

  Security: cell content uses Svelte text interpolation — no {@html} (TUITC-SEC-010).

  Props received from TerminalPane:
    tp         — composable instance (useTerminalPane return value)
    active     — whether this pane is focused
    lineHeight — optional line-height override
    onkeydown  — keyboard handler (defined in TerminalPane to access tp.decckm etc.)
-->
<script lang="ts">
  import type { useTerminalPane } from '$lib/composables/useTerminalPane.svelte';
  import * as m from '$lib/paraglide/messages';

  interface Props {
    tp: ReturnType<typeof useTerminalPane>;
    active: boolean;
    lineHeight?: number;
    onkeydown: (event: KeyboardEvent) => void;
  }

  let { tp, active, lineHeight, onkeydown }: Props = $props();
</script>

<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
<div
  bind:this={tp.viewportEl}
  class="terminal-pane__viewport terminal-grid"
  data-screen-generation={tp.screenGeneration}
  style={lineHeight != null ? `--line-height-terminal: ${lineHeight}` : undefined}
  tabindex={active ? 0 : -1}
  role="textbox"
  aria-multiline="true"
  aria-label={m.terminal_output_aria_label()}
  aria-readonly="false"
  {onkeydown}
  onmousedown={tp.handleMousedown}
  onmousemove={tp.handleMousemove}
  onmouseup={tp.handleMouseup}
  onwheel={tp.handleWheel}
  onfocus={tp.handleFocus}
  onblur={tp.handleBlur}
>
  <!-- Cell grid: rows × cells — SECURITY: text via interpolation, never {@html} -->
  {#each tp.gridRows as row, rowIdx}
    <div class="terminal-pane__row">
      {#each row as cell, colIdx}
        {#if cell.width !== 0}
          {@const selected = tp.isSelected(rowIdx, colIdx)}
          <span
            class="terminal-pane__cell"
            class:terminal-pane__cell--wide={cell.width === 2}
            class:terminal-pane__cell--hyperlink={cell.hyperlink != null}
            class:terminal-pane__cell--blink={cell.blink}
            class:terminal-pane__cell--strikethrough={cell.strikethrough}
            class:terminal-pane__cell--selected={selected && active && !tp.selectionFlashing}
            class:terminal-pane__cell--selected-flash={selected && active && tp.selectionFlashing}
            class:terminal-pane__cell--selected-inactive={selected && !active}
            class:terminal-pane__cell--search-active={tp.activeSearchMatchSet.has(
              rowIdx * tp.cols + colIdx,
            )}
            class:terminal-pane__cell--search-match={!tp.activeSearchMatchSet.has(
              rowIdx * tp.cols + colIdx,
            ) && tp.searchMatchSet.has(rowIdx * tp.cols + colIdx)}
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
