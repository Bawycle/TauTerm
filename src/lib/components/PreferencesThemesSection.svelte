<!-- SPDX-License-Identifier: MPL-2.0 -->
<!--
  PreferencesThemesSection — orchestrates the themes section of the preferences panel.
  Owns all theme state and IPC calls. Delegates rendering to ThemeListView and ThemeEditorDialog.

  Props:
    preferences — current Preferences object
    onupdate    — called with PreferencesPatch when active theme changes
-->
<script lang="ts">
  import ThemeListView from './ThemeListView.svelte';
  import ThemeEditorDialog from './ThemeEditorDialog.svelte';
  import { invoke } from '@tauri-apps/api/core';
  import * as m from '$lib/paraglide/messages';
  import type { Preferences, PreferencesPatch, UserTheme } from '$lib/ipc/types';
  import { isBuiltInTheme, getBuiltInThemeTokens } from '$lib/theming/built-in-themes';
  import { buildMinimalValidTheme } from '$lib/theming/validate';

  interface Props {
    preferences?: Preferences;
    onupdate?: (patch: PreferencesPatch) => void;
  }

  let { preferences, onupdate }: Props = $props();

  // ---------------------------------------------------------------------------
  // State — stays in this orchestrator, not hoisted to the parent panel
  // ---------------------------------------------------------------------------

  let themes = $state<UserTheme[]>([]);
  let themesLoaded = $state(false);
  let themeBusy = $state(false);
  let themeError = $state<string | null>(null);
  let editingTheme = $state<UserTheme | null>(null);
  let isNewTheme = $state(false);

  // Load themes when this section first mounts (active guard managed by parent via $effect).
  $effect(() => {
    if (!themesLoaded) {
      loadThemes();
    }
  });

  // ---------------------------------------------------------------------------
  // IPC
  // ---------------------------------------------------------------------------

  async function loadThemes() {
    try {
      themes = await invoke<UserTheme[]>('get_themes');
      themesLoaded = true;
    } catch {
      themeError = m.theme_error_load();
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
      themes = await invoke<UserTheme[]>('get_themes');
      editingTheme = null;
      isNewTheme = false;
    } catch {
      themeError = m.theme_error_save();
    } finally {
      themeBusy = false;
    }
  }

  async function handleDeleteTheme(name: string) {
    if (isBuiltInTheme(name)) return;
    themeBusy = true;
    themeError = null;
    try {
      await invoke('delete_theme', { name });
      themes = themes.filter((t) => t.name !== name);
      if (preferences?.appearance?.themeName === name) {
        onupdate?.({ appearance: { ...preferences.appearance, themeName: 'umbra' } });
      }
    } catch {
      themeError = m.theme_error_delete();
    } finally {
      themeBusy = false;
    }
  }

  // ---------------------------------------------------------------------------
  // Editor lifecycle
  // ---------------------------------------------------------------------------

  function handleNewTheme() {
    const defaults = buildMinimalValidTheme();
    editingTheme = {
      name: '',
      palette: [
        defaults['term-color-0'],
        defaults['term-color-1'],
        defaults['term-color-2'],
        defaults['term-color-3'],
        defaults['term-color-4'],
        defaults['term-color-5'],
        defaults['term-color-6'],
        defaults['term-color-7'],
        defaults['term-color-8'],
        defaults['term-color-9'],
        defaults['term-color-10'],
        defaults['term-color-11'],
        defaults['term-color-12'],
        defaults['term-color-13'],
        defaults['term-color-14'],
        defaults['term-color-15'],
      ] as UserTheme['palette'],
      foreground: defaults['term-fg'],
      background: defaults['term-bg'],
      cursorColor: defaults['term-cursor-bg'],
      selectionBg: defaults['term-selection-bg'],
    };
    isNewTheme = true;
    themeError = null;
  }

  function handleDuplicateTheme(sourceName: string) {
    const defaults = buildMinimalValidTheme();
    const userSource = themes.find((t) => t.name === sourceName);
    const builtInTokens = isBuiltInTheme(sourceName) ? getBuiltInThemeTokens(sourceName) : null;
    const resolveToken = (key: string): string => builtInTokens?.[key] ?? defaults[key] ?? '';

    const basePalette = userSource
      ? ([...userSource.palette] as UserTheme['palette'])
      : ([
          resolveToken('term-color-0'),
          resolveToken('term-color-1'),
          resolveToken('term-color-2'),
          resolveToken('term-color-3'),
          resolveToken('term-color-4'),
          resolveToken('term-color-5'),
          resolveToken('term-color-6'),
          resolveToken('term-color-7'),
          resolveToken('term-color-8'),
          resolveToken('term-color-9'),
          resolveToken('term-color-10'),
          resolveToken('term-color-11'),
          resolveToken('term-color-12'),
          resolveToken('term-color-13'),
          resolveToken('term-color-14'),
          resolveToken('term-color-15'),
        ] as UserTheme['palette']);

    editingTheme = {
      name: `Copy of ${sourceName}`,
      palette: basePalette,
      foreground: userSource?.foreground ?? resolveToken('term-fg'),
      background: userSource?.background ?? resolveToken('term-bg'),
      cursorColor: userSource?.cursorColor ?? resolveToken('term-cursor-bg'),
      selectionBg: userSource?.selectionBg ?? resolveToken('term-selection-bg'),
      lineHeight: userSource?.lineHeight,
    };
    isNewTheme = true;
    themeError = null;
  }

  function handleEditTheme(theme: UserTheme) {
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
</script>

<p
  class="text-(--font-size-ui-xs) font-semibold text-(--color-text-tertiary) uppercase tracking-wider mb-4"
>
  {m.preferences_section_themes()}
</p>

{#if themeError}
  <p class="text-(--font-size-ui-base) text-(--color-error-text) mb-3" role="alert">
    {themeError}
  </p>
{/if}

{#if editingTheme !== null}
  <ThemeEditorDialog
    {editingTheme}
    {isNewTheme}
    {themeBusy}
    onupdate={(updated) => {
      editingTheme = updated;
    }}
    onsave={handleSaveTheme}
    oncancel={handleCancelEdit}
    onduplicate={handleDuplicateTheme}
  />
{:else}
  <ThemeListView
    {themes}
    activeThemeName={preferences?.appearance?.themeName ?? 'umbra'}
    {themeBusy}
    onactivate={handleActivateTheme}
    onedit={handleEditTheme}
    onduplicate={handleDuplicateTheme}
    ondelete={handleDeleteTheme}
    onnew={handleNewTheme}
  />
{/if}
