<!-- SPDX-License-Identifier: MPL-2.0 -->
<!--
  Button — generic action button with 4 variants.

  Variants:
    primary     — filled accent, primary action
    secondary   — outlined accent, secondary action
    ghost       — no background, tertiary action
    destructive — filled error color, irreversible actions

  All variants meet the 44px minimum touch target height (WCAG 2.5.5).
-->
<script lang="ts">
  import type { Snippet } from 'svelte';
  import type { HTMLButtonAttributes } from 'svelte/elements';

  interface Props extends Omit<HTMLButtonAttributes, 'children'> {
    variant?: 'primary' | 'secondary' | 'ghost' | 'destructive';
    disabled?: boolean;
    type?: 'button' | 'submit' | 'reset';
    onclick?: (e: MouseEvent) => void;
    /** Bindable ref to the underlying <button> element (e.g. for programmatic focus). */
    buttonRef?: HTMLButtonElement;
    children: Snippet;
  }

  let {
    variant = 'primary',
    disabled = false,
    type = 'button',
    onclick,
    buttonRef = $bindable<HTMLButtonElement | undefined>(undefined),
    children,
    // eslint-disable-next-line @typescript-eslint/no-unused-vars
    class: _class,
    ...restProps
  }: Props = $props();

  const baseClasses =
    'inline-flex items-center justify-center gap-1 min-h-[44px] px-4 text-[13px] font-medium leading-none whitespace-nowrap rounded-[2px] cursor-pointer disabled:cursor-not-allowed focus-visible:outline-2 focus-visible:outline-offset-2';

  const variantClasses: Record<NonNullable<Props['variant']>, string> = {
    primary:
      'bg-(--color-accent) text-(--color-text-inverted) hover:bg-(--color-blue-500) active:bg-(--color-blue-600) disabled:bg-(--color-neutral-700) disabled:text-(--color-text-tertiary)',
    secondary:
      'bg-transparent text-(--color-accent-text) border border-(--color-accent) hover:bg-(--color-accent-subtle) active:bg-(--color-blue-700) disabled:border-(--color-neutral-700) disabled:text-(--color-text-tertiary)',
    ghost:
      'bg-transparent text-(--color-text-primary) hover:bg-(--color-hover-bg) active:bg-(--color-active-bg) disabled:text-(--color-text-tertiary)',
    destructive:
      'bg-(--color-error) text-(--color-neutral-100) hover:bg-(--color-red-500) active:bg-(--color-red-700) disabled:bg-(--color-neutral-700) disabled:text-(--color-text-tertiary)',
  };

  const classes = $derived(`${baseClasses} ${variantClasses[variant ?? 'primary']}`);
</script>

<button bind:this={buttonRef} {type} {disabled} class={classes} {onclick} {...restProps}>
  {@render children()}
</button>
