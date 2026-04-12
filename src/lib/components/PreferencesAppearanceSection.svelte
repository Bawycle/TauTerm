<!-- SPDX-License-Identifier: MPL-2.0 -->
<!--
  PreferencesAppearanceSection — appearance section of the preferences panel.
  Font family, font size, active theme selection, language.

  Props:
    preferences — current Preferences object
    themes      — loaded user themes (for the theme dropdown)
    onupdate    — called with PreferencesPatch when a preference changes
-->
<script lang="ts">
  import TextInput from '$lib/ui/TextInput.svelte';
  import Dropdown from '$lib/ui/Dropdown.svelte';
  import Toggle from '$lib/ui/Toggle.svelte';
  import * as m from '$lib/paraglide/messages';
  import type { Preferences, PreferencesPatch } from '$lib/ipc/types';
  import { BUILT_IN_THEME_NAMES } from '$lib/theming/built-in-themes';
  import type { UserTheme } from '$lib/ipc/types';

  interface Props {
    preferences?: Preferences;
    themes: UserTheme[];
    onupdate?: (patch: PreferencesPatch) => void;
  }

  let { preferences, themes, onupdate }: Props = $props();

  const languageOptions = [
    { value: 'en', label: m.locale_en() },
    { value: 'fr', label: m.locale_fr() },
  ];

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

  function handleThemeChange(val: string) {
    if (!preferences?.appearance) return;
    onupdate?.({ appearance: { ...preferences.appearance, themeName: val } });
  }

  function handleLanguageChange(val: string) {
    if (!preferences?.appearance) return;
    if (val === 'en' || val === 'fr') {
      onupdate?.({ appearance: { ...preferences.appearance, language: val } });
    }
  }

  function handleOpacityChange(val: string) {
    if (!preferences?.appearance) return;
    const n = parseFloat(val);
    if (isNaN(n)) return;
    const clamped = Math.max(0, Math.min(1, Math.round(n * 100) / 100));
    onupdate?.({ appearance: { ...preferences.appearance, opacity: clamped } });
  }

  function handleHideCursorWhileTypingChange(checked: boolean) {
    if (!preferences?.appearance) return;
    onupdate?.({ appearance: { ...preferences.appearance, hideCursorWhileTyping: checked } });
  }

  function handleShowPaneTitleBarChange(checked: boolean) {
    if (!preferences?.appearance) return;
    onupdate?.({ appearance: { ...preferences.appearance, showPaneTitleBar: checked } });
  }
</script>

<p
  class="text-(--font-size-ui-xs) font-semibold text-(--color-text-heading) uppercase mb-4"
  style="letter-spacing: var(--letter-spacing-label)"
>
  {m.preferences_section_appearance()}
</p>

<div class="space-y-4">
  <TextInput
    id="pref-font-family"
    label={m.preferences_appearance_font_family()}
    value={preferences?.appearance?.fontFamily ?? ''}
    helper={m.preferences_appearance_font_family_hint()}
    oninput={handleFontFamilyChange}
  />

  <TextInput
    id="pref-font-size"
    label={m.preferences_appearance_font_size_range()}
    type="number"
    value={String(preferences?.appearance?.fontSize ?? 13)}
    helper={m.preferences_appearance_font_size_hint()}
    oninput={handleFontSizeChange}
  />

  <Dropdown
    id="pref-theme"
    label={m.preferences_appearance_theme()}
    options={[
      ...BUILT_IN_THEME_NAMES.map((name) => ({
        value: name,
        label: name.charAt(0).toUpperCase() + name.slice(1),
      })),
      ...themes.map((t) => ({ value: t.name, label: t.name })),
    ]}
    value={preferences?.appearance?.themeName ?? 'umbra'}
    helper={m.preferences_appearance_theme_hint()}
    onchange={handleThemeChange}
  />

  <Dropdown
    id="pref-language"
    label={m.preferences_appearance_language()}
    options={languageOptions}
    value={preferences?.appearance?.language ?? 'en'}
    helper={m.preferences_appearance_language_hint()}
    onchange={handleLanguageChange}
  />

  <TextInput
    id="pref-opacity"
    label={m.preferences_appearance_opacity()}
    type="number"
    value={String(preferences?.appearance?.opacity ?? 1)}
    helper={m.preferences_appearance_opacity_hint()}
    oninput={handleOpacityChange}
  />

  <Toggle
    checked={preferences?.appearance?.hideCursorWhileTyping ?? true}
    label={m.hide_cursor_while_typing_label()}
    onchange={handleHideCursorWhileTypingChange}
  />

  <Toggle
    checked={preferences?.appearance?.showPaneTitleBar ?? true}
    label={m.show_pane_title_bar_label()}
    onchange={handleShowPaneTitleBarChange}
  />
</div>
