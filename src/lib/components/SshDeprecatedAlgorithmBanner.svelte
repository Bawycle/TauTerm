<!-- SPDX-License-Identifier: MPL-2.0 -->
<!--
  SshDeprecatedAlgorithmBanner — non-blocking inline banner shown at the top of a pane
  when a deprecated SSH algorithm is detected (FS-SSH-014, UXD §7.21).

  Displaces terminal content downward — does not overlay.

  Props:
    algorithm  — deprecated algorithm name (e.g. "ssh-rsa" or "ssh-dss")
    ondismiss  — called when the user clicks the dismiss button

  Accessibility:
    - role="alert" and aria-live="assertive" for immediate announcement.
    - Dismiss button has explicit aria-label via Paraglide.
    - Icon is decorative (aria-hidden).
-->
<script lang="ts">
  import { AlertTriangle, X } from 'lucide-svelte';
  import * as m from '$lib/paraglide/messages';

  interface Props {
    algorithm: string;
    ondismiss?: () => void;
  }

  const { algorithm, ondismiss }: Props = $props();
</script>

<div class="ssh-deprecated-banner" role="alert" aria-live="assertive">
  <span class="ssh-deprecated-banner__icon" aria-hidden="true">
    <AlertTriangle size={16} />
  </span>
  <span class="ssh-deprecated-banner__text">
    {m.ssh_banner_deprecated_algorithm({ algorithm })}
  </span>
  <button
    class="ssh-deprecated-banner__dismiss"
    type="button"
    aria-label={m.ssh_banner_deprecated_algorithm_dismiss()}
    onclick={ondismiss}
  >
    <X size={14} aria-hidden="true" />
  </button>
</div>

<style>
  .ssh-deprecated-banner {
    display: flex;
    flex-direction: row;
    align-items: center;
    min-height: var(--size-target-min, 44px);
    padding: var(--space-2, 8px) var(--space-3, 12px);
    background-color: var(--color-warning-bg);
    border-bottom: 1px solid var(--color-warning);
    font-family: var(--font-ui);
    font-size: var(--font-size-ui-base);
    color: var(--color-warning-text);
    gap: var(--space-2, 8px);
    /* Ensure the banner sits above the terminal grid but does not overlay. */
    position: relative;
    z-index: 1;
    /* Prevent line breaks mid-layout. */
    flex-shrink: 0;
  }

  .ssh-deprecated-banner__icon {
    flex-shrink: 0;
    color: var(--color-warning);
    display: flex;
    align-items: center;
  }

  .ssh-deprecated-banner__text {
    flex: 1;
    min-width: 0;
    word-break: break-word;
  }

  .ssh-deprecated-banner__dismiss {
    flex-shrink: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    /* 44px hit area (WCAG 2.1 SC 2.5.5) */
    width: var(--size-target-min, 44px);
    height: var(--size-target-min, 44px);
    background: transparent;
    border: none;
    border-radius: var(--radius-sm);
    cursor: pointer;
    color: var(--color-warning-text);
    padding: 0;
    transition: background-color 100ms ease;
  }

  .ssh-deprecated-banner__dismiss:hover {
    background-color: color-mix(in srgb, var(--color-warning) 20%, transparent);
    color: var(--color-neutral-100);
  }

  .ssh-deprecated-banner__dismiss:focus-visible {
    outline: 2px solid var(--color-focus-ring);
    outline-offset: 2px;
  }
</style>
