<!-- SPDX-License-Identifier: MPL-2.0 -->
<!--
  TabBar — horizontal tab strip at the top of the window.
  Displays all open tabs, the active indicator, SSH badges,
  notification dots, and the new-tab button.

  IPC commands:
    - invoke('create_tab') on new-tab click
    - invoke('close_tab') on tab close click
    - invoke('set_active_tab') on tab click
-->
<script lang="ts">
  // i18n: import * as m from '$lib/paraglide/messages';
  // (available once Paraglide compilation runs)
</script>

<div class="tab-bar" role="tablist" aria-label="Terminal tabs">
  <!-- Tab items rendered here from session state -->
  <div class="tab-bar__tabs">
    <!-- TODO: render TabItem components from session state -->
  </div>

  <button class="tab-bar__new-tab" type="button" aria-label="New tab" title="New tab">
    <!-- Plus icon (Lucide) rendered here -->
    <span class="tab-bar__new-tab-icon" aria-hidden="true">+</span>
  </button>
</div>

<style>
  .tab-bar {
    display: flex;
    align-items: center;
    height: var(--size-tab-height);
    min-height: var(--size-tab-height);
    background-color: var(--color-tab-bg);
    border-bottom: 1px solid var(--color-border);
    overflow: hidden;
    flex-shrink: 0;
  }

  .tab-bar__tabs {
    display: flex;
    align-items: stretch;
    flex: 1;
    height: 100%;
    overflow: hidden;
  }

  .tab-bar__new-tab {
    display: flex;
    align-items: center;
    justify-content: center;
    width: var(--size-target-min);
    height: 100%;
    flex-shrink: 0;
    border: none;
    background: transparent;
    color: var(--color-tab-new-fg);
    cursor: pointer;
    font-size: var(--font-size-ui-md);
    transition: color var(--duration-instant);
  }

  .tab-bar__new-tab:hover {
    color: var(--color-tab-new-hover-fg);
    background-color: var(--color-hover-bg);
  }

  .tab-bar__new-tab-icon {
    width: var(--size-icon-sm);
    height: var(--size-icon-sm);
  }
</style>
