<!-- SPDX-License-Identifier: MPL-2.0 -->
<!--
  PreferencesConnectionsSection — SSH connections section of the preferences panel.
  Placeholder: known-hosts import button, empty state message.
-->
<script lang="ts">
  import Button from '$lib/ui/Button.svelte';
  import * as m from '$lib/paraglide/messages';
</script>

<p class="text-(--font-size-ui-xs) font-semibold text-(--color-text-tertiary) uppercase tracking-wider mb-4">
  {m.preferences_section_connections()}
</p>
<p class="text-(--font-size-ui-base) text-(--color-text-secondary) mb-4">
  {m.connection_empty_state()}
</p>
<!-- Known-hosts import -->
<Button variant="secondary" onclick={() => {}}>
  {m.action_import_known_hosts()}
</Button>
