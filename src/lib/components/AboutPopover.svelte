<!-- SPDX-License-Identifier: MPL-2.0 -->
<!--
  AboutPopover — version label in the StatusBar that opens a non-modal About popover.

  Structure:
    - Popover.Trigger: styled version label button (44×44 hit area via ::before)
    - Popover.Content: 320px panel with app info, license, third-party notices

  Third-party licenses are generated at build time by scripts/generate-licenses.sh
  into src/lib/generated/third-party-licenses.json. If unavailable, an empty list
  is shown with a fallback message.

  Focus management:
    - Bits UI Popover auto-focuses the first focusable element in Content (Close button,
      which is first in DOM order via absolute positioning).
    - On close (Escape / click outside / Close button), onCloseAutoFocus fires.
    - e.preventDefault() blocks Bits UI's default focus-return to the trigger.
    - Delegate focus restoration to the parent — same pattern as PreferencesPanel.
-->
<script lang="ts">
  import { Popover } from 'bits-ui';
  import { X, ExternalLink, Copy, Check } from 'lucide-svelte';
  import { openUrl } from '@tauri-apps/plugin-opener';
  import * as m from '$lib/paraglide/messages';

  interface Props {
    version: string;
    /** Called at the exact moment Bits UI releases focus (onCloseAutoFocus).
     *  e.preventDefault() is already done here;
     *  use this to schedule focus restoration (same as PreferencesPanel pattern). */
    onCloseAutoFocus?: () => void;
  }

  const { version, onCloseAutoFocus: onCloseAutoFocusProp }: Props = $props();

  const REPO_URL = 'https://github.com/Bawycle/TauTerm';
  const LICENSE_URL = 'https://www.mozilla.org/en-US/MPL/2.0/';
  const NOTICES_URL = `${REPO_URL}/blob/master/THIRD-PARTY-NOTICES.md`;

  let copied = $state(false);

  function handleSourceCodeClick(e: MouseEvent) {
    e.preventDefault();
    openUrl(REPO_URL);
  }

  function handleLicenseClick(e: MouseEvent) {
    e.preventDefault();
    openUrl(LICENSE_URL);
  }

  function handleNoticesClick(e: MouseEvent) {
    e.preventDefault();
    openUrl(NOTICES_URL);
  }

  async function handleCopyVersion() {
    await navigator.clipboard.writeText(version);
    copied = true;
    setTimeout(() => {
      copied = false;
    }, 1500);
  }
</script>

<Popover.Root>
  <Popover.Trigger class="about-trigger" aria-label={m.about_app_version_aria({ version })}>
    v{version}
  </Popover.Trigger>

  <Popover.Content
    side="top"
    align="end"
    sideOffset={8}
    class="about-content"
    onCloseAutoFocus={(e) => {
      // Prevent Bits UI from returning focus to the trigger (version label).
      e.preventDefault();
      // Delegate focus restoration to the parent — same pattern as
      // PreferencesPanel's onCloseAutoFocus in TerminalView.
      onCloseAutoFocusProp?.();
    }}
  >
    <!-- Absolute-positioned close button: DOM-first for initial focus -->
    <Popover.Close class="about-content__close-btn" aria-label={m.about_close()}>
      <X size={16} aria-hidden="true" />
    </Popover.Close>

    <!-- Block 1: App identity -->
    <div class="about-content__header">
      <span class="about-content__app-name">TauTerm</span>
    </div>

    <!-- Identity row: version + copy + · + license link -->
    <div class="about-content__identity-row">
      <span class="about-content__version">v{version}</span>
      <button
        type="button"
        class="about-content__copy-btn"
        aria-label={m.about_copy_version()}
        onclick={handleCopyVersion}
      >
        {#if copied}
          <Check size={12} aria-hidden="true" />
        {:else}
          <Copy size={12} aria-hidden="true" />
        {/if}
      </button>
      <span class="about-content__dot">·</span>
      <a href={LICENSE_URL} class="about-content__license-link" onclick={handleLicenseClick}>
        MPL-2.0
      </a>
    </div>

    <!-- Source code link -->
    <a href={REPO_URL} class="about-content__source-link" onclick={handleSourceCodeClick}>
      {m.about_source_code()}
      <ExternalLink size={12} aria-hidden="true" />
    </a>

    <hr class="about-content__separator" />

    <!-- Block 2: Third-party notices — link to repo file -->
    <a href={NOTICES_URL} class="about-content__source-link" onclick={handleNoticesClick}>
      {m.about_third_party_notices()}
      <ExternalLink size={12} aria-hidden="true" />
    </a>
  </Popover.Content>
</Popover.Root>

<style>
  /* -------------------------------------------------------------------------
     Trigger: version label button
     Visible area is small (status bar height); ::before extends hit area to
     the WCAG 2.5.5 minimum 44×44px without affecting layout.
  ------------------------------------------------------------------------- */
  :global(.about-trigger) {
    position: relative;
    display: inline-flex;
    align-items: center;
    background: transparent;
    border: none;
    padding: 0 var(--space-1);
    font-family: var(--font-mono-ui);
    font-size: var(--font-size-ui-xs);
    color: var(--color-text-muted);
    cursor: pointer;
    line-height: 1;
    flex-shrink: 0;
    transition: color var(--duration-instant);
  }

  :global(.about-trigger:hover) {
    color: var(--color-text-tertiary);
  }

  :global(.about-trigger:focus-visible) {
    outline: 2px solid var(--color-focus-ring);
    outline-offset: -1px;
    border-radius: var(--radius-sm);
  }

  /* Extend interactive target to 44×44px: (44 - 24) / 2 = 10px on each axis */
  :global(.about-trigger::before) {
    content: '';
    position: absolute;
    inset: -10px;
    min-width: var(--size-target-min);
    min-height: var(--size-target-min);
  }

  /* -------------------------------------------------------------------------
     Popover content panel
  ------------------------------------------------------------------------- */
  :global(.about-content) {
    position: relative;
    width: 320px;
    max-height: 360px;
    overflow-y: auto;
    background-color: var(--color-bg-raised);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-md);
    padding: var(--space-4);
    box-shadow: var(--shadow-overlay);
    z-index: var(--z-popover);
  }

  /* -------------------------------------------------------------------------
     Close button — absolute top-right
  ------------------------------------------------------------------------- */
  :global(.about-content__close-btn) {
    position: absolute;
    top: var(--space-2);
    right: var(--space-2);
    display: flex;
    align-items: center;
    justify-content: center;
    width: 24px;
    height: 24px;
    border: none;
    border-radius: var(--radius-sm);
    background: transparent;
    color: var(--color-text-tertiary);
    cursor: pointer;
    padding: 0;
    transition:
      color var(--duration-instant),
      background-color var(--duration-instant);
  }

  :global(.about-content__close-btn:hover) {
    color: var(--color-text-primary);
    background-color: var(--color-hover-bg);
  }

  :global(.about-content__close-btn:focus-visible) {
    outline: 2px solid var(--color-focus-ring);
    outline-offset: -1px;
  }

  /* -------------------------------------------------------------------------
     Header block
  ------------------------------------------------------------------------- */
  :global(.about-content__header) {
    /* Right padding prevents app name from overlapping the absolute close button */
    padding-right: var(--space-6);
  }

  :global(.about-content__app-name) {
    font-size: var(--font-size-ui-md);
    font-weight: var(--font-weight-semibold);
    color: var(--color-text-primary);
  }

  /* -------------------------------------------------------------------------
     Identity row: version + copy button + separator dot + license link
  ------------------------------------------------------------------------- */
  :global(.about-content__identity-row) {
    display: flex;
    align-items: center;
    gap: var(--space-1);
    margin-top: var(--space-1);
  }

  :global(.about-content__version) {
    font-family: var(--font-mono-ui);
    font-size: var(--font-size-ui-sm);
    color: var(--color-text-primary);
  }

  :global(.about-content__copy-btn) {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 20px;
    height: 20px;
    border: none;
    background: transparent;
    padding: 0;
    cursor: pointer;
    color: var(--color-text-tertiary);
    border-radius: var(--radius-sm);
    transition:
      color var(--duration-instant),
      background-color var(--duration-instant);
  }

  :global(.about-content__copy-btn:hover) {
    color: var(--color-text-secondary);
  }

  :global(.about-content__copy-btn:focus-visible) {
    outline: 2px solid var(--color-focus-ring);
    outline-offset: -1px;
  }

  :global(.about-content__dot) {
    color: var(--color-text-muted);
    font-size: var(--font-size-ui-sm);
  }

  :global(.about-content__license-link) {
    font-size: var(--font-size-ui-sm);
    color: var(--color-text-secondary);
    text-decoration: none;
    cursor: pointer;
    transition: color var(--duration-instant);
  }

  :global(.about-content__license-link:hover) {
    color: var(--color-accent);
    text-decoration: underline;
  }

  :global(.about-content__license-link:focus-visible) {
    outline: 2px solid var(--color-focus-ring);
    outline-offset: 1px;
    border-radius: var(--radius-sm);
  }

  /* -------------------------------------------------------------------------
     Source code link
  ------------------------------------------------------------------------- */
  :global(.about-content__source-link) {
    display: inline-flex;
    align-items: center;
    gap: var(--space-1);
    margin-top: var(--space-1);
    font-size: var(--font-size-ui-xs);
    color: var(--color-accent);
    text-decoration: none;
    cursor: pointer;
  }

  :global(.about-content__source-link:hover) {
    text-decoration: underline;
  }

  :global(.about-content__source-link:focus-visible) {
    outline: 2px solid var(--color-focus-ring);
    outline-offset: 1px;
    border-radius: var(--radius-sm);
  }

  /* -------------------------------------------------------------------------
     Separator
  ------------------------------------------------------------------------- */
  :global(.about-content__separator) {
    border: none;
    border-top: 1px solid var(--color-border);
    margin: var(--space-3) 0;
  }
</style>
