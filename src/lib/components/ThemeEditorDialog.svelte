<!-- SPDX-License-Identifier: MPL-2.0 -->
<!--
  ThemeEditorDialog — theme editing form (create or edit a user theme).
  Also handles the read-only display for built-in themes.

  Props:
    editingTheme  — the theme being edited (must not be null when rendered)
    isNewTheme    — whether we are creating a new theme vs editing an existing one
    themeBusy     — whether a save operation is in progress
    onupdate      — called with an updated UserTheme whenever a field changes
    onsave        — called with the final theme to persist
    oncancel      — called when the user cancels editing
    onduplicate   — called with sourceName to start a duplicate-based new theme
-->
<script lang="ts">
  import TextInput from '$lib/ui/TextInput.svelte';
  import Button from '$lib/ui/Button.svelte';
  import ThemePaletteEditor from './ThemePaletteEditor.svelte';
  import ThemePreview from './ThemePreview.svelte';
  import ThemeContrastAdvisory from './ThemeContrastAdvisory.svelte';
  import { Copy } from 'lucide-svelte';
  import * as m from '$lib/paraglide/messages';
  import type { UserTheme } from '$lib/ipc/types';
  import { contrastRatio, WCAG_AA_THRESHOLD } from '$lib/utils/contrast';
  import { isBuiltInTheme } from '$lib/theming/built-in-themes';

  interface Props {
    editingTheme: UserTheme;
    isNewTheme: boolean;
    themeBusy: boolean;
    onupdate: (updated: UserTheme) => void;
    onsave: (theme: UserTheme) => void;
    oncancel: () => void;
    onduplicate: (sourceName: string) => void;
  }

  let { editingTheme, isNewTheme, themeBusy, onupdate, onsave, oncancel, onduplicate }: Props =
    $props();

  // ---------------------------------------------------------------------------
  // Contrast advisory
  // ---------------------------------------------------------------------------

  const editingContrastRatio = $derived(
    contrastRatio(editingTheme.foreground, editingTheme.background),
  );
  const contrastBelowAA = $derived(editingContrastRatio < WCAG_AA_THRESHOLD);

  // ---------------------------------------------------------------------------
  // Preview CSS variables
  // ---------------------------------------------------------------------------

  const previewStyle = $derived.by(() => {
    const parts: string[] = [
      `--preview-bg: ${editingTheme.background}`,
      `--preview-fg: ${editingTheme.foreground}`,
      `--preview-cursor: ${editingTheme.cursorColor}`,
    ];
    for (let i = 0; i < 16; i++) {
      parts.push(`--preview-color-${i}: ${editingTheme.palette[i]}`);
    }
    return parts.join('; ');
  });

  // ---------------------------------------------------------------------------
  // Helpers
  // ---------------------------------------------------------------------------

  /**
   * Clamp and validate a line_height value to the allowed range [1.0, 2.0].
   * Returns undefined when the input is not a valid number in range.
   */
  function clampLineHeight(val: string): number | undefined {
    const n = parseFloat(val);
    if (isNaN(n)) return undefined;
    return Math.max(1.0, Math.min(2.0, Math.round(n * 10) / 10));
  }

  function handlePaletteColorChange(index: number, value: string) {
    const palette = [...editingTheme.palette] as UserTheme['palette'];
    palette[index] = value;
    onupdate({ ...editingTheme, palette });
  }
</script>

{#if isBuiltInTheme(editingTheme.name)}
  <!-- Read-only display for built-in themes -->
  <div class="space-y-4">
    <p class="text-(--font-size-ui-base) font-medium text-(--color-text-primary) mb-2">
      {editingTheme.name.charAt(0).toUpperCase() + editingTheme.name.slice(1)}
    </p>
    <p class="text-(--font-size-ui-sm) text-(--color-text-secondary) mb-4">
      {m.theme_builtin_label()}
    </p>
    <!-- Read-only color swatches display -->
    <div class="flex gap-1 mb-4">
      {#each editingTheme.palette as color}
        <div
          class="w-5 h-5 rounded-sm border border-(--color-border)"
          style="background:{color}"
          title={color}
        ></div>
      {/each}
    </div>
    <div class="flex gap-2">
      <Button variant="secondary" onclick={() => onduplicate(editingTheme.name)}>
        <Copy class="w-4 h-4 mr-2" />
        {m.theme_duplicate_to_edit()}
      </Button>
      <Button variant="ghost" onclick={oncancel}>
        {m.theme_cancel()}
      </Button>
    </div>
  </div>
{:else}
  <!-- Theme editor form -->
  <div class="space-y-4">
    <p class="text-(--font-size-ui-base) font-medium text-(--color-text-primary) mb-2">
      {isNewTheme ? m.theme_create_title() : m.theme_edit_title()}
    </p>

    <TextInput
      id="theme-name"
      label={m.theme_name_label()}
      value={editingTheme.name}
      oninput={(val) => onupdate({ ...editingTheme, name: val })}
    />

    <TextInput
      id="theme-fg"
      label={m.theme_foreground_label()}
      value={editingTheme.foreground}
      oninput={(val) => onupdate({ ...editingTheme, foreground: val })}
    />

    <TextInput
      id="theme-bg"
      label={m.theme_background_label()}
      value={editingTheme.background}
      oninput={(val) => onupdate({ ...editingTheme, background: val })}
    />

    <TextInput
      id="theme-cursor"
      label={m.theme_cursor_color_label()}
      value={editingTheme.cursorColor}
      oninput={(val) => onupdate({ ...editingTheme, cursorColor: val })}
    />

    <TextInput
      id="theme-selection"
      label={m.theme_selection_bg_label()}
      value={editingTheme.selectionBg}
      oninput={(val) => onupdate({ ...editingTheme, selectionBg: val })}
    />

    <!-- Line height (FS-THEME-010): range 1.0–2.0, step 0.1 -->
    <div class="flex flex-col gap-1">
      <label
        for="theme-line-height"
        class="text-(--font-size-ui-sm) font-medium text-(--color-text-secondary)"
      >
        {m.theme_line_height_label()}
      </label>
      <input
        id="theme-line-height"
        type="number"
        min="1.0"
        max="2.0"
        step="0.1"
        value={editingTheme.lineHeight ?? ''}
        placeholder="1.2"
        class="h-[36px] px-3 rounded-[2px] border border-(--color-border)
               bg-(--color-bg-input) text-(--color-text-primary) text-(--font-size-ui-base)
               focus:outline-2 focus:outline-(--color-focus-ring)"
        oninput={(e) => {
          const val = (e.currentTarget as HTMLInputElement).value;
          const clamped = clampLineHeight(val);
          onupdate({ ...editingTheme, lineHeight: clamped });
        }}
        aria-label={m.theme_line_height_label()}
      />
      <span class="text-(--font-size-ui-xs) text-(--color-text-tertiary)"
        >{m.theme_line_height_hint()}</span
      >
    </div>

    <ThemePaletteEditor palette={editingTheme.palette} onpalettechange={handlePaletteColorChange} />

    <!-- Contrast advisory (UXD §7.20.4) -->
    {#if contrastBelowAA}
      <ThemeContrastAdvisory contrastRatio={editingContrastRatio} threshold={WCAG_AA_THRESHOLD} />
    {/if}

    <!-- Real-time preview (UXD §7.20.5) -->
    <ThemePreview {previewStyle} />

    <div class="flex gap-2 pt-2">
      <Button
        variant="primary"
        disabled={themeBusy || !editingTheme.name.trim()}
        onclick={() => onsave(editingTheme)}
      >
        {m.theme_save()}
      </Button>
      <Button variant="ghost" onclick={oncancel}>
        {m.theme_cancel()}
      </Button>
    </div>
  </div>
{/if}
