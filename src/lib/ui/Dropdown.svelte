<!-- SPDX-License-Identifier: MPL-2.0 -->
<!--
  Dropdown — single-select listbox built on Bits UI Select (v2 API).

  Bits UI v2 Select.Root requires `type="single"` and uses `onValueChange`
  (not `onSelectedChange` from v1). Select.Value renders the selected label.

  Props:
    options     — array of { value, label } pairs
    value       — currently selected value
    placeholder — text shown when no value is selected (defaults to i18n key)
    disabled    — disables the trigger
    label       — visible label rendered above the trigger
    id          — HTML id for the trigger (label association)
    onchange    — called with new value string on selection
-->
<script lang="ts">
  import { Select } from 'bits-ui';
  import { ChevronDown } from 'lucide-svelte';
  import * as m from '$lib/paraglide/messages';

  interface Option {
    value: string;
    label: string;
  }

  interface Props {
    options: Option[];
    value?: string;
    placeholder?: string;
    disabled?: boolean;
    label?: string;
    id?: string;
    /** Helper text shown below the trigger. Hidden when not provided. */
    helper?: string;
    onchange?: (value: string) => void;
  }

  const {
    options,
    value,
    placeholder,
    disabled = false,
    label,
    id,
    helper,
    onchange,
  }: Props = $props();

  // Stable per-instance fallback ID computed once at instantiation.
  // Used when the caller does not pass `id`, so label-for always points to a valid element.
  const _fallbackId = `dd-${Math.random().toString(36).slice(2, 8)}`;
  const uid = $derived(id ?? _fallbackId);

  /** Resolved placeholder: prop overrides the i18n default. */
  const resolvedPlaceholder = $derived(placeholder ?? m.dropdown_placeholder());

  /** Label to display in the trigger — falls back to placeholder when nothing is selected. */
  const displayLabel = $derived(
    value ? (options.find((o) => o.value === value)?.label ?? value) : resolvedPlaceholder,
  );

  const triggerTextClass = $derived(
    value ? 'text-(--color-text-primary)' : 'text-(--color-text-tertiary)',
  );

  const describedBy = $derived(helper ? `${uid}-helper` : undefined);
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

  <Select.Root type="single" {value} onValueChange={(v) => onchange?.(v)} {disabled}>
    <Select.Trigger
      id={uid}
      aria-describedby={describedBy}
      class="relative w-full h-[44px] px-3 pr-9 text-(--font-size-ui-base) bg-(--color-bg-input) border border-(--color-border) rounded-(--radius-sm) flex items-center text-left cursor-pointer disabled:cursor-not-allowed disabled:border-(--color-border-subtle) disabled:text-(--color-text-tertiary) focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-(--color-focus-ring) transition-[background-color,color,border-color] duration-(--duration-fast) ease-out {triggerTextClass}"
    >
      {#snippet child({ props })}
        <button {...props} type="button" class="w-full h-full flex items-center">
          <span class="block truncate flex-1">{displayLabel}</span>
          <ChevronDown
            size={14}
            class="absolute right-3 top-1/2 -translate-y-1/2 text-(--color-icon-default) pointer-events-none"
          />
        </button>
      {/snippet}
    </Select.Trigger>

    <Select.Portal to="body">
      <Select.Content
        class="z-(--z-popover) w-[var(--bits-select-anchor-width)] bg-(--color-bg-raised) border border-(--color-border-overlay) rounded-(--radius-md) shadow-(--shadow-raised) max-h-[240px] overflow-y-auto"
        sideOffset={4}
      >
        {#each options as option (option.value)}
          <Select.Item
            value={option.value}
            label={option.label}
            class="h-[44px] px-3 flex items-center text-(--font-size-ui-base) text-(--color-text-primary) cursor-pointer hover:bg-(--color-hover-bg) data-[highlighted]:bg-(--color-hover-bg) data-[selected]:bg-(--color-accent-subtle) data-[selected]:border-l-2 data-[selected]:border-(--color-accent) transition-[background-color,color,border-color] duration-(--duration-fast) ease-out"
          >
            {option.label}
          </Select.Item>
        {/each}
      </Select.Content>
    </Select.Portal>
  </Select.Root>

  {#if helper}
    <p id="{uid}-helper" class="text-(--font-size-ui-sm) text-(--color-text-secondary) mt-1">
      {helper}
    </p>
  {/if}
</div>
