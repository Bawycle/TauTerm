<!-- SPDX-License-Identifier: MPL-2.0 -->
<!--
  PreferencesPanel — modal preferences dialog (UXD §7.6, FS-PREF-001..006).

  Sections: Keyboard, Appearance, Terminal Behavior, Themes, Connections.
  Uses Bits UI Dialog as base for focus trap, Escape close, aria-modal.
  Ctrl+, shortcut is wired in TerminalView.

  Props:
    open        — whether the panel is open (bindable)
    preferences — current Preferences object from get_preferences IPC
    onclose     — called when the dialog should close
    onupdate    — called with PreferencesPatch when a preference changes

  Security: all values rendered as text interpolation, no {@html}.
-->
<script lang="ts">
  import { Dialog } from 'bits-ui';
  import { X } from 'lucide-svelte';
  import { getThemes } from '$lib/ipc/commands';
  import { DEFAULT_SHORTCUTS } from '$lib/preferences/shortcuts';
  import PreferencesSectionNav from './PreferencesSectionNav.svelte';
  import PreferencesKeyboardSection from './PreferencesKeyboardSection.svelte';
  import PreferencesAppearanceSection from './PreferencesAppearanceSection.svelte';
  import PreferencesTerminalSection from './PreferencesTerminalSection.svelte';
  import PreferencesThemesSection from './PreferencesThemesSection.svelte';
  import PreferencesConnectionsSection from './PreferencesConnectionsSection.svelte';
  import * as m from '$lib/paraglide/messages';
  import type { Preferences, PreferencesPatch, UserTheme } from '$lib/ipc/types';

  // ---------------------------------------------------------------------------
  // Props
  // ---------------------------------------------------------------------------

  interface Props {
    open?: boolean;
    preferences?: Preferences;
    onclose?: () => void;
    onupdate?: (patch: PreferencesPatch) => void;
    /** Called by Bits UI FocusScope when the dialog closes, before it restores
     *  focus to the trigger. Typically used to call e.preventDefault() and
     *  restore focus to the terminal viewport instead. */
    onCloseAutoFocus?: (e: Event) => void;
  }

  let {
    open = $bindable(false),
    preferences,
    onclose,
    onupdate,
    onCloseAutoFocus,
  }: Props = $props();

  // ---------------------------------------------------------------------------
  // Section navigation
  // ---------------------------------------------------------------------------

  type Section = 'keyboard' | 'appearance' | 'terminal' | 'themes' | 'connections';
  let activeSection = $state<Section>('keyboard');

  const sections: { id: Section; label: () => string }[] = [
    { id: 'keyboard', label: () => m.preferences_section_keyboard() },
    { id: 'appearance', label: () => m.preferences_section_appearance() },
    { id: 'terminal', label: () => m.preferences_section_terminal() },
    { id: 'themes', label: () => m.preferences_section_themes() },
    { id: 'connections', label: () => m.preferences_section_connections() },
  ];

  // ---------------------------------------------------------------------------
  // Shared themes list — needed by both Appearance and Themes sections.
  // Loaded lazily when either section first becomes active.
  // ---------------------------------------------------------------------------

  let sharedThemes = $state<UserTheme[]>([]);
  let sharedThemesLoaded = $state(false);

  $effect(() => {
    if ((activeSection === 'themes' || activeSection === 'appearance') && !sharedThemesLoaded) {
      getThemes()
        .then((loaded) => {
          sharedThemes = loaded;
          sharedThemesLoaded = true;
        })
        .catch(() => {
          // Error display is handled inside PreferencesThemesSection.
        });
    }
  });

  // ---------------------------------------------------------------------------
  // FS-KBD-002: Keyboard shortcuts — read from preferences, persist on change
  // ---------------------------------------------------------------------------

  /**
   * Effective shortcuts — derived from preferences.keyboard.bindings merged with defaults.
   * Reactive to preferences prop changes: no $effect needed.
   */
  const shortcuts = $derived({
    ...DEFAULT_SHORTCUTS,
    ...(preferences?.keyboard?.bindings ?? {}),
  });

  const shortcutActions: { id: string; label: () => string }[] = [
    { id: 'new_tab', label: () => m.preferences_keyboard_action_label_new_tab() },
    { id: 'close_tab', label: () => m.preferences_keyboard_action_label_close_tab() },
    { id: 'paste', label: () => m.preferences_keyboard_action_label_paste() },
    { id: 'search', label: () => m.preferences_keyboard_action_label_search() },
    { id: 'preferences', label: () => m.preferences_keyboard_action_label_preferences() },
    { id: 'next_tab', label: () => m.preferences_keyboard_action_label_next_tab() },
    { id: 'prev_tab', label: () => m.preferences_keyboard_action_label_prev_tab() },
    { id: 'rename_tab', label: () => m.preferences_keyboard_action_label_rename_tab() },
    {
      id: 'toggle_fullscreen',
      label: () => m.preferences_keyboard_action_label_toggle_fullscreen(),
    },
    { id: 'split_pane_h', label: () => m.preferences_keyboard_action_label_split_pane_h() },
    { id: 'split_pane_v', label: () => m.preferences_keyboard_action_label_split_pane_v() },
    { id: 'close_pane', label: () => m.preferences_keyboard_action_label_close_pane() },
    {
      id: 'navigate_pane_left',
      label: () => m.preferences_keyboard_action_label_navigate_pane_left(),
    },
    {
      id: 'navigate_pane_right',
      label: () => m.preferences_keyboard_action_label_navigate_pane_right(),
    },
    { id: 'navigate_pane_up', label: () => m.preferences_keyboard_action_label_navigate_pane_up() },
    {
      id: 'navigate_pane_down',
      label: () => m.preferences_keyboard_action_label_navigate_pane_down(),
    },
  ];

  async function handleShortcutChange(actionId: string, newShortcut: string) {
    const updated = { ...shortcuts, [actionId]: newShortcut };
    onupdate?.({ keyboard: { bindings: updated } });
  }
</script>

<Dialog.Root
  bind:open
  onOpenChange={(o) => {
    if (!o) onclose?.();
  }}
>
  <Dialog.Portal>
    <Dialog.Overlay class="fixed inset-0 z-(--z-modal-backdrop) bg-(--color-bg-overlay)/60" />

    <Dialog.Content
      class="preferences-panel fixed z-(--z-modal) top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2
             w-[640px] max-w-[90vw] max-h-[80vh]
             bg-(--color-bg-raised) border border-(--color-border-overlay) rounded-(--radius-md)
             shadow-(--shadow-overlay) flex flex-col overflow-hidden"
      aria-modal="true"
      onCloseAutoFocus={(e) => {
        e.preventDefault();
        onCloseAutoFocus?.(e);
      }}
    >
      <!-- Header -->
      <div class="flex items-center justify-between px-6 pt-6 pb-4 flex-shrink-0">
        <Dialog.Title class="text-(--font-size-ui-lg) font-semibold text-(--color-text-primary)">
          {m.preferences_title()}
        </Dialog.Title>
        <Dialog.Close
          class="flex items-center justify-center w-[44px] h-[44px] text-(--color-text-secondary) hover:text-(--color-text-primary) hover:bg-(--color-hover-bg) rounded-(--radius-sm)"
          aria-label={m.action_close()}
        >
          <X size={16} aria-hidden="true" />
        </Dialog.Close>
      </div>

      <Dialog.Description class="sr-only">{m.preferences_title()}</Dialog.Description>

      <!-- Body: left nav + right content -->
      <div class="flex flex-1 min-h-0 border-t border-(--color-border-overlay)">
        <PreferencesSectionNav
          {sections}
          {activeSection}
          onselect={(section) => {
            activeSection = section;
          }}
        />

        <!-- Section content -->
        <div class="flex-1 overflow-y-auto p-6">
          {#if activeSection === 'keyboard'}
            <PreferencesKeyboardSection
              {shortcuts}
              {shortcutActions}
              onshortcutchange={handleShortcutChange}
            />
          {:else if activeSection === 'appearance'}
            <PreferencesAppearanceSection {preferences} themes={sharedThemes} {onupdate} />
          {:else if activeSection === 'terminal'}
            <PreferencesTerminalSection {preferences} {onupdate} />
          {:else if activeSection === 'themes'}
            <PreferencesThemesSection {preferences} {onupdate} />
          {:else if activeSection === 'connections'}
            <PreferencesConnectionsSection />
          {/if}
        </div>
      </div>
    </Dialog.Content>
  </Dialog.Portal>
</Dialog.Root>
