<!-- SPDX-License-Identifier: MPL-2.0 -->
<!--
  SearchOverlay — incremental search bar over the terminal pane (UXD §7.4).

  Positioned top-right of the pane. Triggers search_pane IPC on each keystroke
  (debounced 150ms). Emits events for next/prev navigation and close.

  IPC: search_pane — implemented in Rust (vt/search.rs + commands/input_cmds.rs).

  Props:
    open       — whether the overlay is visible (bindable)
    matchCount — total number of matches (from last search result)
    currentMatch — 1-based index of active match (0 = no active match)
    onclose    — called when overlay should close (Escape or X button)
    onsearch   — called with SearchQuery when user types
    onnext     — called when user navigates to next match
    onprev     — called when user navigates to previous match

  Accessibility:
    - role="search" on container (UXD §7.4.1)
    - aria-label on input
    - aria-label on Prev/Next buttons
  Security: no {@html}, regex is opt-in (SEC-UI-003).
-->
<script lang="ts">
  import { ChevronUp, ChevronDown, X } from 'lucide-svelte';
  import * as m from '$lib/paraglide/messages';
  import type { SearchQuery } from '$lib/ipc/types';

  // ---------------------------------------------------------------------------
  // Props
  // ---------------------------------------------------------------------------

  interface Props {
    open?: boolean;
    matchCount?: number;
    currentMatch?: number;
    onclose?: () => void;
    onsearch?: (query: SearchQuery) => void;
    onnext?: () => void;
    onprev?: () => void;
  }

  const {
    open = $bindable(false),
    matchCount = 0,
    currentMatch = 0,
    onclose,
    onsearch,
    onnext,
    onprev,
  }: Props = $props();

  // ---------------------------------------------------------------------------
  // Local state
  // ---------------------------------------------------------------------------

  let searchText = $state('');
  let caseSensitive = $state(false);
  let regexMode = $state(false);
  let debounceTimer = $state<ReturnType<typeof setTimeout> | null>(null);

  // Auto-focus search input when overlay opens
  let inputEl = $state<HTMLInputElement | null>(null);
  $effect(() => {
    if (open && inputEl) {
      inputEl.focus();
    }
  });

  // ---------------------------------------------------------------------------
  // Match count display
  // ---------------------------------------------------------------------------

  const matchCountDisplay = $derived.by(() => {
    if (matchCount === 0) return m.search_no_results();
    return m.search_result_count({ current: currentMatch, total: matchCount });
  });

  // ---------------------------------------------------------------------------
  // Search handlers
  // ---------------------------------------------------------------------------

  function triggerSearch() {
    if (debounceTimer) clearTimeout(debounceTimer);
    debounceTimer = setTimeout(() => {
      onsearch?.({ text: searchText, caseSensitive, regex: regexMode });
    }, 150);
  }

  function handleInput(e: Event) {
    searchText = (e.target as HTMLInputElement).value;
    triggerSearch();
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'Escape') {
      e.preventDefault();
      handleClose();
    } else if (e.key === 'Enter' && e.shiftKey) {
      e.preventDefault();
      onprev?.();
    } else if (e.key === 'Enter') {
      e.preventDefault();
      onnext?.();
    }
  }

  function handleClose() {
    searchText = '';
    if (debounceTimer) clearTimeout(debounceTimer);
    onclose?.();
  }
</script>

{#if open}
  <div class="search-overlay" role="search" aria-label={m.action_search()}>
    <!-- Search input -->
    <input
      bind:this={inputEl}
      type="text"
      class="search-overlay__input"
      placeholder={m.search_placeholder()}
      value={searchText}
      oninput={handleInput}
      onkeydown={handleKeydown}
      aria-label={m.action_search()}
      autocomplete="off"
      spellcheck={false}
    />

    <!-- Match count -->
    <span class="search-overlay__count" aria-live="polite" aria-atomic="true">
      {matchCountDisplay}
    </span>

    <!-- Previous match -->
    <button
      class="search-overlay__nav-btn"
      onclick={() => onprev?.()}
      aria-label={m.search_prev_match()}
      title={m.search_prev_match()}
      disabled={matchCount === 0}
    >
      <ChevronUp size={14} aria-hidden="true" />
    </button>

    <!-- Next match -->
    <button
      class="search-overlay__nav-btn"
      onclick={() => onnext?.()}
      aria-label={m.search_next_match()}
      title={m.search_next_match()}
      disabled={matchCount === 0}
    >
      <ChevronDown size={14} aria-hidden="true" />
    </button>

    <!-- Close -->
    <button
      class="search-overlay__close-btn"
      onclick={handleClose}
      aria-label={m.search_close()}
      title={m.search_close()}
    >
      <X size={14} aria-hidden="true" />
    </button>
  </div>
{/if}

<style>
  .search-overlay {
    position: absolute;
    top: var(--space-2, 8px);
    right: var(--space-2, 8px);
    z-index: var(--z-search, 20);
    display: flex;
    align-items: center;
    gap: var(--space-1, 4px);
    padding: var(--space-2, 8px);
    background-color: var(--color-bg-raised);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-md, 4px);
    box-shadow: var(--shadow-raised);
    /* --size-search-overlay-width = 360px (docs/uxd/02-tokens.md §3.7) */
    width: min(var(--size-search-overlay-width, 360px), calc(100% - 2 * var(--space-4, 16px)));
    font-family: var(--font-ui);
  }

  .search-overlay__input {
    flex: 1;
    min-width: 0;
    /*
     * 36px is intentionally below --size-target-min (44px). This is a text input,
     * not a button — WCAG 2.5.5 applies to interactive controls that trigger an
     * action, not to form fields where the click target is the field itself.
     * The buttons in this overlay do meet the 44px minimum via --size-target-min.
     */
    height: 36px;
    padding: 0 var(--space-2, 8px);
    font-size: var(--font-size-ui-base, 13px);
    color: var(--color-text-primary);
    background-color: var(--term-bg);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-sm, 2px);
    outline: none;
  }

  .search-overlay__input::placeholder {
    color: var(--color-text-tertiary);
  }

  .search-overlay__input:focus-visible {
    outline: 2px solid var(--color-focus-ring);
    outline-offset: -1px;
  }

  .search-overlay__count {
    font-size: var(--font-size-ui-sm, 12px);
    color: var(--color-text-secondary);
    /*
     * 64px is a layout-specific minimum to prevent the counter from collapsing
     * to zero width on short strings like "1 / 9". No design token covers this
     * value because it is driven by content width, not the spacing scale.
     */
    min-width: 64px;
    text-align: center;
    white-space: nowrap;
    flex-shrink: 0;
  }

  .search-overlay__nav-btn,
  .search-overlay__close-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    /* --size-target-min = 44px — WCAG 2.5.5 minimum interactive target (docs/uxd/02-tokens.md §3.7) */
    width: var(--size-target-min, 44px);
    height: var(--size-target-min, 44px);
    color: var(--color-icon-default);
    background: transparent;
    border: none;
    border-radius: var(--radius-sm, 2px);
    cursor: pointer;
    outline: none;
    flex-shrink: 0;
  }

  .search-overlay__nav-btn:hover,
  .search-overlay__close-btn:hover {
    background-color: var(--color-hover-bg);
    color: var(--color-icon-active);
  }

  .search-overlay__nav-btn:focus-visible,
  .search-overlay__close-btn:focus-visible {
    outline: 2px solid var(--color-focus-ring);
    outline-offset: 2px;
  }

  .search-overlay__nav-btn:disabled {
    opacity: 0.4;
    cursor: not-allowed;
  }

  .search-overlay__nav-btn:disabled:hover {
    background: transparent;
    color: var(--color-icon-default);
  }
</style>
