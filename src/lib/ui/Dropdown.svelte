<!-- SPDX-License-Identifier: MPL-2.0 -->
<!--
  Dropdown — single-select listbox built on Bits UI Select (v2 API).

  Bits UI v2 Select.Root requires `type="single"` and uses `onValueChange`
  (not `onSelectedChange` from v1). Select.Value renders the selected label.

  Props:
    options     — array of { value, label } pairs
    value       — currently selected value
    placeholder — text shown when no value is selected
    disabled    — disables the trigger
    label       — visible label rendered above the trigger
    id          — HTML id for the trigger (label association)
    onchange    — called with new value string on selection
-->
<script lang="ts">
  import { Select } from 'bits-ui';
  import { ChevronDown } from 'lucide-svelte';

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
    onchange?: (value: string) => void;
  }

  const {
    options,
    value,
    placeholder = 'Select…',
    disabled = false,
    label,
    id,
    onchange,
  }: Props = $props();

  /** Label to display in the trigger — falls back to placeholder when nothing is selected. */
  const displayLabel = $derived(
    value ? (options.find((o) => o.value === value)?.label ?? value) : placeholder,
  );

  const triggerTextClass = $derived(value ? 'text-(--color-text-primary)' : 'text-(--color-text-tertiary)');
</script>

<div class="flex flex-col">
  {#if label}
    <label for={id} class="block text-[12px] font-medium text-(--color-text-secondary) mb-1">
      {label}
    </label>
  {/if}

  <Select.Root
    type="single"
    {value}
    onValueChange={(v) => onchange?.(v)}
    {disabled}
  >
    <Select.Trigger
      {id}
      class="relative w-full h-[44px] px-3 pr-9 text-[13px] bg-(--term-bg) border border-(--color-border) rounded-[2px] flex items-center text-left cursor-pointer disabled:cursor-not-allowed disabled:border-(--color-border-subtle) disabled:text-(--color-text-tertiary) focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-(--color-focus-ring) {triggerTextClass}"
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

    <Select.Content
      class="z-[30] w-[var(--bits-select-anchor-width)] bg-(--color-bg-raised) border border-(--color-border) rounded-[4px] shadow-(--shadow-raised) max-h-[240px] overflow-y-auto"
      sideOffset={4}
    >
      {#each options as option (option.value)}
        <Select.Item
          value={option.value}
          label={option.label}
          class="h-[44px] px-3 flex items-center text-[13px] text-(--color-text-primary) cursor-pointer hover:bg-(--color-hover-bg) data-[highlighted]:bg-(--color-hover-bg) data-[selected]:bg-(--color-accent-subtle) data-[selected]:border-l-2 data-[selected]:border-(--color-accent)"
        >
          {option.label}
        </Select.Item>
      {/each}
    </Select.Content>
  </Select.Root>
</div>
