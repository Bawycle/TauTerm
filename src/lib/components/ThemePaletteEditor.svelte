<!-- SPDX-License-Identifier: MPL-2.0 -->
<!--
  ThemePaletteEditor — 16-slot color palette editor used inside ThemeEditorDialog.

  Props:
    palette       — array of 16 hex color strings
    onpalettechange — called with (index, value) when a slot changes
-->
<script lang="ts">
  import TextInput from '$lib/ui/TextInput.svelte';
  import * as m from '$lib/paraglide/messages';
  import type { UserTheme } from '$lib/ipc/types';

  interface Props {
    palette: UserTheme['palette'];
    onpalettechange: (index: number, value: string) => void;
  }

  let { palette, onpalettechange }: Props = $props();

  function paletteLabel(index: number): string {
    return m.theme_color_index({ index: String(index) });
  }
</script>

<fieldset class="border border-(--color-border) rounded-(--radius-sm) p-3">
  <legend
    class="text-(--font-size-ui-xs) font-semibold text-(--color-text-tertiary) uppercase tracking-wider px-1"
  >
    {m.theme_palette_label()}
  </legend>
  <div class="grid grid-cols-2 gap-2 mt-2">
    {#each palette as color, i (i)}
      <div class="flex items-center gap-2">
        <input
          type="color"
          id="theme-palette-{i}"
          value={color}
          class="w-[44px] h-[32px] cursor-pointer rounded-(--radius-sm) border border-(--color-border) bg-transparent"
          oninput={(e) => onpalettechange(i, (e.currentTarget as HTMLInputElement).value)}
          aria-label={paletteLabel(i)}
        />
        <TextInput
          id="theme-palette-text-{i}"
          label={paletteLabel(i)}
          value={color}
          oninput={(val) => onpalettechange(i, val)}
        />
      </div>
    {/each}
  </div>
</fieldset>
