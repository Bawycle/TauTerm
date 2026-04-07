<!-- SPDX-License-Identifier: MPL-2.0 -->
<!--
  PreferencesTerminalSection — terminal behavior section of the preferences panel.
  Cursor style/blink, scrollback, bell type, word delimiters.

  Note: cursorStyle and cursorBlinkMs live in AppearancePrefs (not TerminalPrefs)
  per the data model, but are presented in this section (FS-PREF-003, FS-PREF-006).

  Props:
    preferences — current Preferences object
    onupdate    — called with PreferencesPatch when a preference changes
-->
<script lang="ts">
  import TextInput from '$lib/ui/TextInput.svelte';
  import Dropdown from '$lib/ui/Dropdown.svelte';
  import * as m from '$lib/paraglide/messages';
  import type { Preferences, PreferencesPatch, CursorStyle, BellType } from '$lib/ipc/types';

  interface Props {
    preferences?: Preferences;
    onupdate?: (patch: PreferencesPatch) => void;
  }

  let { preferences, onupdate }: Props = $props();

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

  /** Memory estimate: ~5 500 bytes per line per pane (upper bound per arch/07). */
  const scrollbackEstimateMb = $derived(
    preferences?.terminal?.scrollbackLines
      ? Math.round(((preferences.terminal.scrollbackLines * 5500) / (1024 * 1024)) * 10) / 10
      : 0,
  );

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
</script>

<p
  class="text-(--font-size-ui-xs) font-semibold text-(--color-text-tertiary) uppercase tracking-wider mb-4"
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
