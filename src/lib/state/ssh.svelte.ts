// SPDX-License-Identifier: MPL-2.0

/**
 * Reactive SSH state — per-pane SSH lifecycle + auth prompt state.
 *
 * Tracks:
 *   - SSH lifecycle state per pane (connecting / authenticating / connected /
 *     disconnected / closed) — updated from ssh-state-changed events
 *   - Active host key prompt (TOFU dialog)
 *   - Active credential prompt (password dialog)
 *   - Bracketed paste mode per pane — needed for Ctrl+Shift+V in TerminalView
 */

import type {
  PaneId,
  SshLifecycleState,
  SshStateChangedEvent,
  HostKeyPromptEvent,
  CredentialPromptEvent,
} from '$lib/ipc/types';

// ---------------------------------------------------------------------------
// Reactive state — module-level singleton
// ---------------------------------------------------------------------------

/**
 * Svelte 5 module-level state container.
 *
 * `export let x = $state(v)` is invalid when `x` is later reassigned — Svelte
 * forbids re-exporting a reassigned primitive state from a module. Wrapping all
 * mutable scalars inside a single `$state` object avoids the restriction: the
 * object reference never changes, only its properties are mutated.
 */
const _ssh = $state<{
  hostKeyPrompt: HostKeyPromptEvent | null;
  credentialPrompt: CredentialPromptEvent | null;
}>({
  hostKeyPrompt: null,
  credentialPrompt: null,
});

/** SSH lifecycle state keyed by PaneId. */
export const sshStates = $state<Map<PaneId, SshLifecycleState>>(new Map());

/**
 * Active TOFU / key-change host key dialog, or null when none pending.
 * Read-only export — mutate via setHostKeyPrompt / clearHostKeyPrompt.
 */
export const hostKeyPrompt = {
  get value(): HostKeyPromptEvent | null {
    return _ssh.hostKeyPrompt;
  },
};

/**
 * Active credential prompt dialog, or null when none pending.
 * Read-only export — mutate via setCredentialPrompt / clearCredentialPrompt.
 */
export const credentialPrompt = {
  get value(): CredentialPromptEvent | null {
    return _ssh.credentialPrompt;
  },
};

/**
 * Bracketed paste mode per pane.
 * Tracked from mode-state-changed events so TerminalView can handle
 * Ctrl+Shift+V without coupling to each TerminalPane instance.
 */
export const bracketedPasteByPane = $state<Map<PaneId, boolean>>(new Map());

// ---------------------------------------------------------------------------
// Updaters — called from event handlers
// ---------------------------------------------------------------------------

/**
 * Apply a SshStateChangedEvent to the per-pane SSH lifecycle map.
 */
export function applySshStateChanged(ev: SshStateChangedEvent): void {
  const next = new Map(sshStates);
  next.set(ev.paneId, ev.state);
  // Replace the map reference to trigger Svelte 5 reactivity.
  sshStates.clear();
  for (const [k, v] of next) sshStates.set(k, v);
}

/**
 * Set the active host key prompt (opens TOFU dialog).
 */
export function setHostKeyPrompt(prompt: HostKeyPromptEvent): void {
  _ssh.hostKeyPrompt = prompt;
}

/**
 * Clear the host key prompt (dialog dismissed or accepted).
 * Returns the prompt that was cleared (for use in IPC calls), or null.
 */
export function clearHostKeyPrompt(): HostKeyPromptEvent | null {
  const prev = _ssh.hostKeyPrompt;
  _ssh.hostKeyPrompt = null;
  return prev;
}

/**
 * Set the active credential prompt (opens password dialog).
 */
export function setCredentialPrompt(prompt: CredentialPromptEvent): void {
  _ssh.credentialPrompt = prompt;
}

/**
 * Clear the credential prompt (dialog dismissed or submitted).
 * Returns the prompt that was cleared (for use in IPC calls), or null.
 */
export function clearCredentialPrompt(): CredentialPromptEvent | null {
  const prev = _ssh.credentialPrompt;
  _ssh.credentialPrompt = null;
  return prev;
}

/**
 * Update bracketed paste mode for a pane.
 */
export function setBracketedPaste(paneId: PaneId, active: boolean): void {
  bracketedPasteByPane.set(paneId, active);
}

/**
 * Returns the bracketed paste mode for a given pane (false if unknown).
 */
export function getBracketedPaste(paneId: PaneId): boolean {
  return bracketedPasteByPane.get(paneId) ?? false;
}

/**
 * Returns the SSH lifecycle state for a given pane, or null if unknown.
 */
export function getSshState(paneId: PaneId): SshLifecycleState | null {
  return sshStates.get(paneId) ?? null;
}
