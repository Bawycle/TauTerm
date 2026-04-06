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

/** SSH lifecycle state keyed by PaneId. */
export const sshStates = $state<Map<PaneId, SshLifecycleState>>(new Map());

/** Active TOFU / key-change host key dialog, or null when none pending. */
export let hostKeyPrompt = $state<HostKeyPromptEvent | null>(null);

/** Active credential prompt dialog, or null when none pending. */
export let credentialPrompt = $state<CredentialPromptEvent | null>(null);

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
  hostKeyPrompt = prompt;
}

/**
 * Clear the host key prompt (dialog dismissed or accepted).
 * Returns the prompt that was cleared (for use in IPC calls), or null.
 */
export function clearHostKeyPrompt(): HostKeyPromptEvent | null {
  const prev = hostKeyPrompt;
  hostKeyPrompt = null;
  return prev;
}

/**
 * Set the active credential prompt (opens password dialog).
 */
export function setCredentialPrompt(prompt: CredentialPromptEvent): void {
  credentialPrompt = prompt;
}

/**
 * Clear the credential prompt (dialog dismissed or submitted).
 * Returns the prompt that was cleared (for use in IPC calls), or null.
 */
export function clearCredentialPrompt(): CredentialPromptEvent | null {
  const prev = credentialPrompt;
  credentialPrompt = null;
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
