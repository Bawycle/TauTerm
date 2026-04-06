<!-- SPDX-License-Identifier: MPL-2.0 -->
<!--
  SshReconnectionSeparator — visual separator injected into the scrollback at
  reconnection time (FS-SSH-042, UXD §7.19).

  Not interactive: no click, no selection, does not appear in clipboard copies.

  Props:
    timestampMs — Unix timestamp (ms) at the moment of reconnection.
                  When undefined or 0, the time portion is omitted.

  Visual: full-width horizontal rule with a left-aligned label overlaid.
  Rendered as a UI overlay by the frontend — not PTY content.
-->
<script lang="ts">
  import * as m from '$lib/paraglide/messages';

  interface Props {
    /** Unix timestamp in milliseconds. 0 or undefined means timestamp unavailable. */
    timestampMs?: number;
  }

  const { timestampMs }: Props = $props();

  /** Format HH:MM:SS from a Unix millisecond timestamp. */
  function formatTime(ms: number): string {
    const d = new Date(ms);
    const h = String(d.getHours()).padStart(2, '0');
    const min = String(d.getMinutes()).padStart(2, '0');
    const s = String(d.getSeconds()).padStart(2, '0');
    return `${h}:${min}:${s}`;
  }

  const label = $derived(
    timestampMs && timestampMs > 0
      ? m.ssh_reconnection_separator_label_at({ time: formatTime(timestampMs) })
      : m.ssh_reconnection_separator_label(),
  );
</script>

<!--
  aria-hidden: this is a UI decoration, not terminal content.
  Not focusable, not interactive per UXD §7.19.
-->
<div class="ssh-reconnection-separator" aria-hidden="true">
  <span class="ssh-reconnection-separator__label">{label}</span>
</div>

<style>
  .ssh-reconnection-separator {
    position: relative;
    display: flex;
    align-items: center;
    padding: var(--space-1, 4px) 0;
    /* The ::before pseudo-element is the full-width rule. */
    user-select: none;
    pointer-events: none;
    flex-shrink: 0;
  }

  /* Full-width horizontal rule rendered at the vertical center */
  .ssh-reconnection-separator::before {
    content: '';
    position: absolute;
    left: 0;
    right: 0;
    top: 50%;
    height: 1px;
    background-color: var(--color-ssh-connected);
  }

  .ssh-reconnection-separator__label {
    position: relative;
    /* Sits above the rule */
    z-index: 1;
    padding-right: var(--space-2, 8px);
    /* Use terminal background to "cut out" the rule behind the text */
    background-color: var(--term-bg);
    color: var(--color-text-secondary);
    font-family: var(--font-ui);
    font-size: var(--font-size-ui-xs);
    font-weight: var(--font-weight-normal, 400);
    white-space: nowrap;
  }
</style>
