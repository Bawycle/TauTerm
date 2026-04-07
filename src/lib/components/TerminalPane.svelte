<!-- SPDX-License-Identifier: MPL-2.0 -->
<!--
  TerminalPane — an individual terminal pane bound to a PTY session.

  Orchestrates the composable (useTerminalPane) and delegates rendering to:
    - TerminalPaneViewport    — cell grid, cursor, keyboard/mouse bindings
    - TerminalPaneScrollbar   — interactive scrollbar overlay
    - TerminalPaneScrollToBottom — scroll-to-bottom button
    - TerminalPaneBanners     — SSH deprecated, ProcessTerminated, SSH disconnected
    - TerminalPaneReconnectionSeparators — reconnection separator overlays
    - TerminalPanePasteDialog — multiline paste confirmation dialog

  Reactive logic is extracted to TerminalPane.svelte.ts (composable, §11.2).
  This file contains only template markup, state ownership, and event binding.

  Props:
    paneId  — unique pane identifier (PaneId from IPC contract)
    active  — whether this pane currently has focus

  Security:
    - No {@html} — all cell content uses Svelte text interpolation (textContent path)
    - Input is encoded as byte array (Uint8Array → number[]), never raw string
    - Resize dimensions clamped to minimum 1 (TUITC-SEC-050)
-->
<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { listen } from '@tauri-apps/api/event';
  import { invoke } from '@tauri-apps/api/core';
  import { keyEventToVtSequence } from '$lib/terminal/keyboard.js';
  import { useTerminalPane } from '$lib/composables/useTerminalPane.svelte';
  import ContextMenu from './ContextMenu.svelte';
  import TerminalPaneViewport from './TerminalPaneViewport.svelte';
  import TerminalPaneScrollbar from './TerminalPaneScrollbar.svelte';
  import TerminalPaneScrollToBottom from './TerminalPaneScrollToBottom.svelte';
  import TerminalPaneBanners from './TerminalPaneBanners.svelte';
  import TerminalPaneReconnectionSeparators from './TerminalPaneReconnectionSeparators.svelte';
  import TerminalPanePasteDialog from './TerminalPanePasteDialog.svelte';
  import * as m from '$lib/paraglide/messages';
  import type {
    PaneId,
    TabId,
    SshLifecycleState,
    SshWarningEvent,
    SshReconnectedEvent,
    SearchMatch,
    BellType,
  } from '$lib/ipc/types';

  interface Props {
    paneId: PaneId;
    tabId: TabId;
    active: boolean;
    /**
     * 1-based pane number for differentiated aria-label in multi-pane layouts.
     * When undefined, a generic label is used (single-pane case).
     */
    paneNumber?: number;
    /** Set to true when the PTY process has exited (from session-state-changed event). */
    terminated?: boolean;
    exitCode?: number;
    signalName?: string;
    /** Whether there is more than one pane (controls Close Pane visibility). */
    canClosePane?: boolean;
    /** SSH lifecycle state for Disconnected reconnect UI. */
    sshState?: SshLifecycleState | null;
    /**
     * Characters treated as word delimiters for double-click word selection.
     * Mirrors the Rust backend default (TerminalPrefs.wordDelimiters).
     */
    wordDelimiters?: string;
    /**
     * Whether to show a confirmation dialog when pasting multi-line text
     * without bracketed paste active (FS-CLIP-009).
     */
    confirmMultilinePaste?: boolean;
    /**
     * Cursor blink interval in milliseconds (FS-VT-032).
     * Mirrors AppearancePrefs.cursorBlinkMs. Default: 533 (OFF=266ms, ratio 2:1).
     */
    cursorBlinkMs?: number;
    /**
     * Bell notification type (FS-VT-090/093).
     * Mirrors TerminalPrefs.bellType. Default: 'audio'.
     */
    bellType?: BellType;
    /**
     * Terminal font family (FS-THEME-006). Used by Canvas cell measurement (F8).
     * When defined, overrides the global font token on this pane.
     */
    fontFamily?: string;
    /**
     * Terminal font size in pixels (FS-THEME-007). Used by Canvas cell measurement (F8).
     */
    fontSize?: number;
    /**
     * Terminal line height multiplier (FS-THEME-010). Range: 1.0–2.0.
     * When defined, overrides the global `--line-height-terminal` token on this pane.
     */
    lineHeight?: number;
    /**
     * Search matches for the current query in this pane (FS-SEARCH-006).
     * Only populated when this pane is active and a search is running.
     */
    searchMatches?: SearchMatch[];
    /**
     * 1-based index of the currently active search match (FS-SEARCH-006).
     * 0 means no active match.
     */
    activeSearchMatchIndex?: number;
    onrestart?: () => void;
    onclosepane?: () => void;
    onsearch?: () => void;
    onsplitH?: () => void;
    onsplitV?: () => void;
    /** Called when the user checks "Don't ask again" in the paste confirmation dialog. */
    ondisableConfirmMultilinePaste?: () => void;
    /** Called whenever the terminal dimensions (cols × rows) change (DIV-UXD-008). */
    ondimensionschange?: (cols: number, rows: number) => void;
    /** Called when this pane becomes/ceases to be the active viewport for focus management. */
    onviewportactive?: (el: HTMLElement | null) => void;
  }

  const {
    paneId,
    tabId: _tabId,
    active,
    paneNumber,
    terminated = false,
    exitCode = 0,
    signalName,
    canClosePane = true,
    sshState = null,
    wordDelimiters = ' \t|"\'`&()*,;<=>[]{}~',
    confirmMultilinePaste = true,
    cursorBlinkMs = 533,
    bellType = 'audio',
    fontFamily,
    fontSize,
    lineHeight,
    searchMatches = [],
    activeSearchMatchIndex = 0,
    onrestart,
    onclosepane,
    onsearch,
    onsplitH,
    onsplitV,
    ondisableConfirmMultilinePaste,
    ondimensionschange,
    onviewportactive,
  }: Props = $props();

  // ── SSH deprecated algorithm banner state (FS-SSH-014, UXD §7.21) ─────────
  // Local state: stays in the component as it is purely per-pane UI state.
  let deprecatedAlgorithm = $state<string | null>(null);

  // ── SSH reconnection separator state (FS-SSH-042, UXD §7.19) ──────────────
  let reconnectionSeparators = $state<number[]>([]);

  // Subscribe to SSH warning and reconnected events for this pane.
  let unlistenSshWarning: (() => void) | null = null;
  let unlistenSshReconnected: (() => void) | null = null;

  onMount(async () => {
    unlistenSshWarning = await listen<SshWarningEvent>('ssh-warning', (event) => {
      const ev = event.payload;
      if (ev.paneId !== paneId) return;
      deprecatedAlgorithm = ev.algorithm;
    });
    unlistenSshReconnected = await listen<SshReconnectedEvent>('ssh-reconnected', (event) => {
      const ev = event.payload;
      if (ev.paneId !== paneId) return;
      reconnectionSeparators = [...reconnectionSeparators, ev.timestampMs];
    });
  });

  onDestroy(() => {
    unlistenSshWarning?.();
    unlistenSshReconnected?.();
  });

  /** Dismiss the deprecated algorithm banner for this pane session (UXD §7.21.3). */
  async function handleDismissAlgorithmBanner() {
    deprecatedAlgorithm = null;
    try {
      await invoke('dismiss_ssh_algorithm_warning', { paneId });
    } catch {
      // Non-fatal — UI already dismissed.
    }
  }

  // Props are passed as getter functions to preserve Svelte 5 reactivity
  // across the composable boundary (§11.2 pattern).
  const tp = useTerminalPane({
    paneId: () => paneId,
    active: () => active,
    wordDelimiters: () => wordDelimiters,
    confirmMultilinePaste: () => confirmMultilinePaste,
    cursorBlinkMs: () => cursorBlinkMs,
    bellType: () => bellType,
    searchMatches: () => searchMatches,
    activeSearchMatchIndex: () => activeSearchMatchIndex,
    ondimensionschange: () => ondimensionschange,
    ondisableConfirmMultilinePaste: () => ondisableConfirmMultilinePaste,
    fontFamily: () => fontFamily,
    fontSize: () => fontSize,
    lineHeight: () => lineHeight,
  });

  // Notify parent of the active viewport element for focus management.
  $effect(() => {
    if (active && tp.viewportEl) {
      onviewportactive?.(tp.viewportEl);
    }
    return () => {
      onviewportactive?.(null);
    };
  });

  function handleKeydown(event: KeyboardEvent) {
    // Application shortcuts (Ctrl+Shift+*) are handled at the TerminalView level
    if (event.ctrlKey && event.shiftKey) return;
    if (event.ctrlKey && event.key === ',') return; // Ctrl+, = preferences
    if (event.isComposing) return;

    const sequence = keyEventToVtSequence(event, tp.decckm, tp.deckpam);
    if (sequence !== null) {
      event.preventDefault();
      tp.sendBytes(sequence);
    }
  }
</script>

<div
  class="terminal-pane"
  class:terminal-pane--active={active}
  class:terminal-pane--bell-flash={tp.bellFlashing}
  class:terminal-pane--pulse-output={!active && tp.borderPulse === 'output'}
  class:terminal-pane--pulse-bell={!active && tp.borderPulse === 'bell'}
  class:terminal-pane--pulse-exit={!active && tp.borderPulse === 'exit'}
  data-pane-id={paneId}
  data-active={active ? 'true' : undefined}
  role="region"
  aria-label={paneNumber != null
    ? m.terminal_pane_n_aria_label({ n: paneNumber })
    : m.terminal_pane_aria_label()}
>
  <!-- ContextMenu wraps the viewport so right-click opens it -->
  <ContextMenu
    variant="terminal"
    hasSelection={tp.hasSelection}
    {canClosePane}
    oncopy={tp.handleContextMenuCopy}
    onpaste={tp.handleContextMenuPaste}
    {onsearch}
    {onsplitH}
    {onsplitV}
    {onclosepane}
  >
    <TerminalPaneViewport {tp} {active} {lineHeight} onkeydown={handleKeydown} />
    <TerminalPaneScrollbar {tp} />
    <TerminalPaneScrollToBottom scrollOffset={tp.scrollOffset} onclick={tp.handleScrollToBottom} />
  </ContextMenu>

  <!-- SSH reconnection separators — UI overlay injected at reconnect events
       (FS-SSH-042, UXD §7.19). Not interactive; aria-hidden (purely decorative). -->
  <TerminalPaneReconnectionSeparators separators={reconnectionSeparators} />

  <!-- Banners: deprecated SSH algorithm, process terminated, SSH disconnected -->
  <TerminalPaneBanners
    {deprecatedAlgorithm}
    {terminated}
    {exitCode}
    {signalName}
    {sshState}
    {canClosePane}
    onDismissAlgorithm={handleDismissAlgorithmBanner}
    {onrestart}
    {onclosepane}
    onReconnect={tp.handleReconnect}
  />
</div>

<!-- FS-CLIP-009: Multiline paste confirmation dialog (outside .terminal-pane for z-index) -->
<TerminalPanePasteDialog {tp} {confirmMultilinePaste} {ondisableConfirmMultilinePaste} />

<style>
  .terminal-pane {
    position: relative;
    width: 100%;
    height: 100%;
    overflow: hidden;
    background-color: var(--term-bg);
    border: 2px solid var(--color-pane-border-inactive);
  }

  .terminal-pane--active {
    border: 2px solid var(--color-pane-border-active);
  }

  /* Viewport styles remain here since TerminalPaneViewport renders inside .terminal-pane */
  :global(.terminal-pane__viewport) {
    position: relative;
    width: 100%;
    height: 100%;
    overflow: hidden;
    font-family: var(--font-terminal);
    font-size: var(--font-size-terminal);
    line-height: var(--line-height-terminal);
    color: var(--term-fg);
    background-color: var(--term-bg);
    white-space: pre;
    outline: none;
    cursor: text;
    user-select: none;
  }

  :global(.terminal-pane__row) {
    display: flex;
    flex-wrap: nowrap;
    height: 1lh;
    min-height: 1lh;
  }

  :global(.terminal-pane__cell) {
    display: inline-block;
    width: 1ch;
    min-width: 1ch;
    height: 1lh;
    overflow: hidden;
    white-space: pre;
    flex-shrink: 0;
  }

  :global(.terminal-pane__cell--wide) {
    width: 2ch;
    min-width: 2ch;
  }

  /* OSC 8 hyperlink cells (FS-VT-071): pointer cursor on hover to indicate Ctrl+Click affordance */
  :global(.terminal-pane__cell--hyperlink) {
    cursor: pointer;
  }

  /* F4 — SGR 5/6 text blink animation (step-end, 2:1 ON:OFF ratio, 799ms total).
   *
   * The animation total duration is --term-blink-on-duration + --term-blink-off-duration
   * (533ms + 266ms = 799ms). With step-end timing, each keyframe value is held until
   * the *end* of its interval: opacity:1 is held from 0%→66.67% (the ON phase, ≈533ms)
   * then opacity:0 from 66.67%→100% (the OFF phase, ≈266ms), giving the 2:1 ON:OFF ratio.
   *
   * Paused when the pane is not the active pane in a multi-pane layout
   * (.terminal-pane--active class absent). Disabled entirely for prefers-reduced-motion: reduce.
   */
  @keyframes term-blink {
    0% {
      opacity: 1;
    }
    66.67% {
      opacity: 0; /* ON ratio 533/(533+266) ≈ 66.67% */
    }
    100% {
      opacity: 1;
    }
  }

  :global(.terminal-pane__cell--blink) {
    animation: term-blink calc(var(--term-blink-on-duration) + var(--term-blink-off-duration))
      step-end infinite;
  }

  /* Pause blink when pane is not focused */
  .terminal-pane:not(.terminal-pane--active) :global(.terminal-pane__cell--blink) {
    animation-play-state: paused;
  }

  @media (prefers-reduced-motion: reduce) {
    :global(.terminal-pane__cell--blink) {
      animation: none;
    }
  }

  /* F9 — Strikethrough at exactly 50% cell height via ::after pseudo-element.
   *
   * text-decoration: line-through is NOT used because its vertical position is
   * browser-controlled and cannot be set to exactly 50%. Instead, an absolutely
   * positioned line is rendered at var(--term-strikethrough-position) (50%),
   * shifted up by half its own thickness via translateY(-50%).
   */
  :global(.terminal-pane__cell--strikethrough) {
    position: relative;
  }

  :global(.terminal-pane__cell--strikethrough::after) {
    content: '';
    position: absolute;
    top: var(--term-strikethrough-position); /* 50% */
    left: 0;
    right: 0;
    height: var(--term-strikethrough-thickness); /* 1px */
    background: currentColor;
    transform: translateY(-50%);
    pointer-events: none;
  }

  /* Search match highlighting (FS-SEARCH-006) */
  :global(.terminal-pane__cell--search-match) {
    background-color: var(--term-search-match-bg) !important;
    color: var(--term-search-match-fg) !important;
  }

  :global(.terminal-pane__cell--search-active) {
    background-color: var(--term-search-active-bg) !important;
    color: var(--term-search-active-fg) !important;
  }

  /*
   * Selection colors (TUITC-UX-060/061) — declared AFTER search-match so that
   * selection takes priority over search highlights when both apply to the same
   * cell (same specificity + !important → last declaration wins in the cascade).
   */
  :global(.terminal-pane__cell--selected) {
    background-color: var(--term-selection-bg) !important;
  }

  :global(.terminal-pane__cell--selected-inactive) {
    background-color: var(--term-selection-bg-inactive) !important;
  }

  /* Copy flash (UXD §7.12) — 80ms bright flash on selection to confirm auto-copy */
  :global(.terminal-pane__cell--selected-flash) {
    background-color: var(--term-selection-flash) !important;
  }

  /* Visual bell flash (FS-VT-090) — brief border pulse using --color-indicator-bell */
  .terminal-pane--bell-flash {
    border-color: var(--color-indicator-bell) !important;
  }

  /* Pane border activity pulses for inactive panes (UXD §7.2.1) */
  .terminal-pane--pulse-output {
    border-color: var(--color-indicator-output) !important;
  }

  .terminal-pane--pulse-bell {
    border-color: var(--color-indicator-bell) !important;
  }

  .terminal-pane--pulse-exit {
    border-color: var(--color-error) !important;
  }

  /* Reduced motion: disable smooth transitions for pulses (UXD §7.2.1) */
  @media (prefers-reduced-motion: reduce) {
    .terminal-pane--pulse-output,
    .terminal-pane--pulse-bell,
    .terminal-pane--pulse-exit {
      transition: none;
    }
  }

  /* Cursor (TUITC-UX-050 to 053) */
  :global(.terminal-pane__cursor) {
    position: absolute;
    pointer-events: none;
    z-index: var(--z-cursor, 10);
  }

  :global(.terminal-pane__cursor--block) {
    width: 1ch;
    height: 1lh;
    background-color: var(--term-cursor-bg);
    /*
     * F7 — Back-to-front rendering: bg fill → cursor fill → glyph in --term-cursor-fg.
     * mix-blend-mode: difference is removed; the glyph is rendered explicitly via
     * the ::after pseudo-element using content: attr(data-char).
     * This guarantees the foreground text color is always var(--term-cursor-fg),
     * independent of the cell's own fg color.
     */
  }

  :global(.terminal-pane__cursor--block::after) {
    content: attr(data-char);
    position: absolute;
    inset: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    color: var(--term-cursor-fg);
    font-family: inherit;
    font-size: inherit;
    line-height: inherit;
    white-space: pre;
    pointer-events: none;
  }

  :global(.terminal-pane__cursor--underline) {
    width: 1ch;
    height: var(--size-cursor-underline-height, 2px);
    /*
     * Item 1 fix (UXD §7.3.1): `attr(style top)` is invalid CSS — the attr()
     * function does not support reading arbitrary inline style properties.
     * Instead, we receive --cursor-top from the inline style attribute
     * (set in the template as `style="--cursor-top:{cursor.row}lh; top:var(--cursor-top); …"`)
     * and offset by one line-height to position the underline at the bottom of
     * the character cell.
     */
    top: calc(var(--cursor-top) + 1lh - var(--size-cursor-underline-height, 2px)) !important;
    background-color: var(--term-cursor-bg);
  }

  :global(.terminal-pane__cursor--bar) {
    width: var(--size-cursor-bar-width, 2px);
    height: 1lh;
    background-color: var(--term-cursor-bg);
  }

  /* Unfocused: hollow outline per FS-VT-034, UXD §7.3.1 */
  :global(.terminal-pane__cursor--unfocused) {
    background-color: transparent !important;
    border: var(--size-cursor-outline-width) solid var(--term-cursor-unfocused);
    mix-blend-mode: normal;
  }

  /* Scrollbar overlay (FS-SB-007, TUITC-UX-070 to 073) */
  :global(.terminal-pane__scrollbar) {
    position: absolute;
    top: 0;
    right: 0;
    width: var(--size-scrollbar-width, 8px);
    height: 100%;
    z-index: var(--z-scrollbar, 15);
    cursor: pointer;
    transition: opacity var(--duration-slow, 300ms);
  }

  :global(.terminal-pane__scrollbar--dragging) {
    cursor: grabbing;
  }

  :global(.terminal-pane__scrollbar-thumb) {
    position: absolute;
    right: 0;
    width: 100%;
    min-height: 32px;
    background-color: var(--color-scrollbar-thumb);
    border-radius: var(--radius-full, 9999px);
    transition: background-color var(--duration-fast, 80ms);
    cursor: grab;
  }

  :global(.terminal-pane__scrollbar--dragging .terminal-pane__scrollbar-thumb) {
    cursor: grabbing;
  }

  :global(.terminal-pane__scrollbar-thumb--hover) {
    background-color: var(--color-scrollbar-thumb-hover);
  }

  @media (prefers-reduced-motion: reduce) {
    :global(.terminal-pane__scrollbar) {
      transition: none;
    }
    :global(.terminal-pane__scrollbar-thumb) {
      transition: none;
    }
  }

  /* Reconnection separators overlay (FS-SSH-042) */
  :global(.terminal-pane__reconnection-separators) {
    position: absolute;
    inset: 0;
    pointer-events: none;
    overflow: hidden;
  }
</style>
