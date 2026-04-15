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
  import { invoke } from '@tauri-apps/api/core';
  import { onSshWarning, onSshReconnected } from '$lib/ipc';
  import { keyEventToVtSequence } from '$lib/terminal/keyboard.js';
  import { useTerminalPane } from '$lib/composables/useTerminalPane.svelte';
  import ContextMenu from './ContextMenu.svelte';
  import TerminalPaneViewport from './TerminalPaneViewport.svelte';
  import TerminalPaneScrollbar from './TerminalPaneScrollbar.svelte';
  import TerminalPaneScrollToBottom from './TerminalPaneScrollToBottom.svelte';
  import TerminalPaneBanners from './TerminalPaneBanners.svelte';
  import TerminalPaneReconnectionSeparators from './TerminalPaneReconnectionSeparators.svelte';
  import TerminalPanePasteDialog from './TerminalPanePasteDialog.svelte';
  import PaneTitleBar from './PaneTitleBar.svelte';
  import * as m from '$lib/paraglide/messages';
  import { sshStates } from '$lib/state/ssh.svelte';
  import type { PaneId, TabId, SshLifecycleState, SearchMatch, BellType } from '$lib/ipc';

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
    /**
     * When true, hides the mouse cursor while the user is typing in the terminal.
     * Mirrors AppearancePrefs.hideCursorWhileTyping. Default: true.
     */
    hideCursorWhileTyping?: boolean;
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
    /** Called when the user renames this pane via the title bar. */
    onrenamepane?: (label: string | null) => void;
    /**
     * Whether to show the pane title bar (true when tab has ≥2 panes AND showPaneTitleBar preference is enabled).
     */
    showTitleBar?: boolean;
    /**
     * Resolved title for this pane's title bar. If undefined, the i18n fallback is used.
     */
    paneTitle?: string;
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
    hideCursorWhileTyping = true,
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
    onrenamepane,
    showTitleBar = false,
    paneTitle = undefined,
  }: Props = $props();

  // ── SSH lifecycle state — derived directly from module-level reactive Record ─
  // sshStates is a $state<Record> (plain object). Accessing sshStates[paneId]
  // creates a per-key reactive dependency, including for keys not yet present.
  // Using a plain object instead of $state<Map> avoids a Svelte 5 limitation:
  // Map.get(nonExistentKey) does not subscribe to future Map.set(key, ...) calls,
  // so $derived consumers mounted before the SSH connection starts would never
  // see the Connecting/Authenticating state transitions.
  const sshState = $derived(sshStates[paneId] ?? null);

  // ── SSH deprecated algorithm banner state (FS-SSH-014, UXD §7.21) ─────────
  // Local state: stays in the component as it is purely per-pane UI state.
  let deprecatedAlgorithm = $state<string | null>(null);

  // ── SSH reconnection separator state (FS-SSH-042, UXD §7.19) ──────────────
  let reconnectionSeparators = $state<number[]>([]);

  // Subscribe to SSH warning and reconnected events for this pane.
  let unlistenSshWarning: (() => void) | null = null;
  let unlistenSshReconnected: (() => void) | null = null;

  onMount(async () => {
    unlistenSshWarning = await onSshWarning((ev) => {
      if (ev.paneId !== paneId) return;
      deprecatedAlgorithm = ev.algorithm;
    });
    unlistenSshReconnected = await onSshReconnected((ev) => {
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

  // Notify parent of the active input element for focus management.
  // We pass inputEl (the hidden textarea) — it is the true focus receptor.
  // The parent calls .focus() on this element to restore focus after panel
  // switches, so it must point to the element that receives keyboard input.
  $effect(() => {
    if (active && tp.inputEl) {
      onviewportactive?.(tp.inputEl);
      return () => {
        onviewportactive?.(null);
      };
    }
  });

  // ── Mouse cursor hiding while typing (UI-2) ─────────────────────────────────
  // When the user presses a key that can produce terminal output, hide the mouse
  // cursor. It is restored as soon as the mouse moves over the viewport.
  let cursorHidden = $state(false);

  /** Restore the mouse cursor — bound to mousemove on the viewport. */
  function handleViewportMouseMove() {
    if (cursorHidden) cursorHidden = false;
  }

  function handleKeydown(event: KeyboardEvent) {
    // Application shortcuts (Ctrl+Shift+*) are handled at the TerminalView level.
    // AltGr on Linux/WebKitGTK emits ctrlKey=true; guard must not block AltGr+Shift.
    if (event.ctrlKey && event.shiftKey && !event.getModifierState('AltGraph')) return;
    if (event.ctrlKey && event.key === ',' && !event.getModifierState('AltGraph')) return; // Ctrl+, = preferences
    // Block in-progress composition (dead key pre-edit, IME candidate selection).
    if (event.isComposing) return;

    // Bare printable characters (no Ctrl/Alt/Meta modifier) are handled by
    // handleInput via the hidden textarea's input event — which correctly
    // captures GTK IM commits (dead keys, AltGr, IBus). Routing them through
    // keydown would cause double-send (keydown sends the raw key, input sends
    // the IM-resolved character, e.g. space → ~ after dead-tilde).
    if (event.key.length === 1 && !event.ctrlKey && !event.altKey && !event.metaKey) {
      return;
    }

    // Hide mouse cursor on output-producing keystrokes when the preference is on.
    if (hideCursorWhileTyping) {
      const k = event.key;
      const isModifierOnly =
        k === 'Control' ||
        k === 'Alt' ||
        k === 'Shift' ||
        k === 'Meta' ||
        k === 'CapsLock' ||
        k === 'NumLock' ||
        k === 'ScrollLock' ||
        k === 'Dead';
      if (!isModifierOnly) {
        cursorHidden = true;
      }
    }

    const sequence = keyEventToVtSequence(event, tp.decckm, tp.deckpam);
    if (sequence !== null) {
      event.preventDefault();
      tp.restartCursorBlink();
      tp.sendBytes(sequence);
    }
  }

  // Attach handleKeydown in capture phase on the hidden textarea.
  // Svelte's onkeydown={handler} uses bubble phase. In WebKitGTK, Tab's default
  // action (sequential focus navigation) is performed before bubble-phase handlers
  // run, so event.preventDefault() in bubble phase arrives too late — focus has
  // already moved. Capture phase fires before any default action, ensuring
  // preventDefault() suppresses the focus navigation reliably.
  $effect(() => {
    const el = tp.inputEl;
    if (!el) return;
    el.addEventListener('keydown', handleKeydown, { capture: true });
    return () => el.removeEventListener('keydown', handleKeydown, { capture: true });
  });

  /**
   * Handle GTK IM / IME composition commits arriving via the hidden textarea's
   * input event. This covers dead keys (e.g. dead_tilde + space → ~), AltGr
   * characters, and IBus/Fcitx input — all delivered by WebKitGTK as `input`
   * events on the editable textarea rather than as keydown events.
   *
   * Drains the textarea value immediately to prevent accumulation.
   * Control characters (cp < 0x20, cp === 0x7f) are excluded — they must
   * arrive through intentional key bindings handled by handleKeydown.
   *
   * Note: some IMs (Fcitx5) write directly to textarea.value rather than
   * setting event.data. Reading target.value first covers both cases.
   */
  // Guard against double-send when WebKitGTK/IBus emits an input event
  // (isComposing=false) immediately after compositionend. Without this flag,
  // handleInput's `target.value || inputEvent.data` fallback would re-send
  // the composed character via event.data even though the textarea was already
  // drained by handleCompositionEnd. Pattern used by xterm.js and CodeMirror.
  let compositionJustEnded = false;

  function handleInput(event: Event & { currentTarget: EventTarget & HTMLTextAreaElement }) {
    const inputEvent = event as unknown as InputEvent;
    // During composition (dead key pre-edit, IME candidate selection),
    // the IM is managing the textarea content. Do not drain or send —
    // the final composed character will arrive via compositionend.
    if (inputEvent.isComposing) return;
    // compositionend already sent the composed text and drained the textarea.
    // Skip the post-composition input event to prevent double-send.
    if (compositionJustEnded) {
      compositionJustEnded = false;
      const target = event.target as HTMLTextAreaElement;
      target.value = '';
      return;
    }
    const target = event.target as HTMLTextAreaElement;
    const text = target.value || inputEvent.data || '';
    target.value = '';
    if (!text) return;
    sendPrintableText(text);
  }

  /**
   * Handle compositionend — sends the final composed text (e.g. "î" from
   * dead ^ + i) and drains the textarea. Sets compositionJustEnded to
   * prevent the subsequent input event (if any) from double-sending.
   */
  function handleCompositionEnd(event: CompositionEvent) {
    compositionJustEnded = true;
    const target = event.target as HTMLTextAreaElement;
    // Prefer textarea value (some IMs like Fcitx5 write directly to it) over event.data.
    const text = target.value || event.data || '';
    target.value = '';
    if (!text) return;
    sendPrintableText(text);
  }

  /**
   * Send printable characters from an IM commit or direct input to the PTY.
   * Filters out control characters (must come through handleKeydown).
   */
  function sendPrintableText(text: string) {
    const encoder = new TextEncoder();
    for (const char of text) {
      const cp = char.codePointAt(0);
      if (cp === undefined || cp < 0x20 || cp === 0x7f) continue;
      // WebKitGTK inserts NBSP (U+00A0) instead of regular space (U+0020)
      // in textarea elements to prevent HTML whitespace collapsing.
      // Normalize to ASCII space so the shell receives a proper word separator.
      const byte = cp === 0xa0 ? ' ' : char;
      tp.sendBytes(encoder.encode(byte));
    }
    tp.restartCursorBlink();
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
  {#if showTitleBar}
    <PaneTitleBar
      title={paneTitle ?? m.pane_title_fallback()}
      isActive={active}
      onrename={onrenamepane}
    />
  {/if}

  <!-- Viewport wrapper: fills remaining flex space below PaneTitleBar.
       flex: 1; min-height: 0 prevents the viewport from overflowing the pane
       when a title bar is present. ContextMenu uses class="contents" so this
       div is the actual flex child that sizes the viewport correctly. -->
  <div class="terminal-pane__viewport-wrapper">
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
      <TerminalPaneViewport
        {tp}
        {active}
        {lineHeight}
        {cursorHidden}
        oninput={handleInput}
        oncompositionend={handleCompositionEnd}
        onmousemove={handleViewportMouseMove}
      />
      <TerminalPaneScrollbar {tp} />
      <TerminalPaneScrollToBottom
        scrollOffset={tp.scrollOffset}
        onclick={tp.handleScrollToBottom}
      />
    </ContextMenu>

    <!-- SSH reconnection separators — UI overlay injected at reconnect events
         (FS-SSH-042, UXD §7.19). Not interactive; aria-hidden (purely decorative).
         Must be inside viewport-wrapper so position:absolute inset:0 anchors to
         the viewport area only, not the full pane (which includes PaneTitleBar). -->
    <TerminalPaneReconnectionSeparators separators={reconnectionSeparators} />

    <!-- Banners: deprecated SSH algorithm, process terminated, SSH disconnected.
         Inside viewport-wrapper so absolute positioning excludes the title bar. -->
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
</div>

<!-- FS-CLIP-009: Multiline paste confirmation dialog (outside .terminal-pane for z-index) -->
<TerminalPanePasteDialog {tp} {confirmMultilinePaste} {ondisableConfirmMultilinePaste} />

<style>
  .terminal-pane {
    position: relative;
    display: flex;
    flex-direction: column;
    width: 100%;
    height: 100%;
    overflow: hidden;
    background-color: color-mix(
      in srgb,
      var(--term-bg) calc(var(--terminal-opacity, 1) * 100%),
      transparent
    );
    border: 2px solid var(--color-pane-border-inactive);
  }

  .terminal-pane--active {
    border: 2px solid var(--color-pane-border-active);
  }

  /* Viewport wrapper: fills the remaining flex space after the title bar.
     position: relative is required so that ContextMenu/scrollbar/scroll-to-bottom
     absolute overlays position correctly relative to this element. */
  .terminal-pane__viewport-wrapper {
    flex: 1;
    min-height: 0;
    overflow: hidden;
    position: relative;
  }

  /* Viewport styles remain here since TerminalPaneViewport renders inside .terminal-pane */
  /*
   * Hidden textarea — keyboard focus receptor and GTK IM input sink.
   * position:fixed removes it from layout flow. display:none or
   * visibility:hidden would prevent focus entirely, so we use opacity:0
   * + clip-path to make it invisible while keeping it focusable.
   */
  :global(.terminal-pane__input) {
    position: fixed;
    top: 0;
    left: 0;
    width: 1px;
    height: 1px;
    opacity: 0;
    pointer-events: none;
    resize: none;
    overflow: hidden;
    padding: 0;
    margin: 0;
    border: 0;
    outline: 0;
    font-size: 1px;
    line-height: 1;
    clip-path: inset(50%);
    white-space: nowrap;
  }

  :global(.terminal-pane__viewport--cursor-hidden) {
    cursor: none;
  }

  :global(.terminal-pane__viewport) {
    position: relative;
    width: 100%;
    height: 100%;
    overflow: hidden;
    font-family: var(--font-terminal);
    font-size: var(--font-size-terminal);
    line-height: var(--line-height-terminal);
    color: var(--term-fg);
    background-color: color-mix(
      in srgb,
      var(--term-bg) calc(var(--terminal-opacity, 1) * 100%),
      transparent
    );
    white-space: pre;
    /* Font ligatures: no visible effect on the current span-per-cell model (the shaping
       context is fragmented at every span boundary, preventing cross-glyph ligature
       formation). These declarations are future-proof: they will activate automatically
       when run-merging groups adjacent same-style cells into contiguous text nodes. */
    font-feature-settings:
      'liga' 1,
      'calt' 1;
    font-variant-ligatures: contextual;
    outline: none;
    cursor: text;
    contain: strict;
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
