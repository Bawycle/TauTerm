<!-- SPDX-License-Identifier: MPL-2.0 -->
<!--
  ThemeContrastAdvisory — WCAG 2.1 AA contrast warning displayed in the theme editor.
  Rendered only when the fg/bg contrast ratio is below 4.5:1.

  Props:
    contrastRatio — current computed ratio (displayed in the message)
    threshold     — WCAG AA threshold constant
-->
<script lang="ts">
  import * as m from '$lib/paraglide/messages';

  interface Props {
    contrastRatio: number;
    threshold: number;
  }

  let { contrastRatio, threshold }: Props = $props();
</script>

<div
  class="flex items-start gap-2 p-3 rounded-(--radius-sm)
         bg-(--color-warning-bg) border border-(--color-warning)"
  role="alert"
  aria-live="polite"
>
  <span class="text-(--font-size-ui-base) text-(--color-warning-text) leading-snug">
    {m.theme_contrast_warning({
      ratio: contrastRatio.toFixed(2),
      threshold: String(threshold),
    })}
  </span>
</div>
