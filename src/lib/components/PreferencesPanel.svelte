<!-- SPDX-License-Identifier: MPL-2.0 -->
<!--
  PreferencesPanel — modal preferences dialog (UXD §7.6, FS-PREF-001..006).

  Sections: Keyboard, Appearance, Terminal Behavior, Connections.
  Uses ui/Dialog as base for focus trap, Escape close, aria-modal.
  Ctrl+, shortcut is wired in TerminalView.

  Props:
    open        — whether the panel is open (bindable)
    preferences — current Preferences object from get_preferences IPC
    onclose     — called when the dialog should close
    onupdate    — called with PreferencesPatch when a preference changes

  Security: all values rendered as text interpolation, no {@html}.
  Note: Themes section and inline ConnectionManager are deferred references
  to their standalone components for reuse.
-->
<script lang="ts">
  import { Dialog } from 'bits-ui';
  import { X } from 'lucide-svelte';
  import TextInput from '$lib/ui/TextInput.svelte';
  import Dropdown from '$lib/ui/Dropdown.svelte';
  import Toggle from '$lib/ui/Toggle.svelte';
  import Button from '$lib/ui/Button.svelte';
  import KeyboardShortcutRecorder from './KeyboardShortcutRecorder.svelte';
  import * as m from '$lib/paraglide/messages';
  import type { Preferences, PreferencesPatch } from '$lib/ipc/types';

  // ---------------------------------------------------------------------------
  // Props
  // ---------------------------------------------------------------------------

  interface Props {
    open?: boolean;
    preferences?: Preferences;
    onclose?: () => void;
    onupdate?: (patch: PreferencesPatch) => void;
  }

  let {
    open = $bindable(false),
    preferences,
    onclose,
    onupdate,
  }: Props = $props();

  // ---------------------------------------------------------------------------
  // Section navigation
  // ---------------------------------------------------------------------------

  type Section = 'keyboard' | 'appearance' | 'terminal' | 'connections';
  let activeSection = $state<Section>('keyboard');

  const sections: { id: Section; label: () => string }[] = [
    { id: 'keyboard', label: () => m.preferences_section_keyboard() },
    { id: 'appearance', label: () => m.preferences_section_appearance() },
    { id: 'terminal', label: () => m.preferences_section_terminal() },
    { id: 'connections', label: () => m.preferences_section_connections() },
  ];

  // ---------------------------------------------------------------------------
  // Default application shortcuts (FS-KBD-003)
  // Simplified: stored locally, emitted via onupdate when changed.
  // A full implementation would persist these in Preferences.
  // ---------------------------------------------------------------------------

  const defaultShortcuts: Record<string, string> = {
    new_tab: 'Ctrl+Shift+T',
    close_tab: 'Ctrl+Shift+W',
    paste: 'Ctrl+Shift+V',
    search: 'Ctrl+Shift+F',
    preferences: 'Ctrl+,',
    next_tab: 'Ctrl+Tab',
    prev_tab: 'Ctrl+Shift+Tab',
    rename_tab: 'F2',
  };

  let shortcuts = $state<Record<string, string>>({ ...defaultShortcuts });

  function handleShortcutChange(actionId: string, newShortcut: string) {
    shortcuts[actionId] = newShortcut;
    // TODO: persist via update_preferences once preferences schema includes shortcuts
  }

  // ---------------------------------------------------------------------------
  // Appearance
  // ---------------------------------------------------------------------------

  function handleFontFamilyChange(val: string) {
    onupdate?.({ appearance: { fontFamily: val } });
  }

  function handleFontSizeChange(val: string) {
    const n = parseInt(val, 10);
    if (isNaN(n)) return;
    const clamped = Math.max(8, Math.min(32, n));
    onupdate?.({ appearance: { fontSize: clamped } });
  }

  function handleLanguageChange(val: string) {
    if (val === 'En' || val === 'Fr') {
      onupdate?.({ appearance: { language: val } });
    }
  }

  const languageOptions = [
    { value: 'En', label: m.locale_en() },
    { value: 'Fr', label: m.locale_fr() },
  ];

  // ---------------------------------------------------------------------------
  // Terminal behavior
  // ---------------------------------------------------------------------------

  const cursorShapeOptions = $derived([
    { value: 'block', label: m.preferences_cursor_shape_block() },
    { value: 'underline', label: m.preferences_cursor_shape_underline() },
    { value: 'bar', label: m.preferences_cursor_shape_bar() },
  ]);

  const bellTypeOptions = $derived([
    { value: 'visual', label: m.preferences_bell_type_visual() },
    { value: 'audible', label: m.preferences_bell_type_audible() },
    { value: 'disabled', label: m.preferences_bell_type_disabled() },
  ]);

  function handleScrollbackChange(val: string) {
    const n = parseInt(val, 10);
    if (!isNaN(n) && n > 0) {
      onupdate?.({ terminal: { scrollbackLines: n } });
    }
  }

  /** Rough memory estimate: ~200 bytes per line per pane. */
  const scrollbackEstimateMb = $derived(
    preferences?.terminal?.scrollbackLines
      ? Math.round((preferences.terminal.scrollbackLines * 200) / (1024 * 1024) * 10) / 10
      : 0
  );

  // ---------------------------------------------------------------------------
  // Keyboard shortcut action labels
  // ---------------------------------------------------------------------------

  const shortcutActions: { id: string; label: () => string }[] = [
    { id: 'new_tab', label: () => m.preferences_keyboard_action_label_new_tab() },
    { id: 'close_tab', label: () => m.preferences_keyboard_action_label_close_tab() },
    { id: 'paste', label: () => m.preferences_keyboard_action_label_paste() },
    { id: 'search', label: () => m.preferences_keyboard_action_label_search() },
    { id: 'preferences', label: () => m.preferences_keyboard_action_label_preferences() },
    { id: 'next_tab', label: () => m.preferences_keyboard_action_label_next_tab() },
    { id: 'prev_tab', label: () => m.preferences_keyboard_action_label_prev_tab() },
    { id: 'rename_tab', label: () => m.preferences_keyboard_action_label_rename_tab() },
  ];
</script>

<Dialog.Root bind:open onOpenChange={(o) => { if (!o) onclose?.(); }}>
  <Dialog.Portal>
    <Dialog.Overlay class="fixed inset-0 z-[49] bg-(--color-bg-overlay)/60" />

    <Dialog.Content
      class="preferences-panel fixed z-[50] top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2
             w-[640px] max-w-[90vw] max-h-[80vh]
             bg-(--color-bg-raised) border border-(--color-border) rounded-[4px]
             shadow-(--shadow-overlay) flex flex-col overflow-hidden"
      aria-modal="true"
    >
      <!-- Header -->
      <div class="flex items-center justify-between px-6 pt-6 pb-4 flex-shrink-0">
        <Dialog.Title class="text-[16px] font-semibold text-(--color-text-primary)">
          {m.preferences_title()}
        </Dialog.Title>
        <Dialog.Close
          class="flex items-center justify-center w-[44px] h-[44px] text-(--color-text-secondary) hover:text-(--color-text-primary) hover:bg-(--color-hover-bg) rounded-[2px]"
          aria-label={m.action_close()}
        >
          <X size={16} aria-hidden="true" />
        </Dialog.Close>
      </div>

      <Dialog.Description class="sr-only">{m.preferences_title()}</Dialog.Description>

      <!-- Body: left nav + right content -->
      <div class="flex flex-1 min-h-0 border-t border-(--color-border)">

        <!-- Section navigation -->
        <nav
          class="w-[180px] flex-shrink-0 border-r border-(--color-border) py-2"
          aria-label={m.preferences_sections_nav()}
        >
          {#each sections as section (section.id)}
            <button
              class="preferences-panel__nav-item w-full text-left px-4 h-[40px] text-[13px] cursor-pointer
                     hover:bg-(--color-hover-bg) focus-visible:outline-2 focus-visible:outline-(--color-focus-ring)"
              class:preferences-panel__nav-item--active={activeSection === section.id}
              onclick={() => { activeSection = section.id; }}
              aria-current={activeSection === section.id ? 'page' : undefined}
            >
              {section.label()}
            </button>
          {/each}
        </nav>

        <!-- Section content -->
        <div class="flex-1 overflow-y-auto p-6">

          <!-- ===== KEYBOARD SECTION ===== -->
          {#if activeSection === 'keyboard'}
            <p class="text-[11px] font-semibold text-(--color-text-tertiary) uppercase tracking-wider mb-4">
              {m.preferences_section_keyboard()}
            </p>
            <div class="space-y-1">
              {#each shortcutActions as action (action.id)}
                <div class="flex items-center justify-between h-[44px]">
                  <span class="text-[13px] text-(--color-text-primary)">{action.label()}</span>
                  <KeyboardShortcutRecorder
                    value={shortcuts[action.id]}
                    existingShortcuts={shortcuts}
                    actionId={action.id}
                    onchange={(s) => handleShortcutChange(action.id, s)}
                  />
                </div>
              {/each}
            </div>

          <!-- ===== APPEARANCE SECTION ===== -->
          {:else if activeSection === 'appearance'}
            <p class="text-[11px] font-semibold text-(--color-text-tertiary) uppercase tracking-wider mb-4">
              {m.preferences_section_appearance()}
            </p>

            <div class="space-y-4">
              <TextInput
                id="pref-font-family"
                label={m.preferences_appearance_font_family()}
                value={preferences?.appearance?.fontFamily ?? ''}
                oninput={handleFontFamilyChange}
              />

              <TextInput
                id="pref-font-size"
                label={m.preferences_appearance_font_size_range()}
                type="number"
                value={String(preferences?.appearance?.fontSize ?? 13)}
                oninput={handleFontSizeChange}
              />

              <Dropdown
                id="pref-language"
                label={m.preferences_appearance_language()}
                options={languageOptions}
                value={preferences?.appearance?.language ?? 'En'}
                onchange={handleLanguageChange}
              />
            </div>

          <!-- ===== TERMINAL BEHAVIOR SECTION ===== -->
          {:else if activeSection === 'terminal'}
            <p class="text-[11px] font-semibold text-(--color-text-tertiary) uppercase tracking-wider mb-4">
              {m.preferences_section_terminal()}
            </p>

            <div class="space-y-4">
              <Dropdown
                id="pref-cursor-shape"
                label={m.preferences_terminal_cursor_style()}
                options={cursorShapeOptions}
                value="block"
              />

              <TextInput
                id="pref-cursor-blink"
                label={m.preferences_terminal_cursor_blink_rate()}
                type="number"
                value="530"
              />

              <TextInput
                id="pref-scrollback"
                label={m.preferences_terminal_scrollback()}
                type="number"
                value={String(preferences?.terminal?.scrollbackLines ?? 10000)}
                oninput={handleScrollbackChange}
                helper={m.preferences_terminal_scrollback_estimate({ mb: scrollbackEstimateMb })}
              />

              <Dropdown
                id="pref-bell"
                label={m.preferences_terminal_bell_type()}
                options={bellTypeOptions}
                value={preferences?.terminal?.bell ? 'audible' : 'visual'}
              />

              <TextInput
                id="pref-word-delimiters"
                label={m.preferences_terminal_word_delimiters()}
                value={' ,.;:{}[]()"\`|\\/'}
              />
            </div>

          <!-- ===== CONNECTIONS SECTION ===== -->
          {:else if activeSection === 'connections'}
            <p class="text-[11px] font-semibold text-(--color-text-tertiary) uppercase tracking-wider mb-4">
              {m.preferences_section_connections()}
            </p>
            <p class="text-[13px] text-(--color-text-secondary) mb-4">
              {m.connection_empty_state()}
            </p>
            <!-- Known-hosts import -->
            <Button variant="secondary" onclick={() => {}}>
              {m.action_import_known_hosts()}
            </Button>
          {/if}

        </div>
      </div>
    </Dialog.Content>
  </Dialog.Portal>
</Dialog.Root>

<style>
  .preferences-panel__nav-item {
    color: var(--color-text-secondary);
    border-left: 2px solid transparent;
    transition: background-color var(--duration-fast) var(--ease-out);
  }

  .preferences-panel__nav-item--active {
    color: var(--color-accent-text);
    border-left-color: var(--color-accent);
    background-color: transparent;
  }
</style>
