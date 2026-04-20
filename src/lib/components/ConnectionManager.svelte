<!-- SPDX-License-Identifier: MPL-2.0 -->
<!--
  ConnectionManager — SSH connection list with CRUD and open actions (UXD §7.7).

  Can be rendered as a standalone right-side slide-in panel (standalone=true)
  or inline inside PreferencesPanel Connections section (standalone=false).

  The connection data comes from get_connections / Preferences.connections IPC.
  All mutations are emitted as events — parent (TerminalView or PreferencesPanel)
  calls the appropriate IPC commands.

  Props:
    standalone    — if true, renders with slide-in panel chrome
    connections   — SshConnectionConfig[] from IPC
    onsave        — (config) → emit save-connection IPC
    ondelete      — (connectionId) → emit delete-connection IPC
    onopen        — ({ connectionId, target: 'tab' | 'pane' }) → open SSH session
    onclose       — called when standalone panel should close

  Security:
    - all string fields (host, label, username) rendered via text interpolation — no {@html}
    - password field is type="password", cleared after IPC call
    - no clipboard read on render (SEC-UI-005)
-->
<script lang="ts">
  import {
    X,
    Plus,
    Check,
    Server,
    ExternalLink,
    SplitSquareVertical,
    Pencil,
    Copy,
    Trash2,
    ChevronDown,
  } from 'lucide-svelte';
  import TextInput from '$lib/ui/TextInput.svelte';
  import Toggle from '$lib/ui/Toggle.svelte';
  import Button from '$lib/ui/Button.svelte';
  import * as m from '$lib/paraglide/messages';
  import type { SshConnectionConfig } from '$lib/ipc';

  // ---------------------------------------------------------------------------
  // Props
  // ---------------------------------------------------------------------------

  interface OpenTarget {
    connectionId: string;
    target: 'tab' | 'pane';
  }

  interface Props {
    standalone?: boolean;
    connections?: SshConnectionConfig[];
    /**
     * Called when the user saves a connection config.
     * `password` is present when auth method is "password" and a password was
     * entered. The parent is responsible for storing it via SecretService
     * (SEC-CRED-004) using the real ConnectionId returned by the backend —
     * never using the placeholder empty string sent for new connections.
     */
    onsave?: (config: SshConnectionConfig, password?: string) => void;
    ondelete?: (connectionId: string) => void;
    onopen?: (args: OpenTarget) => void;
    onclose?: () => void;
  }

  const {
    standalone = false,
    connections = [],
    onsave,
    ondelete,
    onopen,
    onclose,
  }: Props = $props();

  // ---------------------------------------------------------------------------
  // Edit form state
  // ---------------------------------------------------------------------------

  type AuthMethod = 'identity' | 'password';

  let showForm = $state(false);
  let editingId = $state<string | null>(null);

  let formLabel = $state('');
  let formGroup = $state('');
  let formHost = $state('');
  let formPort = $state('22');
  let formUsername = $state('');
  let formAuthMethod = $state<AuthMethod>('identity');
  let formIdentityFile = $state('');
  /** Password is stored only transiently in component state — cleared after save. */
  let formPassword = $state('');
  let formAllowOsc52 = $state(false);

  // ---------------------------------------------------------------------------
  // Collapsed groups state
  // ---------------------------------------------------------------------------

  let collapsedGroups = $state<Set<string>>(new Set());

  function toggleGroup(group: string) {
    const next = new Set(collapsedGroups);
    if (next.has(group)) next.delete(group);
    else next.add(group);
    collapsedGroups = next;
  }

  // ---------------------------------------------------------------------------
  // Grouped connections
  // ---------------------------------------------------------------------------

  const groupedConnections = $derived.by(() => {
    const groups = new Map<string, SshConnectionConfig[]>();
    for (const conn of connections) {
      const g =
        (conn as SshConnectionConfig & { group?: string }).group ?? m.connection_group_ungrouped();
      if (!groups.has(g)) groups.set(g, []);
      groups.get(g)!.push(conn);
    }
    return groups;
  });

  // ---------------------------------------------------------------------------
  // Form actions
  // ---------------------------------------------------------------------------

  function openNewForm() {
    editingId = null;
    formLabel = '';
    formGroup = '';
    formHost = '';
    formPort = '22';
    formUsername = '';
    formAuthMethod = 'identity';
    formIdentityFile = '';
    formPassword = '';
    formAllowOsc52 = false;
    showForm = true;
  }

  function openEditForm(conn: SshConnectionConfig) {
    editingId = conn.id;
    formLabel = conn.label ?? '';
    formGroup = (conn as SshConnectionConfig & { group?: string }).group ?? '';
    formHost = conn.host;
    formPort = String(conn.port);
    formUsername = conn.username;
    formAuthMethod = conn.identityFile ? 'identity' : 'password';
    formIdentityFile = conn.identityFile ?? '';
    formPassword = '';
    formAllowOsc52 = conn.allowOsc52Write ?? false;
    showForm = true;
  }

  function handleSave() {
    const rawPort = parseInt(formPort, 10);
    // Clamp port to valid TCP range [1, 65535] (Bug 3 fix)
    const port = Math.max(1, Math.min(65535, isNaN(rawPort) ? 22 : rawPort));
    const config: SshConnectionConfig = {
      id: editingId ?? '',
      label: formLabel,
      host: formHost,
      port,
      username: formUsername,
      identityFile: formAuthMethod === 'identity' ? formIdentityFile : null,
      allowOsc52Write: formAllowOsc52,
      keepaliveIntervalSecs: null,
      keepaliveMaxFailures: null,
    };
    // Pass password to parent so it can store it via SecretService (SEC-CRED-004)
    // after the backend returns the real ConnectionId. The parent must NOT store
    // it under config.id (which may be '' for new connections).
    const password = formAuthMethod === 'password' && formPassword ? formPassword : undefined;
    onsave?.(config, password);
    // Clear password from component state immediately (SEC-UI-002)
    formPassword = '';
    showForm = false;
  }

  function handleCancel() {
    formPassword = '';
    showForm = false;
  }

  function handleDelete(connectionId: string) {
    ondelete?.(connectionId);
  }

  function handleDuplicate(conn: SshConnectionConfig) {
    const duped: SshConnectionConfig = {
      ...conn,
      id: '',
      label: conn.label ? m.connection_duplicate_label_suffix({ label: conn.label }) : '',
    };
    onsave?.(duped);
  }
</script>

<div
  class="connection-manager"
  class:connection-manager--standalone={standalone}
  role="complementary"
  aria-label={m.connection_manager_title()}
>
  {#if standalone}
    <!-- Slide-in panel header -->
    <div class="connection-manager__header">
      <h2 class="text-(--font-size-ui-lg) font-semibold text-(--color-text-primary)">
        {m.connection_manager_title()}
      </h2>
      <button
        class="flex items-center justify-center w-[44px] h-[44px] text-(--color-text-secondary) hover:text-(--color-text-primary) hover:bg-(--color-hover-bg) rounded-(--radius-sm) transition-[background-color,color,border-color] duration-(--duration-fast) ease-out"
        onclick={onclose}
        aria-label={m.action_close()}
      >
        <X size={16} aria-hidden="true" />
      </button>
    </div>
  {/if}

  <div class="connection-manager__body">
    {#if !showForm}
      <!-- Connection list view -->
      <div class="connection-manager__actions">
        <Button variant="primary" onclick={openNewForm}>
          <Plus size={16} aria-hidden="true" />
          {m.connection_new()}
        </Button>
      </div>

      {#if connections.length === 0}
        <p class="text-(--font-size-ui-base) text-(--color-text-secondary) px-1 mt-4">
          {m.connection_empty_state()}
        </p>
      {:else}
        <div class="connection-manager__list" role="list">
          {#each [...groupedConnections] as [group, items] (group)}
            <!-- Group heading -->
            <button
              class="connection-manager__group-heading"
              onclick={() => toggleGroup(group)}
              aria-expanded={!collapsedGroups.has(group)}
            >
              <span>{group}</span>
              <ChevronDown
                size={14}
                aria-hidden="true"
                class="transition-transform duration-150"
                style={collapsedGroups.has(group) ? 'transform: rotate(-90deg)' : ''}
              />
            </button>

            {#if !collapsedGroups.has(group)}
              {#each items as conn (conn.id)}
                <div class="connection-manager__item" role="listitem">
                  <!-- Left: icon + labels -->
                  <div class="connection-manager__item-info">
                    <span class="connection-manager__item-icon"
                      ><Server size={16} aria-hidden="true" /></span
                    >
                    <div>
                      <p class="connection-manager__item-primary">
                        {conn.label || `${conn.host}:${conn.port}`}
                      </p>
                      <p class="connection-manager__item-secondary">
                        {conn.username}@{conn.host}
                      </p>
                    </div>
                  </div>

                  <!-- Right: action buttons (visible on hover via CSS, always accessible) -->
                  <div
                    class="connection-manager__item-actions"
                    role="group"
                    aria-label={m.connection_actions_aria_label()}
                  >
                    <button
                      class="connection-manager__action-btn"
                      onclick={() => onopen?.({ connectionId: conn.id, target: 'tab' })}
                      aria-label={m.action_open_in_new_tab()}
                      title={m.action_open_in_new_tab()}
                    >
                      <ExternalLink size={14} aria-hidden="true" />
                    </button>
                    <button
                      class="connection-manager__action-btn"
                      onclick={() => onopen?.({ connectionId: conn.id, target: 'pane' })}
                      aria-label={m.action_open_in_pane()}
                      title={m.action_open_in_pane()}
                    >
                      <SplitSquareVertical size={14} aria-hidden="true" />
                    </button>
                    <button
                      class="connection-manager__action-btn"
                      onclick={() => openEditForm(conn)}
                      aria-label={m.connection_edit()}
                      title={m.connection_edit()}
                    >
                      <Pencil size={14} aria-hidden="true" />
                    </button>
                    <button
                      class="connection-manager__action-btn"
                      onclick={() => handleDuplicate(conn)}
                      aria-label={m.connection_duplicate()}
                      title={m.connection_duplicate()}
                    >
                      <Copy size={14} aria-hidden="true" />
                    </button>
                    <button
                      class="connection-manager__action-btn connection-manager__action-btn--delete"
                      onclick={() => handleDelete(conn.id)}
                      aria-label={m.connection_delete()}
                      title={m.connection_delete()}
                    >
                      <Trash2 size={14} aria-hidden="true" />
                    </button>
                  </div>
                </div>
              {/each}
            {/if}
          {/each}
        </div>
      {/if}
    {:else}
      <!-- Edit form view -->
      <div
        class="connection-manager__form"
        role="form"
        aria-label={editingId ? m.connection_edit() : m.connection_new()}
      >
        <h3 class="connection-manager__form-title">
          {editingId ? m.connection_edit_title() : m.connection_new()}
        </h3>
        <div class="space-y-3">
          <TextInput
            id="cm-label"
            label={m.connection_field_label()}
            value={formLabel}
            oninput={(v) => {
              formLabel = v;
            }}
          />
          <TextInput
            id="cm-group"
            label={m.connection_field_group()}
            value={formGroup}
            oninput={(v) => {
              formGroup = v;
            }}
          />
          <TextInput
            id="cm-host"
            label={m.connection_field_host()}
            value={formHost}
            oninput={(v) => {
              formHost = v;
            }}
          />
          <TextInput
            id="cm-port"
            label={m.connection_field_port()}
            type="number"
            value={formPort}
            oninput={(v) => {
              formPort = v;
            }}
          />
          <TextInput
            id="cm-username"
            label={m.connection_field_user()}
            value={formUsername}
            oninput={(v) => {
              formUsername = v;
            }}
          />

          <!-- Auth method -->
          <div>
            <p class="text-(--font-size-ui-sm) font-medium text-(--color-text-secondary) mb-2">
              {m.connection_field_auth_method()}
            </p>
            <div class="flex gap-4">
              <label
                class="flex items-center gap-2 text-(--font-size-ui-base) text-(--color-text-primary) cursor-pointer"
              >
                <input
                  type="radio"
                  name="cm-auth-method"
                  value="identity"
                  checked={formAuthMethod === 'identity'}
                  onchange={() => {
                    formAuthMethod = 'identity';
                  }}
                />
                {m.connection_auth_identity_file()}
              </label>
              <label
                class="flex items-center gap-2 text-(--font-size-ui-base) text-(--color-text-primary) cursor-pointer"
              >
                <input
                  type="radio"
                  name="cm-auth-method"
                  value="password"
                  checked={formAuthMethod === 'password'}
                  onchange={() => {
                    formAuthMethod = 'password';
                  }}
                />
                {m.connection_auth_password()}
              </label>
            </div>
          </div>

          {#if formAuthMethod === 'identity'}
            <TextInput
              id="cm-identity-file"
              label={m.connection_field_identity()}
              value={formIdentityFile}
              oninput={(v) => {
                formIdentityFile = v;
              }}
            />
          {:else}
            <!-- Password field — type="password" ensures masking (SEC-UI-002) -->
            <TextInput
              id="cm-password"
              label={m.connection_field_password()}
              type="password"
              value={formPassword}
              oninput={(v) => {
                formPassword = v;
              }}
            />
          {/if}

          <Toggle
            label={m.connection_osc52_label()}
            checked={formAllowOsc52}
            onchange={(v) => {
              formAllowOsc52 = v;
            }}
          />
        </div>

        <div class="flex gap-2 mt-6">
          <Button variant="primary" onclick={handleSave}>
            {#if editingId}
              <Check size={16} aria-hidden="true" />
              {m.connection_action_save_changes()}
            {:else}
              <Plus size={16} aria-hidden="true" />
              {m.connection_action_add()}
            {/if}
          </Button>
          <Button variant="ghost" onclick={handleCancel}>
            {m.action_cancel()}
          </Button>
        </div>
      </div>
    {/if}
  </div>
</div>

<style>
  .connection-manager--standalone {
    position: fixed;
    right: 0;
    top: var(--size-tab-height);
    bottom: var(--size-status-bar-height, 28px);
    width: 400px;
    background-color: var(--color-bg-raised);
    border-left: 1px solid var(--color-border-overlay);
    box-shadow: var(--shadow-overlay);
    z-index: var(--z-overlay, 40);
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }

  .connection-manager__header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: var(--space-4, 16px);
    border-bottom: 1px solid var(--color-border);
    flex-shrink: 0;
  }

  .connection-manager__body {
    flex: 1;
    overflow-y: auto;
    padding: 0 var(--space-4);
    display: flex;
    flex-direction: column;
  }

  .connection-manager__actions {
    padding: var(--space-3, 12px) 0 var(--space-2, 8px);
  }

  .connection-manager__list {
    flex: 1;
    overflow-y: auto;
  }

  .connection-manager__group-heading {
    display: flex;
    align-items: center;
    justify-content: space-between;
    width: 100%;
    padding: var(--space-2, 8px) 0;
    color: var(--color-text-muted);
    font-size: var(--font-size-ui-xs);
    font-weight: var(--font-weight-semibold);
    letter-spacing: var(--letter-spacing-label);
    text-transform: uppercase;
    text-align: left;
    cursor: pointer;
    background: transparent;
    border: none;
    outline: none;
  }

  .connection-manager__group-heading:focus-visible {
    outline: 2px solid var(--color-focus-ring);
    outline-offset: 2px;
    border-radius: var(--radius-sm);
  }

  .connection-manager__item {
    display: flex;
    align-items: center;
    justify-content: space-between;
    min-height: 44px;
    padding: var(--space-2, 8px) 0;
    border-bottom: 1px solid var(--color-border-subtle);
  }

  .connection-manager__item:hover .connection-manager__item-actions {
    opacity: 1;
  }

  .connection-manager__item-info {
    display: flex;
    align-items: center;
    gap: var(--space-2, 8px);
    flex: 1;
    min-width: 0;
  }

  .connection-manager__item-icon {
    color: var(--color-icon-default);
    flex-shrink: 0;
  }

  .connection-manager__item-primary {
    font-size: var(--font-size-ui-base);
    color: var(--color-text-primary);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .connection-manager__item-secondary {
    font-size: var(--font-size-ui-sm);
    color: var(--color-text-secondary);
  }

  .connection-manager__item-actions {
    display: flex;
    align-items: center;
    gap: 2px;
    flex-shrink: 0;
    /* Show on focus/keyboard even when not hovered */
  }

  .connection-manager__action-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 44px;
    height: 44px;
    color: var(--color-icon-default);
    background: transparent;
    border: none;
    border-radius: var(--radius-sm);
    cursor: pointer;
    outline: none;
    transition:
      background-color var(--duration-fast) var(--ease-out),
      color var(--duration-fast) var(--ease-out);
  }

  .connection-manager__action-btn:hover {
    color: var(--color-icon-active);
    background-color: var(--color-hover-bg);
  }

  .connection-manager__action-btn:focus-visible {
    outline: 2px solid var(--color-focus-ring);
    outline-offset: 2px;
  }

  .connection-manager__action-btn--delete {
    color: var(--color-error);
  }

  .connection-manager__form {
    flex: 1;
    overflow-y: auto;
    padding: var(--space-4, 16px) 0;
  }

  .connection-manager__form-title {
    font-size: var(--font-size-ui-base);
    font-weight: var(--font-weight-semibold);
    color: var(--color-text-primary);
    margin-bottom: var(--space-3);
  }
</style>
