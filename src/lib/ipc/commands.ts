// SPDX-License-Identifier: MPL-2.0

/**
 * Typed wrappers for all IPC commands (invoke() calls).
 *
 * Every call to the Tauri backend must go through these wrappers.
 * Each wrapper is a thin async function that handles only the serialisation
 * contract — error handling is left to the caller.
 *
 * Source of truth for command names: src-tauri/src/lib.rs generate_handler![].
 */

import { invoke } from '@tauri-apps/api/core';
import type {
  SessionState,
  TabState,
  TabId,
  PaneId,
  PaneState,
  ScreenSnapshot,
  ScrollPositionState,
  SearchQuery,
  SearchMatch,
  SshConnectionConfig,
  Preferences,
  PreferencesPatch,
  UserTheme,
  Credentials,
  CreateTabConfig,
  FullscreenState,
} from './types';

// ---------------------------------------------------------------------------
// Session commands
// ---------------------------------------------------------------------------

export function getSessionState(): Promise<SessionState> {
  return invoke('get_session_state');
}

export function createTab(config: CreateTabConfig): Promise<TabState> {
  return invoke('create_tab', { config });
}

export function closeTab(tabId: TabId): Promise<void> {
  return invoke('close_tab', { tabId });
}

export function renameTab(tabId: TabId, label: string | null): Promise<TabState> {
  return invoke('rename_tab', { tabId, label });
}

export function reorderTab(tabId: TabId, newOrder: number): Promise<void> {
  return invoke('reorder_tab', { tabId, newOrder });
}

export function setActiveTab(tabId: TabId): Promise<void> {
  return invoke('set_active_tab', { tabId });
}

// ---------------------------------------------------------------------------
// Pane commands
// ---------------------------------------------------------------------------

export function splitPane(paneId: PaneId, direction: 'horizontal' | 'vertical'): Promise<TabState> {
  return invoke('split_pane', { paneId, direction });
}

/** Returns the updated TabState, or null if the last pane was closed (tab removed). */
export function closePane(paneId: PaneId): Promise<TabState | null> {
  return invoke('close_pane', { paneId });
}

export function setActivePane(paneId: PaneId): Promise<void> {
  return invoke('set_active_pane', { paneId });
}

export function setPaneLabel(paneId: PaneId, label: string | null): Promise<TabState> {
  return invoke('set_pane_label', { paneId, label });
}

// ---------------------------------------------------------------------------
// Terminal I/O commands
// ---------------------------------------------------------------------------

export function sendInput(paneId: PaneId, data: number[]): Promise<void> {
  return invoke('send_input', { paneId, data });
}

export function getPaneScreenSnapshot(paneId: PaneId): Promise<ScreenSnapshot> {
  return invoke('get_pane_screen_snapshot', { paneId });
}

export function resizePane(
  paneId: PaneId,
  cols: number,
  rows: number,
  pixelWidth: number,
  pixelHeight: number,
): Promise<void> {
  return invoke('resize_pane', { paneId, cols, rows, pixelWidth, pixelHeight });
}

// ---------------------------------------------------------------------------
// Scroll commands
// ---------------------------------------------------------------------------

export function scrollPane(paneId: PaneId, offset: number): Promise<ScrollPositionState> {
  return invoke('scroll_pane', { paneId, offset });
}

export function scrollToBottom(paneId: PaneId): Promise<void> {
  return invoke('scroll_to_bottom', { paneId });
}

// ---------------------------------------------------------------------------
// Search commands
// ---------------------------------------------------------------------------

export function searchPane(paneId: PaneId, query: SearchQuery): Promise<SearchMatch[]> {
  return invoke('search_pane', { paneId, query });
}

// ---------------------------------------------------------------------------
// SSH commands
// ---------------------------------------------------------------------------

export function openSshConnection(paneId: PaneId, connectionId: string): Promise<void> {
  return invoke('open_ssh_connection', { paneId, connectionId });
}

export function closeSshConnection(paneId: PaneId): Promise<void> {
  return invoke('close_ssh_connection', { paneId });
}

export function reconnectSsh(paneId: PaneId): Promise<void> {
  return invoke('reconnect_ssh', { paneId });
}

export function acceptHostKey(paneId: PaneId): Promise<void> {
  return invoke('accept_host_key', { paneId });
}

export function rejectHostKey(paneId: PaneId): Promise<void> {
  return invoke('reject_host_key', { paneId });
}

export function provideCredentials(paneId: PaneId, credentials: Credentials): Promise<void> {
  return invoke('provide_credentials', { paneId, credentials });
}

export function providePassphrase(
  paneId: PaneId,
  passphrase: string,
  saveInKeychain: boolean,
): Promise<void> {
  return invoke('provide_passphrase', { paneId, passphrase, saveInKeychain });
}

export function dismissSshAlgorithmWarning(paneId: PaneId): Promise<void> {
  return invoke('dismiss_ssh_algorithm_warning', { paneId });
}

// ---------------------------------------------------------------------------
// Connection management commands
// ---------------------------------------------------------------------------

export function getConnections(): Promise<SshConnectionConfig[]> {
  return invoke('get_connections');
}

export function saveConnection(config: SshConnectionConfig): Promise<string> {
  return invoke('save_connection', { config });
}

export function deleteConnection(connectionId: string): Promise<void> {
  return invoke('delete_connection', { connectionId });
}

export function duplicateConnection(connectionId: string): Promise<SshConnectionConfig> {
  return invoke<SshConnectionConfig>('duplicate_connection', { connectionId });
}

export function storeConnectionPassword(
  connectionId: string,
  username: string,
  password: string,
): Promise<void> {
  return invoke('store_connection_password', { connectionId, username, password });
}

// ---------------------------------------------------------------------------
// Preferences commands
// ---------------------------------------------------------------------------

export function getPreferences(): Promise<Preferences> {
  return invoke('get_preferences');
}

export function updatePreferences(patch: PreferencesPatch): Promise<Preferences> {
  return invoke('update_preferences', { patch });
}

// ---------------------------------------------------------------------------
// Theme commands
// ---------------------------------------------------------------------------

export function getThemes(): Promise<UserTheme[]> {
  return invoke('get_themes');
}

export function saveTheme(theme: UserTheme): Promise<void> {
  return invoke('save_theme', { theme });
}

export function deleteTheme(name: string): Promise<void> {
  return invoke('delete_theme', { name });
}

// ---------------------------------------------------------------------------
// Clipboard commands
// ---------------------------------------------------------------------------

export function copyToClipboard(text: string): Promise<void> {
  return invoke('copy_to_clipboard', { text });
}

export function getClipboard(): Promise<string> {
  return invoke('get_clipboard');
}

// ---------------------------------------------------------------------------
// URL / misc commands
// ---------------------------------------------------------------------------

export function openUrl(url: string, paneId?: string): Promise<void> {
  return invoke('open_url', { url, paneId });
}

export function markContextMenuUsed(): Promise<void> {
  return invoke('mark_context_menu_used');
}

// ---------------------------------------------------------------------------
// Window commands
// ---------------------------------------------------------------------------

export function toggleFullscreen(): Promise<FullscreenState> {
  return invoke<FullscreenState>('toggle_fullscreen');
}

/**
 * Returns true if a non-shell foreground process is active in the given pane.
 * Returns false when the pane is idle (shell at prompt), terminated, or not Running.
 * Used by FS-PTY-008 close confirmation logic.
 * @command has_foreground_process
 */
export function hasForegroundProcess(paneId: PaneId): Promise<boolean> {
  return invoke<boolean>('has_foreground_process', { paneId });
}

// Re-export CreateTabConfig so callers can use it from this module
export type { CreateTabConfig, PaneState };
