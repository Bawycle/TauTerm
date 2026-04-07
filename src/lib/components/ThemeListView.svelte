<!-- SPDX-License-Identifier: MPL-2.0 -->
<!--
  ThemeListView — displays the built-in and user-defined theme lists.
  Handles activation, edit initiation, duplication, and deletion via callbacks.

  Props:
    themes            — loaded user themes
    activeThemeName   — currently active theme name
    themeBusy         — whether a delete operation is in progress
    onactivate        — called with theme name to activate
    onedit            — called with a UserTheme to open the editor
    onduplicate       — called with source theme name to duplicate
    ondelete          — called with theme name to delete
    onnew             — called to start creating a new theme
-->
<script lang="ts">
  import Button from '$lib/ui/Button.svelte';
  import { Check, Copy, Pencil, Trash2 } from 'lucide-svelte';
  import * as m from '$lib/paraglide/messages';
  import type { UserTheme } from '$lib/ipc/types';
  import { BUILT_IN_THEME_NAMES, getThemeSwatch } from '$lib/theming/built-in-themes';

  interface Props {
    themes: UserTheme[];
    activeThemeName: string;
    themeBusy: boolean;
    onactivate: (name: string) => void;
    onedit: (theme: UserTheme) => void;
    onduplicate: (sourceName: string) => void;
    ondelete: (name: string) => void;
    onnew: () => void;
  }

  let {
    themes,
    activeThemeName,
    themeBusy,
    onactivate,
    onedit,
    onduplicate,
    ondelete,
    onnew,
  }: Props = $props();

  let hoveredThemeName = $state<string | null>(null);

  function isLightTheme(name: string, userThemes: UserTheme[]): boolean {
    if (name === 'solstice') return true;
    const swatch = getThemeSwatch(name, userThemes);
    const hex = swatch.bg.replace('#', '');
    if (hex.length < 6) return false;
    const r = parseInt(hex.slice(0, 2), 16);
    const g = parseInt(hex.slice(2, 4), 16);
    const b = parseInt(hex.slice(4, 6), 16);
    return (0.299 * r + 0.587 * g + 0.114 * b) > 128;
  }
</script>

<div class="space-y-4">
  <!-- Built-in themes group -->
  <div>
    <p class="text-(--font-size-ui-xs) text-(--color-text-tertiary) uppercase tracking-wider mb-2">
      {m.theme_section_builtin()}
    </p>
    <div class="space-y-1">
      {#each BUILT_IN_THEME_NAMES as themeName (themeName)}
        {@const isActive = activeThemeName === themeName}
        {@const swatch = getThemeSwatch(themeName, themes)}
        {@const isLight = isLightTheme(themeName, themes)}
        <div
          class="flex items-center h-[44px] px-3 rounded-[2px] border cursor-pointer {isActive ? 'border-(--color-accent) bg-(--color-accent-subtle)' : 'border-(--color-border) hover:border-(--color-border-subtle)'}"
          role="button"
          tabindex="0"
          aria-label={themeName}
          aria-pressed={isActive}
          onclick={() => onactivate(themeName)}
          onkeydown={(e) => { if (e.key === 'Enter' || e.key === ' ') { e.preventDefault(); onactivate(themeName); } }}
          onmouseenter={() => { hoveredThemeName = themeName; }}
          onmouseleave={() => { hoveredThemeName = null; }}
        >
          <!-- Active indicator — reserved space always -->
          <div class="w-4 h-4 flex-shrink-0 mr-2 flex items-center justify-center">
            {#if isActive}
              <Check class="w-4 h-4 text-(--color-accent)" />
            {/if}
          </div>

          <!-- Color swatch strip -->
          <div class="flex gap-[2px] mr-3 flex-shrink-0">
            {#each [swatch.bg, swatch.fg, swatch.accent, swatch.cursor, swatch.color1, swatch.color6] as color}
              <div class="w-4 h-4 rounded-sm border border-(--color-border-subtle)" style="background:{color}"></div>
            {/each}
          </div>

          <!-- Name -->
          <span class="text-(--font-size-ui-base) text-(--color-text-primary) flex-1 truncate">
            {themeName.charAt(0).toUpperCase() + themeName.slice(1)}
          </span>

          <!-- Badges -->
          <span class="text-(--font-size-ui-2xs) text-(--color-text-tertiary) ml-2 flex-shrink-0">
            {isLight ? m.theme_light_label() : m.theme_dark_label()}
          </span>
          <span class="text-(--font-size-ui-2xs) text-(--color-text-tertiary) ml-1 mr-2 flex-shrink-0">
            · {m.theme_builtin_label()}
          </span>

          <!-- Duplicate button — visible when active or hovered -->
          <div class="flex-shrink-0 {isActive || hoveredThemeName === themeName ? 'opacity-100' : 'opacity-0 pointer-events-none'}">
            <Button
              variant="ghost"
              class="h-7 w-7 p-0"
              aria-label={m.theme_duplicate_to_edit()}
              onclick={(e) => { e.stopPropagation(); onduplicate(themeName); }}
            >
              <Copy class="w-3.5 h-3.5" />
            </Button>
          </div>
        </div>
      {/each}
    </div>
  </div>

  <!-- My themes group -->
  <div>
    <div class="flex items-center justify-between mb-2">
      <p class="text-(--font-size-ui-xs) text-(--color-text-tertiary) uppercase tracking-wider">
        {m.theme_section_my_themes()}
      </p>
      <Button variant="secondary" class="h-7 text-(--font-size-ui-xs) px-2" onclick={onnew}>
        {m.theme_new()}
      </Button>
    </div>

    {#if themes.length === 0}
      <p class="text-(--font-size-ui-base) text-(--color-text-secondary) mb-3">
        {m.theme_empty_state()}
      </p>
    {:else}
      <div class="space-y-1">
        {#each themes as theme (theme.name)}
          {@const isActive = activeThemeName === theme.name}
          {@const swatch = getThemeSwatch(theme.name, themes)}
          {@const isLight = isLightTheme(theme.name, themes)}
          <div
            class="flex items-center h-[44px] px-3 rounded-[2px] border cursor-pointer {isActive ? 'border-(--color-accent) bg-(--color-accent-subtle)' : 'border-(--color-border) hover:border-(--color-border-subtle)'}"
            role="button"
            tabindex="0"
            aria-label={theme.name}
            aria-pressed={isActive}
            onclick={() => onactivate(theme.name)}
            onkeydown={(e) => { if (e.key === 'Enter' || e.key === ' ') { e.preventDefault(); onactivate(theme.name); } }}
            onmouseenter={() => { hoveredThemeName = theme.name; }}
            onmouseleave={() => { hoveredThemeName = null; }}
          >
            <!-- Active indicator -->
            <div class="w-4 h-4 flex-shrink-0 mr-2 flex items-center justify-center">
              {#if isActive}
                <Check class="w-4 h-4 text-(--color-accent)" />
              {/if}
            </div>

            <!-- Swatch strip -->
            <div class="flex gap-[2px] mr-3 flex-shrink-0">
              {#each [swatch.bg, swatch.fg, swatch.accent, swatch.cursor, swatch.color1, swatch.color6] as color}
                <div class="w-4 h-4 rounded-sm border border-(--color-border-subtle)" style="background:{color}"></div>
              {/each}
            </div>

            <!-- Name -->
            <span class="text-(--font-size-ui-base) text-(--color-text-primary) flex-1 truncate">
              {theme.name}
            </span>

            <!-- Light/Dark badge -->
            <span class="text-(--font-size-ui-2xs) text-(--color-text-tertiary) ml-2 mr-3 flex-shrink-0">
              {isLight ? m.theme_light_label() : m.theme_dark_label()}
            </span>

            <!-- Action buttons -->
            <div class="flex items-center gap-1 flex-shrink-0">
              <Button
                variant="ghost"
                class="h-7 w-7 p-0"
                aria-label={m.theme_edit()}
                disabled={themeBusy}
                onclick={(e) => { e.stopPropagation(); onedit(theme); }}
              >
                <Pencil class="w-3.5 h-3.5" />
              </Button>
              <Button
                variant="ghost"
                class="h-7 w-7 p-0 text-(--color-error)"
                aria-label={m.theme_delete()}
                disabled={themeBusy}
                onclick={(e) => { e.stopPropagation(); ondelete(theme.name); }}
              >
                <Trash2 class="w-3.5 h-3.5" />
              </Button>
            </div>
          </div>
        {/each}
      </div>
    {/if}
  </div>
</div>
