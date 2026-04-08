<!-- SPDX-License-Identifier: MPL-2.0 -->
<!--
  ThemePreview — real-time terminal preview for a theme being edited (UXD §7.20.5).
  CSS variables are scoped to the container so the panel chrome is unaffected.

  Props:
    previewStyle — inline style string with --preview-* CSS variables
-->
<script lang="ts">
  import * as m from '$lib/paraglide/messages';

  interface Props {
    previewStyle: string;
  }

  let { previewStyle }: Props = $props();
</script>

<div
  class="theme-preview rounded-(--radius-sm) border border-(--color-border) overflow-hidden"
  style={previewStyle}
  aria-label={m.theme_preview_label()}
  role="img"
>
  <div
    class="p-3 font-mono text-(--font-size-ui-base) leading-relaxed"
    style="background: var(--preview-bg); color: var(--preview-fg);"
  >
    <!-- Simulated terminal output with ANSI colors -->
    <div>
      <span style="color: var(--preview-color-2);">user@host</span>
      <span style="color: var(--preview-fg);">:</span>
      <span style="color: var(--preview-color-4);">~/projects</span>
      <span style="color: var(--preview-fg);">$ </span>
      <span style="color: var(--preview-fg);">ls -la</span>
    </div>
    <div>
      <span style="color: var(--preview-color-6);">drwxr-xr-x</span>
      <span style="color: var(--preview-fg);"> 2 user group 4096 </span>
      <span style="color: var(--preview-color-4);">src/</span>
    </div>
    <div>
      <span style="color: var(--preview-color-1);">error</span>
      <span style="color: var(--preview-fg);">: file not found</span>
    </div>
    <div>
      <span style="color: var(--preview-color-3);">warning</span>
      <span style="color: var(--preview-fg);">: deprecated API</span>
    </div>
    <div>
      <span style="color: var(--preview-fg);">$ </span>
      <!-- Simulated block cursor -->
      <span
        style="background: var(--preview-cursor); color: var(--preview-bg); display: inline-block; width: 0.6em;"
        aria-hidden="true">&nbsp;</span
      >
    </div>
  </div>
</div>
