<!-- SPDX-License-Identifier: MPL-2.0 -->
<!--
  KeyboardShortcutRecorder — inline keyboard shortcut capture field (UXD §7.17).

  States:
    inactive  — shows current shortcut
    recording — captures next key combination ("Press keys...")
    captured  — shows new shortcut (confirmed on Enter, cancelled on Escape)
    conflict  — new shortcut conflicts with existing binding

  Props:
    value         — current shortcut string (e.g. "Ctrl+Shift+T")
    existingShortcuts — map of action → shortcut string for conflict detection
    actionId      — id of this action (excluded from conflict check)
    onchange      — called with new shortcut string when confirmed
    disabled      — disables the recorder

  Note: Super/Meta key combinations cannot be reliably captured in WebView context
  and are excluded from accepted input (UXD §7.17 constraint).

  Security: all values rendered as text interpolation, no {@html}.
-->
<script lang="ts">
  import * as m from '$lib/paraglide/messages';

  type RecorderState = 'inactive' | 'recording' | 'captured' | 'conflict';

  interface Props {
    value?: string;
    existingShortcuts?: Record<string, string>;
    actionId?: string;
    disabled?: boolean;
    onchange?: (shortcut: string) => void;
  }

  const {
    value = '',
    existingShortcuts = {},
    actionId = '',
    disabled = false,
    onchange,
  }: Props = $props();

  // eslint-disable-next-line prefer-const
  let recorderState = $state<RecorderState>('inactive');
  let pendingShortcut = $state('');
  let conflictingAction = $state('');

  function formatKeyEvent(e: KeyboardEvent): string {
    const parts: string[] = [];
    if (e.ctrlKey) parts.push('Ctrl');
    if (e.altKey) parts.push('Alt');
    if (e.shiftKey) parts.push('Shift');
    // Exclude Meta/Super — not reliably capturable in WebView (UXD §7.17)
    const key = e.key;
    // Filter out bare modifier keys
    if (!['Control', 'Alt', 'Shift', 'Meta', 'Super'].includes(key)) {
      parts.push(key.length === 1 ? key.toUpperCase() : key);
    }
    return parts.join('+');
  }

  function findConflict(shortcut: string): string | null {
    for (const [id, existing] of Object.entries(existingShortcuts)) {
      if (id !== actionId && existing === shortcut) return id;
    }
    return null;
  }

  function handleClick() {
    if (disabled) return;
    recorderState = 'recording';
    pendingShortcut = '';
    conflictingAction = '';
  }

  function handleKeydown(e: KeyboardEvent) {
    if (recorderState !== 'recording') return;

    // Escape always cancels
    if (e.key === 'Escape') {
      e.preventDefault();
      recorderState = 'inactive';
      return;
    }

    // Enter confirms if captured
    if (e.key === 'Enter') {
      e.preventDefault();
      if (recorderState === 'recording' && pendingShortcut) {
        confirmShortcut();
      }
      return;
    }

    e.preventDefault();
    const shortcut = formatKeyEvent(e);
    if (!shortcut || shortcut === '') return;

    pendingShortcut = shortcut;
    const conflict = findConflict(shortcut);
    if (conflict) {
      conflictingAction = conflict;
      recorderState = 'conflict';
    } else {
      conflictingAction = '';
      recorderState = 'captured';
    }
  }

  function handleConfirmKeydown(e: KeyboardEvent) {
    if (recorderState === 'captured' && e.key === 'Enter') {
      e.preventDefault();
      confirmShortcut();
    } else if (e.key === 'Escape') {
      e.preventDefault();
      cancelRecording();
    }
  }

  function confirmShortcut() {
    if (recorderState === 'captured' && pendingShortcut) {
      onchange?.(pendingShortcut);
      recorderState = 'inactive';
    }
  }

  function cancelRecording() {
    recorderState = 'inactive';
    pendingShortcut = '';
    conflictingAction = '';
  }

  const displayValue = $derived(
    recorderState === 'recording'
      ? ''
      : recorderState === 'captured' || recorderState === 'conflict'
        ? pendingShortcut
        : value
  );

  const placeholderText = $derived(
    recorderState === 'recording' ? m.preferences_keyboard_press_shortcut() : ''
  );
</script>

<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
<div class="shortcut-recorder">
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <div
    role="textbox"
    tabindex={disabled ? -1 : 0}
    aria-label="Keyboard shortcut"
    aria-readonly={recorderState !== 'recording'}
    class="shortcut-recorder__field"
    class:shortcut-recorder__field--recording={recorderState === 'recording'}
    class:shortcut-recorder__field--captured={recorderState === 'captured'}
    class:shortcut-recorder__field--conflict={recorderState === 'conflict'}
    class:shortcut-recorder__field--disabled={disabled}
    onclick={handleClick}
    onkeydown={recorderState === 'recording' ? handleKeydown : handleConfirmKeydown}
  >
    {#if recorderState === 'recording'}
      <span class="shortcut-recorder__placeholder">
        {m.preferences_keyboard_press_shortcut()}
      </span>
    {:else}
      <span class="shortcut-recorder__value">
        {displayValue}
      </span>
    {/if}
  </div>

  {#if recorderState === 'conflict' && conflictingAction}
    <p class="shortcut-recorder__conflict-message" role="alert">
      {m.preferences_keyboard_shortcut_conflict({ other: conflictingAction })}
    </p>
  {/if}
</div>

<style>
  .shortcut-recorder__field {
    width: 160px;
    min-height: 44px;
    padding: 0 12px;
    display: flex;
    align-items: center;
    font-family: var(--font-mono-ui);
    font-size: var(--font-size-ui-sm);
    background-color: var(--term-bg);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-sm);
    cursor: pointer;
    outline: none;
    user-select: none;
  }

  .shortcut-recorder__field:focus-visible {
    outline: 2px solid var(--color-focus-ring);
    outline-offset: 2px;
  }

  .shortcut-recorder__field--recording {
    border: 2px solid var(--color-accent);
    background-color: var(--color-accent-subtle);
  }

  .shortcut-recorder__field--captured {
    border: 2px solid var(--color-success);
  }

  .shortcut-recorder__field--conflict {
    border: 2px solid var(--color-error);
    background-color: var(--color-error-bg);
  }

  .shortcut-recorder__field--disabled {
    cursor: not-allowed;
    opacity: 0.5;
  }

  .shortcut-recorder__value {
    color: var(--color-text-primary);
  }

  .shortcut-recorder__placeholder {
    color: var(--color-accent-text);
    animation: pulse-opacity 1.4s ease-in-out infinite;
  }

  .shortcut-recorder__conflict-message {
    margin-top: 4px;
    font-size: var(--font-size-ui-sm);
    color: var(--color-error-text);
  }

  @keyframes pulse-opacity {
    0%, 100% { opacity: 1; }
    50% { opacity: 0.5; }
  }

  @media (prefers-reduced-motion: reduce) {
    .shortcut-recorder__placeholder {
      animation: none;
    }
  }
</style>
