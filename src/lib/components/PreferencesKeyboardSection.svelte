<!-- SPDX-License-Identifier: MPL-2.0 -->
<!--
  PreferencesKeyboardSection — keyboard shortcuts section of the preferences panel.
  Displays the list of configurable actions with their current bindings.

  Props:
    shortcuts        — effective shortcuts map (action id → binding string)
    shortcutActions  — ordered list of action descriptors
    onshortcutchange — called when user records a new binding
-->
<script lang="ts">
  import KeyboardShortcutRecorder from './KeyboardShortcutRecorder.svelte';
  import * as m from '$lib/paraglide/messages';

  interface ActionDescriptor {
    id: string;
    label: () => string;
  }

  interface Props {
    shortcuts: Record<string, string>;
    shortcutActions: ActionDescriptor[];
    onshortcutchange: (actionId: string, newShortcut: string) => void;
  }

  let { shortcuts, shortcutActions, onshortcutchange }: Props = $props();
</script>

<p
  class="text-(--font-size-ui-xs) font-semibold text-(--color-text-tertiary) uppercase mb-4"
  style="letter-spacing: var(--letter-spacing-label)"
>
  {m.preferences_section_keyboard()}
</p>
<div class="space-y-1">
  {#each shortcutActions as action (action.id)}
    <div class="flex items-center justify-between h-[44px]">
      <span class="text-(--font-size-ui-base) text-(--color-text-primary)">{action.label()}</span>
      <KeyboardShortcutRecorder
        value={shortcuts[action.id]}
        existingShortcuts={shortcuts}
        actionId={action.id}
        onchange={(s) => onshortcutchange(action.id, s)}
      />
    </div>
  {/each}
</div>
