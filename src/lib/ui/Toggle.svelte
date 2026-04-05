<!-- SPDX-License-Identifier: MPL-2.0 -->
<!--
  Toggle — accessible on/off switch (ARIA role="switch").

  The hit area wrapper is 44×44px to meet WCAG 2.5.5.
  Track and thumb colors change per state (unchecked/checked/disabled).

  Props:
    checked   — current checked state
    disabled  — disables the toggle
    label     — visible label rendered to the right of the switch
    onchange  — called with new boolean value when toggled
-->
<script lang="ts">
  interface Props {
    checked?: boolean;
    disabled?: boolean;
    label?: string;
    onchange?: (checked: boolean) => void;
  }

  const { checked = false, disabled = false, label, onchange }: Props = $props();

  function handleChange(e: Event) {
    if (!disabled) {
      onchange?.((e.target as HTMLInputElement).checked);
    }
  }

  // Track background classes
  const trackBg = $derived(
    disabled
      ? checked
        ? 'bg-(--color-blue-700)'
        : 'bg-(--color-neutral-750)'
      : checked
        ? 'bg-(--color-accent)'
        : 'bg-(--color-neutral-700)',
  );

  // Thumb color classes
  const thumbBg = $derived(
    disabled
      ? checked
        ? 'bg-(--color-neutral-500)'
        : 'bg-(--color-neutral-600)'
      : checked
        ? 'bg-(--color-neutral-100)'
        : 'bg-(--color-neutral-400)',
  );

  // Thumb position
  const thumbTranslate = $derived(checked ? 'translate-x-[18px]' : 'translate-x-[2px]');
</script>

<label
  class="inline-flex items-center gap-2 cursor-pointer"
  class:cursor-not-allowed={disabled}
  class:opacity-60={disabled}
>
  <!-- Hidden native checkbox for semantics -->
  <input
    type="checkbox"
    role="switch"
    class="sr-only"
    {checked}
    {disabled}
    aria-checked={checked}
    aria-disabled={disabled}
    onchange={handleChange}
  />

  <!-- Visual track — 44×44px hit area wrapper -->
  <span class="flex items-center justify-center w-[44px] h-[44px]">
    <span
      class="relative w-[36px] h-[20px] rounded-full transition-colors duration-[80ms] {trackBg}"
    >
      <span
        class="absolute top-[2px] w-[16px] h-[16px] rounded-full transition-transform duration-[80ms] {thumbBg} {thumbTranslate}"
      ></span>
    </span>
  </span>

  {#if label}
    <span class="text-[13px] text-(--color-text-primary)">{label}</span>
  {/if}
</label>
