<!-- SPDX-License-Identifier: MPL-2.0 -->
<!--
  TextInput — labelled text input with optional error and helper text.

  Props:
    value       — current value (bindable via onchange/oninput)
    placeholder — placeholder text
    disabled    — disables the input
    error       — error message; adds red border and aria-invalid
    label       — visible label rendered above the input
    id          — HTML id for label association
    helper      — helper text shown below (hidden when error is shown)
    type        — input type, defaults to "text"
    maxlength   — maximum character count
    onchange    — called with new value on change event
    oninput     — called with new value on input event

  Accessibility: aria-invalid + aria-describedby wired to error/helper.
-->
<script lang="ts">
  interface Props {
    value?: string;
    placeholder?: string;
    disabled?: boolean;
    error?: string;
    label?: string;
    id?: string;
    helper?: string;
    type?: string;
    maxlength?: number;
    onchange?: (value: string) => void;
    oninput?: (value: string) => void;
  }

  const {
    value = '',
    placeholder,
    disabled = false,
    error,
    label,
    id,
    helper,
    type = 'text',
    maxlength,
    onchange,
    oninput,
  }: Props = $props();

  // Stable per-instance fallback ID computed once at instantiation.
  // Used when the caller does not pass `id`, so aria-describedby always points
  // to a valid, unique element — never the literal string "undefined-error".
  const _fallbackId = `ti-${Math.random().toString(36).slice(2, 8)}`;
  const uid = $derived(id ?? _fallbackId);

  const baseInputClasses =
    'w-full h-[44px] px-3 text-(--font-size-ui-base) text-(--color-text-primary) bg-(--term-bg) rounded-(--radius-sm) border placeholder:text-(--color-text-tertiary) focus-visible:outline-2 focus-visible:outline-(--color-focus-ring) focus-visible:outline-offset-[-2px] disabled:border-(--color-border-subtle) disabled:text-(--color-text-tertiary) disabled:cursor-not-allowed';

  const inputClasses = $derived(
    error
      ? `${baseInputClasses} border-(--color-error)`
      : `${baseInputClasses} border-(--color-border)`,
  );

  const describedBy = $derived(error ? `${uid}-error` : helper ? `${uid}-helper` : undefined);
</script>

<div class="flex flex-col">
  {#if label}
    <label
      for={uid}
      class="block text-(--font-size-ui-sm) font-medium text-(--color-text-secondary) mb-1"
    >
      {label}
    </label>
  {/if}

  <input
    id={uid}
    {type}
    {placeholder}
    {disabled}
    {maxlength}
    {value}
    aria-invalid={!!error}
    aria-describedby={describedBy}
    class={inputClasses}
    oninput={(e) => oninput?.((e.target as HTMLInputElement).value)}
    onchange={(e) => onchange?.((e.target as HTMLInputElement).value)}
  />

  {#if error}
    <p id="{uid}-error" class="text-(--font-size-ui-sm) text-(--color-error-text) mt-1">{error}</p>
  {:else if helper}
    <p id="{uid}-helper" class="text-(--font-size-ui-sm) text-(--color-text-secondary) mt-1">
      {helper}
    </p>
  {/if}
</div>
