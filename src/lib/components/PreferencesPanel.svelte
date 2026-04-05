<!-- SPDX-License-Identifier: MPL-2.0 -->
<!--
  PreferencesPanel — modal preferences dialog (UXD §7.6, FS-PREF-001..006).

  Sections: Keyboard, Appearance, Terminal Behavior, Themes, Connections.
  Uses ui/Dialog as base for focus trap, Escape close, aria-modal.
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
  import TextInput from '$lib/ui/TextInput.svelte';
  import Dropdown from '$lib/ui/Dropdown.svelte';
  import Button from '$lib/ui/Button.svelte';
  import KeyboardShortcutRecorder from './KeyboardShortcutRecorder.svelte';
  import { invoke } from '@tauri-apps/api/core';
  import * as m from '$lib/paraglide/messages';
  import type {
    Preferences,
    PreferencesPatch,
    UserTheme,
    CursorStyle,
    BellType,
  } from '$lib/ipc/types';

  // ---------------------------------------------------------------------------
  // Props
  // ---------------------------------------------------------------------------

  interface Props {
    open?: boolean;
    preferences?: Preferences;
    onclose?: () => void;
    onupdate?: (patch: PreferencesPatch) => void;
  }

  let { open = $bindable(false), preferences, onclose, onupdate }: Props = $props();

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
  // FS-KBD-002: Keyboard shortcuts — read from preferences, persist on change
  // ---------------------------------------------------------------------------

  /**
   * Default shortcuts used as fallback when preferences.keyboard.bindings is empty.
   * Matches the hardcoded shortcuts in TerminalView.handleGlobalKeydown.
   */
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

  /**
   * Effective shortcuts — derived from preferences.keyboard.bindings merged with defaults.
   * Reactive to preferences prop changes: no $effect needed.
   */
  const shortcuts = $derived({
    ...defaultShortcuts,
    ...(preferences?.keyboard?.bindings ?? {}),
  });

  async function handleShortcutChange(actionId: string, newShortcut: string) {
    // Build updated bindings from current effective shortcuts, then persist.
    const updated = { ...shortcuts, [actionId]: newShortcut };
    onupdate?.({ keyboard: { bindings: updated } });
  }

  // ---------------------------------------------------------------------------
  // Appearance
  // ---------------------------------------------------------------------------

  function handleFontFamilyChange(val: string) {
    if (!preferences?.appearance) return;
    onupdate?.({ appearance: { ...preferences.appearance, fontFamily: val } });
  }

  function handleFontSizeChange(val: string) {
    if (!preferences?.appearance) return;
    const n = parseInt(val, 10);
    if (isNaN(n)) return;
    const clamped = Math.max(8, Math.min(32, n));
    onupdate?.({ appearance: { ...preferences.appearance, fontSize: clamped } });
  }

  function handleLanguageChange(val: string) {
    if (!preferences?.appearance) return;
    if (val === 'en' || val === 'fr') {
      onupdate?.({ appearance: { ...preferences.appearance, language: val } });
    }
  }

  const languageOptions = [
    { value: 'en', label: m.locale_en() },
    { value: 'fr', label: m.locale_fr() },
  ];

  // ---------------------------------------------------------------------------
  // Terminal behavior (FS-PREF-003, FS-PREF-006)
  // cursorStyle and cursorBlinkMs live in AppearancePrefs (not TerminalPrefs).
  // ---------------------------------------------------------------------------

  const cursorShapeOptions = $derived([
    { value: 'block', label: m.preferences_cursor_shape_block() },
    { value: 'underline', label: m.preferences_cursor_shape_underline() },
    { value: 'bar', label: m.preferences_cursor_shape_bar() },
  ]);

  const bellTypeOptions = $derived([
    { value: 'none', label: m.preferences_bell_type_disabled() },
    { value: 'visual', label: m.preferences_bell_type_visual() },
    { value: 'audio', label: m.preferences_bell_type_audible() },
    { value: 'both', label: m.preferences_bell_type_both() },
  ]);

  function handleCursorStyleChange(val: string) {
    if (!preferences?.appearance) return;
    const allowed: CursorStyle[] = ['block', 'underline', 'bar'];
    if (!allowed.includes(val as CursorStyle)) return;
    onupdate?.({ appearance: { ...preferences.appearance, cursorStyle: val as CursorStyle } });
  }

  function handleCursorBlinkRateChange(val: string) {
    if (!preferences?.appearance) return;
    const n = parseInt(val, 10);
    if (isNaN(n) || n < 100 || n > 2000) return;
    onupdate?.({ appearance: { ...preferences.appearance, cursorBlinkMs: n } });
  }

  function handleBellTypeChange(val: string) {
    if (!preferences?.terminal) return;
    const allowed: BellType[] = ['none', 'visual', 'audio', 'both'];
    if (!allowed.includes(val as BellType)) return;
    onupdate?.({ terminal: { ...preferences.terminal, bellType: val as BellType } });
  }

  function handleScrollbackChange(val: string) {
    if (!preferences?.terminal) return;
    const n = parseInt(val, 10);
    if (!isNaN(n) && n > 0) {
      onupdate?.({ terminal: { ...preferences.terminal, scrollbackLines: n } });
    }
  }

  function handleWordDelimitersChange(val: string) {
    if (!preferences?.terminal) return;
    onupdate?.({ terminal: { ...preferences.terminal, wordDelimiters: val } });
  }

  /** Rough memory estimate: ~200 bytes per line per pane. */
  const scrollbackEstimateMb = $derived(
    preferences?.terminal?.scrollbackLines
      ? Math.round(((preferences.terminal.scrollbackLines * 200) / (1024 * 1024)) * 10) / 10
      : 0,
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

  // ---------------------------------------------------------------------------
  // Tâche #15: Theme editor (FS-THEME-003..006)
  // ---------------------------------------------------------------------------

  /** The default built-in theme name — cannot be deleted. */
  const DEFAULT_THEME_NAME = 'umbra';

  /** All user themes loaded from backend. */
  let themes = $state<UserTheme[]>([]);
  /** Whether the themes list has been loaded. */
  let themesLoaded = $state(false);
  /** Whether a save/delete operation is in progress. */
  let themeBusy = $state(false);
  /** Error message for theme operations. */
  let themeError = $state<string | null>(null);

  /** The currently edited theme (null = not editing). */
  let editingTheme = $state<UserTheme | null>(null);
  /** Whether we are creating a new theme (vs editing an existing one). */
  let isNewTheme = $state(false);

  /** Load themes when the Themes section becomes active. */
  $effect(() => {
    if (activeSection === 'themes' && !themesLoaded) {
      loadThemes();
    }
  });

  async function loadThemes() {
    try {
      themes = await invoke<UserTheme[]>('get_themes');
      themesLoaded = true;
    } catch {
      themeError = 'Failed to load themes';
    }
  }

  async function handleActivateTheme(name: string) {
    if (!preferences?.appearance) return;
    themeBusy = true;
    themeError = null;
    try {
      onupdate?.({ appearance: { ...preferences.appearance, themeName: name } });
    } finally {
      themeBusy = false;
    }
  }

  async function handleSaveTheme(theme: UserTheme) {
    themeBusy = true;
    themeError = null;
    try {
      await invoke('save_theme', { theme });
      // Reload full list to reflect any changes.
      themes = await invoke<UserTheme[]>('get_themes');
      editingTheme = null;
      isNewTheme = false;
    } catch {
      themeError = 'Failed to save theme';
    } finally {
      themeBusy = false;
    }
  }

  async function handleDeleteTheme(name: string) {
    if (name === DEFAULT_THEME_NAME) return;
    themeBusy = true;
    themeError = null;
    try {
      await invoke('delete_theme', { name });
      themes = themes.filter((t) => t.name !== name);
      // If the deleted theme was active, reset to default.
      if (preferences?.appearance?.themeName === name) {
        onupdate?.({ appearance: { ...preferences.appearance, themeName: DEFAULT_THEME_NAME } });
      }
    } catch {
      themeError = 'Failed to delete theme';
    } finally {
      themeBusy = false;
    }
  }

  function handleNewTheme() {
    const blankPalette: UserTheme['palette'] = [
      '#000000',
      '#800000',
      '#008000',
      '#808000',
      '#000080',
      '#800080',
      '#008080',
      '#c0c0c0',
      '#808080',
      '#ff0000',
      '#00ff00',
      '#ffff00',
      '#0000ff',
      '#ff00ff',
      '#00ffff',
      '#ffffff',
    ];
    editingTheme = {
      name: '',
      palette: blankPalette,
      foreground: '#c0c0c0',
      background: '#000000',
      cursorColor: '#ffffff',
      selectionBg: '#4040a0',
    };
    isNewTheme = true;
    themeError = null;
  }

  function handleEditTheme(theme: UserTheme) {
    // Deep-copy so edits don't mutate the list before save.
    editingTheme = {
      ...theme,
      palette: [...theme.palette] as UserTheme['palette'],
    };
    isNewTheme = false;
    themeError = null;
  }

  function handleCancelEdit() {
    editingTheme = null;
    isNewTheme = false;
    themeError = null;
  }

  /**
   * Update a single palette slot in the editing theme.
   * index must be 0–15.
   */
  function handlePaletteColorChange(index: number, value: string) {
    if (!editingTheme) return;
    const palette = [...editingTheme.palette] as UserTheme['palette'];
    palette[index] = value;
    editingTheme = { ...editingTheme, palette };
  }

  /** Build a 16-color palette row label (e.g. "Color 0"). */
  function paletteLabel(index: number): string {
    return m.theme_color_index({ index: String(index) });
  }
</script>

<Dialog.Root
  bind:open
  onOpenChange={(o) => {
    if (!o) onclose?.();
  }}
>
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
              onclick={() => {
                activeSection = section.id;
              }}
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
            <p
              class="text-[11px] font-semibold text-(--color-text-tertiary) uppercase tracking-wider mb-4"
            >
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
            <p
              class="text-[11px] font-semibold text-(--color-text-tertiary) uppercase tracking-wider mb-4"
            >
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
                value={preferences?.appearance?.language ?? 'en'}
                onchange={handleLanguageChange}
              />
            </div>

            <!-- ===== TERMINAL BEHAVIOR SECTION ===== -->
          {:else if activeSection === 'terminal'}
            <p
              class="text-[11px] font-semibold text-(--color-text-tertiary) uppercase tracking-wider mb-4"
            >
              {m.preferences_section_terminal()}
            </p>

            <div class="space-y-4">
              <!-- cursorStyle lives in AppearancePrefs (not TerminalPrefs) -->
              <Dropdown
                id="pref-cursor-shape"
                label={m.preferences_terminal_cursor_style()}
                options={cursorShapeOptions}
                value={preferences?.appearance?.cursorStyle ?? 'block'}
                onchange={handleCursorStyleChange}
              />

              <!-- cursorBlinkMs lives in AppearancePrefs -->
              <TextInput
                id="pref-cursor-blink"
                label={m.preferences_terminal_cursor_blink_rate()}
                type="number"
                value={String(preferences?.appearance?.cursorBlinkMs ?? 530)}
                oninput={handleCursorBlinkRateChange}
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
                value={preferences?.terminal?.bellType ?? 'visual'}
                onchange={handleBellTypeChange}
              />

              <TextInput
                id="pref-word-delimiters"
                label={m.preferences_terminal_word_delimiters()}
                value={preferences?.terminal?.wordDelimiters ?? ' ,.;:{}[]()"`|\\/'}
                oninput={handleWordDelimitersChange}
              />
            </div>

            <!-- ===== THEMES SECTION ===== -->
          {:else if activeSection === 'themes'}
            <p
              class="text-[11px] font-semibold text-(--color-text-tertiary) uppercase tracking-wider mb-4"
            >
              {m.preferences_section_themes()}
            </p>

            {#if themeError}
              <p class="text-[13px] text-(--color-error-text) mb-3" role="alert">
                {themeError}
              </p>
            {/if}

            {#if editingTheme !== null}
              <!-- ---- Theme editor form ---- -->
              <div class="space-y-4">
                <p class="text-[13px] font-medium text-(--color-text-primary) mb-2">
                  {isNewTheme ? m.theme_create_title() : m.theme_edit_title()}
                </p>

                <TextInput
                  id="theme-name"
                  label={m.theme_name_label()}
                  value={editingTheme.name}
                  oninput={(val) => {
                    if (editingTheme) editingTheme = { ...editingTheme, name: val };
                  }}
                />

                <TextInput
                  id="theme-fg"
                  label={m.theme_foreground_label()}
                  value={editingTheme.foreground}
                  oninput={(val) => {
                    if (editingTheme) editingTheme = { ...editingTheme, foreground: val };
                  }}
                />

                <TextInput
                  id="theme-bg"
                  label={m.theme_background_label()}
                  value={editingTheme.background}
                  oninput={(val) => {
                    if (editingTheme) editingTheme = { ...editingTheme, background: val };
                  }}
                />

                <TextInput
                  id="theme-cursor"
                  label={m.theme_cursor_color_label()}
                  value={editingTheme.cursorColor}
                  oninput={(val) => {
                    if (editingTheme) editingTheme = { ...editingTheme, cursorColor: val };
                  }}
                />

                <TextInput
                  id="theme-selection"
                  label={m.theme_selection_bg_label()}
                  value={editingTheme.selectionBg}
                  oninput={(val) => {
                    if (editingTheme) editingTheme = { ...editingTheme, selectionBg: val };
                  }}
                />

                <fieldset class="border border-(--color-border) rounded-[2px] p-3">
                  <legend
                    class="text-[11px] font-semibold text-(--color-text-tertiary) uppercase tracking-wider px-1"
                  >
                    {m.theme_palette_label()}
                  </legend>
                  <div class="grid grid-cols-2 gap-2 mt-2">
                    {#each editingTheme.palette as color, i (i)}
                      <div class="flex items-center gap-2">
                        <input
                          type="color"
                          id="theme-palette-{i}"
                          value={color}
                          class="w-[44px] h-[32px] cursor-pointer rounded-[2px] border border-(--color-border) bg-transparent"
                          oninput={(e) =>
                            handlePaletteColorChange(
                              i,
                              (e.currentTarget as HTMLInputElement).value,
                            )}
                          aria-label={paletteLabel(i)}
                        />
                        <TextInput
                          id="theme-palette-text-{i}"
                          label={paletteLabel(i)}
                          value={color}
                          oninput={(val) => handlePaletteColorChange(i, val)}
                        />
                      </div>
                    {/each}
                  </div>
                </fieldset>

                <div class="flex gap-2 pt-2">
                  <Button
                    variant="primary"
                    disabled={themeBusy || !editingTheme.name.trim()}
                    onclick={() => {
                      if (editingTheme) handleSaveTheme(editingTheme);
                    }}
                  >
                    {m.theme_save()}
                  </Button>
                  <Button variant="ghost" onclick={handleCancelEdit}>
                    {m.theme_cancel()}
                  </Button>
                </div>
              </div>
            {:else}
              <!-- ---- Theme list ---- -->
              <div class="space-y-4">
                <!-- Built-in theme row -->
                <div>
                  <p class="text-[11px] text-(--color-text-tertiary) uppercase tracking-wider mb-2">
                    {m.theme_section_builtin()}
                  </p>
                  <div
                    class="flex items-center justify-between h-[44px] px-3 rounded-[2px] border border-(--color-border)"
                  >
                    <span class="text-[13px] text-(--color-text-primary)">
                      {m.theme_default_label()}
                    </span>
                    <div class="flex items-center gap-2">
                      {#if preferences?.appearance?.themeName === DEFAULT_THEME_NAME}
                        <span class="text-[11px] text-(--color-accent-text) font-medium">
                          {m.theme_active_label()}
                        </span>
                      {:else}
                        <Button
                          variant="secondary"
                          disabled={themeBusy}
                          onclick={() => handleActivateTheme(DEFAULT_THEME_NAME)}
                        >
                          {m.theme_activate()}
                        </Button>
                      {/if}
                    </div>
                  </div>
                </div>

                <!-- Custom themes -->
                <div>
                  <p class="text-[11px] text-(--color-text-tertiary) uppercase tracking-wider mb-2">
                    {m.theme_section_custom()}
                  </p>

                  {#if themes.length === 0}
                    <p class="text-[13px] text-(--color-text-secondary) mb-3">
                      {m.theme_empty_state()}
                    </p>
                  {:else}
                    <div class="space-y-1">
                      {#each themes as theme (theme.name)}
                        <div
                          class="flex items-center justify-between h-[44px] px-3 rounded-[2px] border border-(--color-border)"
                        >
                          <span class="text-[13px] text-(--color-text-primary) truncate mr-2">
                            {theme.name}
                          </span>
                          <div class="flex items-center gap-1 flex-shrink-0">
                            {#if preferences?.appearance?.themeName === theme.name}
                              <span class="text-[11px] text-(--color-accent-text) font-medium mr-1">
                                {m.theme_active_label()}
                              </span>
                            {:else}
                              <Button
                                variant="secondary"
                                disabled={themeBusy}
                                onclick={() => handleActivateTheme(theme.name)}
                              >
                                {m.theme_activate()}
                              </Button>
                            {/if}
                            <Button
                              variant="ghost"
                              disabled={themeBusy}
                              onclick={() => handleEditTheme(theme)}
                            >
                              {m.theme_edit()}
                            </Button>
                            <Button
                              variant="destructive"
                              disabled={themeBusy}
                              onclick={() => handleDeleteTheme(theme.name)}
                            >
                              {m.theme_delete()}
                            </Button>
                          </div>
                        </div>
                      {/each}
                    </div>
                  {/if}
                </div>

                <Button variant="secondary" onclick={handleNewTheme}>
                  {m.theme_new()}
                </Button>
              </div>
            {/if}

            <!-- ===== CONNECTIONS SECTION ===== -->
          {:else if activeSection === 'connections'}
            <p
              class="text-[11px] font-semibold text-(--color-text-tertiary) uppercase tracking-wider mb-4"
            >
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
