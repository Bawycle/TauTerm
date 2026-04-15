// SPDX-License-Identifier: MPL-2.0

/**
 * Unit tests for IPC command wrappers (src/lib/ipc/commands.ts).
 *
 * For each wrapper, verifies:
 *   - invoke() is called with the correct command name (snake_case)
 *   - Parameters are forwarded with the correct field names
 *   - Errors from invoke() are propagated to the caller
 *   - Commands that return void/null behave correctly
 */

import { vi, describe, it, expect, beforeEach } from 'vitest';
vi.mock('@tauri-apps/api/core', () => ({ invoke: vi.fn() }));
import { invoke } from '@tauri-apps/api/core';
import {
  getSessionState,
  createTab,
  closeTab,
  renameTab,
  reorderTab,
  setActiveTab,
  splitPane,
  closePane,
  setActivePane,
  setPaneLabel,
  sendInput,
  getPaneScreenSnapshot,
  resizePane,
  scrollPane,
  scrollToBottom,
  searchPane,
  openSshConnection,
  closeSshConnection,
  reconnectSsh,
  acceptHostKey,
  rejectHostKey,
  provideCredentials,
  providePassphrase,
  dismissSshAlgorithmWarning,
  getConnections,
  saveConnection,
  deleteConnection,
  duplicateConnection,
  storeConnectionPassword,
  getPreferences,
  updatePreferences,
  getThemes,
  saveTheme,
  deleteTheme,
  copyToClipboard,
  getClipboard,
  openUrl,
  markContextMenuUsed,
  toggleFullscreen,
  hasForegroundProcess,
} from './commands';

const mockInvoke = vi.mocked(invoke);

beforeEach(() => {
  mockInvoke.mockReset();
});

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/** Assert invoke was called with exact command name and args, then return a resolved promise. */
function expectInvoke(command: string, args?: Record<string, unknown>) {
  if (args !== undefined) {
    expect(mockInvoke).toHaveBeenCalledWith(command, args);
  } else {
    expect(mockInvoke).toHaveBeenCalledWith(command);
  }
}

// ---------------------------------------------------------------------------
// Session commands
// ---------------------------------------------------------------------------

describe('getSessionState', () => {
  it('calls get_session_state with no args', async () => {
    mockInvoke.mockResolvedValueOnce({ tabs: [], activeTabId: 't1' });
    await getSessionState();
    expectInvoke('get_session_state');
  });

  it('propagates invoke rejection', async () => {
    mockInvoke.mockRejectedValueOnce({ code: 'INTERNAL_ERROR', message: 'fail' });
    await expect(getSessionState()).rejects.toMatchObject({ code: 'INTERNAL_ERROR' });
  });
});

describe('createTab', () => {
  it('calls create_tab with config wrapper', async () => {
    const config = { cols: 80, rows: 24 };
    mockInvoke.mockResolvedValueOnce({});
    await createTab(config);
    expectInvoke('create_tab', { config });
  });

  it('propagates rejection', async () => {
    mockInvoke.mockRejectedValueOnce({ code: 'PTY_SPAWN_FAILED', message: 'fail' });
    await expect(createTab({ cols: 80, rows: 24 })).rejects.toMatchObject({
      code: 'PTY_SPAWN_FAILED',
    });
  });
});

describe('closeTab', () => {
  it('calls close_tab with tabId', async () => {
    mockInvoke.mockResolvedValueOnce(undefined);
    await closeTab('tab-1');
    expectInvoke('close_tab', { tabId: 'tab-1' });
  });
});

describe('renameTab', () => {
  it('calls rename_tab with tabId and label', async () => {
    mockInvoke.mockResolvedValueOnce({});
    await renameTab('tab-1', 'My Tab');
    expectInvoke('rename_tab', { tabId: 'tab-1', label: 'My Tab' });
  });

  it('accepts null label to clear user label', async () => {
    mockInvoke.mockResolvedValueOnce({});
    await renameTab('tab-1', null);
    expectInvoke('rename_tab', { tabId: 'tab-1', label: null });
  });
});

describe('reorderTab', () => {
  it('calls reorder_tab with tabId and newOrder', async () => {
    mockInvoke.mockResolvedValueOnce(undefined);
    await reorderTab('tab-1', 2);
    expectInvoke('reorder_tab', { tabId: 'tab-1', newOrder: 2 });
  });
});

describe('setActiveTab', () => {
  it('calls set_active_tab with tabId', async () => {
    mockInvoke.mockResolvedValueOnce(undefined);
    await setActiveTab('tab-1');
    expectInvoke('set_active_tab', { tabId: 'tab-1' });
  });
});

// ---------------------------------------------------------------------------
// Pane commands
// ---------------------------------------------------------------------------

describe('splitPane', () => {
  it('calls split_pane with paneId and direction', async () => {
    mockInvoke.mockResolvedValueOnce({});
    await splitPane('pane-1', 'horizontal');
    expectInvoke('split_pane', { paneId: 'pane-1', direction: 'horizontal' });
  });

  it('accepts vertical direction', async () => {
    mockInvoke.mockResolvedValueOnce({});
    await splitPane('pane-1', 'vertical');
    expectInvoke('split_pane', { paneId: 'pane-1', direction: 'vertical' });
  });
});

describe('closePane', () => {
  it('calls close_pane with paneId', async () => {
    mockInvoke.mockResolvedValueOnce({});
    await closePane('pane-1');
    expectInvoke('close_pane', { paneId: 'pane-1' });
  });

  it('returns null when the last pane is closed', async () => {
    mockInvoke.mockResolvedValueOnce(null);
    const result = await closePane('pane-1');
    expect(result).toBeNull();
  });
});

describe('setActivePane', () => {
  it('calls set_active_pane with paneId', async () => {
    mockInvoke.mockResolvedValueOnce(undefined);
    await setActivePane('pane-1');
    expectInvoke('set_active_pane', { paneId: 'pane-1' });
  });
});

describe('setPaneLabel', () => {
  it('calls set_pane_label with paneId and label', async () => {
    mockInvoke.mockResolvedValueOnce({});
    await setPaneLabel('pane-1', 'My Pane');
    expectInvoke('set_pane_label', { paneId: 'pane-1', label: 'My Pane' });
  });

  it('accepts null to clear label', async () => {
    mockInvoke.mockResolvedValueOnce({});
    await setPaneLabel('pane-1', null);
    expectInvoke('set_pane_label', { paneId: 'pane-1', label: null });
  });
});

// ---------------------------------------------------------------------------
// Terminal I/O commands
// ---------------------------------------------------------------------------

describe('sendInput', () => {
  it('calls send_input with paneId and data array', async () => {
    mockInvoke.mockResolvedValueOnce(undefined);
    await sendInput('pane-1', [65, 66, 67]);
    expectInvoke('send_input', { paneId: 'pane-1', data: [65, 66, 67] });
  });

  it('propagates rejection', async () => {
    mockInvoke.mockRejectedValueOnce({ code: 'PTY_IO_ERROR', message: 'write error' });
    await expect(sendInput('pane-1', [65])).rejects.toMatchObject({ code: 'PTY_IO_ERROR' });
  });
});

describe('getPaneScreenSnapshot', () => {
  it('calls get_pane_screen_snapshot with paneId', async () => {
    mockInvoke.mockResolvedValueOnce({ cols: 80, rows: 24, cells: [] });
    await getPaneScreenSnapshot('pane-1');
    expectInvoke('get_pane_screen_snapshot', { paneId: 'pane-1' });
  });
});

describe('resizePane', () => {
  it('calls resize_pane with all 5 parameters', async () => {
    mockInvoke.mockResolvedValueOnce(undefined);
    await resizePane('pane-1', 80, 24, 800, 480);
    expectInvoke('resize_pane', {
      paneId: 'pane-1',
      cols: 80,
      rows: 24,
      pixelWidth: 800,
      pixelHeight: 480,
    });
  });
});

// ---------------------------------------------------------------------------
// Scroll commands
// ---------------------------------------------------------------------------

describe('scrollPane', () => {
  it('calls scroll_pane with paneId and offset', async () => {
    mockInvoke.mockResolvedValueOnce({ offset: 10, scrollbackLines: 100 });
    await scrollPane('pane-1', 10);
    expectInvoke('scroll_pane', { paneId: 'pane-1', offset: 10 });
  });
});

describe('scrollToBottom', () => {
  it('calls scroll_to_bottom with paneId', async () => {
    mockInvoke.mockResolvedValueOnce(undefined);
    await scrollToBottom('pane-1');
    expectInvoke('scroll_to_bottom', { paneId: 'pane-1' });
  });
});

// ---------------------------------------------------------------------------
// Search commands
// ---------------------------------------------------------------------------

describe('searchPane', () => {
  it('calls search_pane with paneId and query', async () => {
    const query = { text: 'hello', caseSensitive: false, regex: false };
    mockInvoke.mockResolvedValueOnce([]);
    await searchPane('pane-1', query);
    expectInvoke('search_pane', { paneId: 'pane-1', query });
  });
});

// ---------------------------------------------------------------------------
// SSH commands
// ---------------------------------------------------------------------------

describe('openSshConnection', () => {
  it('calls open_ssh_connection with paneId and connectionId', async () => {
    mockInvoke.mockResolvedValueOnce(undefined);
    await openSshConnection('pane-1', 'conn-42');
    expectInvoke('open_ssh_connection', { paneId: 'pane-1', connectionId: 'conn-42' });
  });

  it('propagates rejection', async () => {
    mockInvoke.mockRejectedValueOnce({ code: 'SSH_CONNECTION_FAILED', message: 'refused' });
    await expect(openSshConnection('pane-1', 'conn-42')).rejects.toMatchObject({
      code: 'SSH_CONNECTION_FAILED',
    });
  });
});

describe('closeSshConnection', () => {
  it('calls close_ssh_connection with paneId', async () => {
    mockInvoke.mockResolvedValueOnce(undefined);
    await closeSshConnection('pane-1');
    expectInvoke('close_ssh_connection', { paneId: 'pane-1' });
  });
});

describe('reconnectSsh', () => {
  it('calls reconnect_ssh with paneId', async () => {
    mockInvoke.mockResolvedValueOnce(undefined);
    await reconnectSsh('pane-1');
    expectInvoke('reconnect_ssh', { paneId: 'pane-1' });
  });
});

describe('acceptHostKey', () => {
  it('calls accept_host_key with paneId', async () => {
    mockInvoke.mockResolvedValueOnce(undefined);
    await acceptHostKey('pane-1');
    expectInvoke('accept_host_key', { paneId: 'pane-1' });
  });
});

describe('rejectHostKey', () => {
  it('calls reject_host_key with paneId', async () => {
    mockInvoke.mockResolvedValueOnce(undefined);
    await rejectHostKey('pane-1');
    expectInvoke('reject_host_key', { paneId: 'pane-1' });
  });
});

describe('provideCredentials', () => {
  it('calls provide_credentials with paneId and credentials', async () => {
    const credentials = { username: 'alice', password: 'secret' };
    mockInvoke.mockResolvedValueOnce(undefined);
    await provideCredentials('pane-1', credentials);
    expectInvoke('provide_credentials', { paneId: 'pane-1', credentials });
  });

  it('propagates rejection on auth failure', async () => {
    mockInvoke.mockRejectedValueOnce({ code: 'SSH_AUTH_FAILED', message: 'bad credentials' });
    await expect(
      provideCredentials('pane-1', { username: 'alice', password: 'wrong' }),
    ).rejects.toMatchObject({ code: 'SSH_AUTH_FAILED' });
  });
});

describe('providePassphrase', () => {
  it('calls provide_passphrase with paneId, passphrase, and saveInKeychain', async () => {
    mockInvoke.mockResolvedValueOnce(undefined);
    await providePassphrase('pane-1', 'my-passphrase', true);
    expectInvoke('provide_passphrase', {
      paneId: 'pane-1',
      passphrase: 'my-passphrase',
      saveInKeychain: true,
    });
  });
});

describe('dismissSshAlgorithmWarning', () => {
  it('calls dismiss_ssh_algorithm_warning with paneId', async () => {
    mockInvoke.mockResolvedValueOnce(undefined);
    await dismissSshAlgorithmWarning('pane-1');
    expectInvoke('dismiss_ssh_algorithm_warning', { paneId: 'pane-1' });
  });
});

// ---------------------------------------------------------------------------
// Connection management commands
// ---------------------------------------------------------------------------

describe('getConnections', () => {
  it('calls get_connections with no args', async () => {
    mockInvoke.mockResolvedValueOnce([]);
    await getConnections();
    expectInvoke('get_connections');
  });
});

describe('saveConnection', () => {
  it('calls save_connection with config wrapper', async () => {
    const config = {
      id: 'conn-1',
      label: 'My Server',
      host: 'example.com',
      port: 22,
      username: 'alice',
      allowOsc52Write: false,
    };
    mockInvoke.mockResolvedValueOnce('conn-1');
    await saveConnection(config);
    expectInvoke('save_connection', { config });
  });
});

describe('deleteConnection', () => {
  it('calls delete_connection with connectionId', async () => {
    mockInvoke.mockResolvedValueOnce(undefined);
    await deleteConnection('conn-1');
    expectInvoke('delete_connection', { connectionId: 'conn-1' });
  });
});

describe('duplicateConnection', () => {
  it('calls duplicate_connection with connectionId', async () => {
    mockInvoke.mockResolvedValueOnce({});
    await duplicateConnection('conn-1');
    expectInvoke('duplicate_connection', { connectionId: 'conn-1' });
  });
});

describe('storeConnectionPassword', () => {
  it('calls store_connection_password with connectionId, username, password', async () => {
    mockInvoke.mockResolvedValueOnce(undefined);
    await storeConnectionPassword('conn-1', 'alice', 'hunter2');
    expectInvoke('store_connection_password', {
      connectionId: 'conn-1',
      username: 'alice',
      password: 'hunter2',
    });
  });
});

// ---------------------------------------------------------------------------
// Preferences commands
// ---------------------------------------------------------------------------

describe('getPreferences', () => {
  it('calls get_preferences with no args', async () => {
    mockInvoke.mockResolvedValueOnce({});
    await getPreferences();
    expectInvoke('get_preferences');
  });
});

describe('updatePreferences', () => {
  it('calls update_preferences with patch wrapper', async () => {
    const patch = { appearance: { fontSize: 14 } };
    mockInvoke.mockResolvedValueOnce({});
    await updatePreferences(patch);
    expectInvoke('update_preferences', { patch });
  });

  it('propagates rejection on validation error', async () => {
    mockInvoke.mockRejectedValueOnce({ code: 'PREF_INVALID_VALUE', message: 'bad value' });
    await expect(updatePreferences({ appearance: { fontSize: -1 } })).rejects.toMatchObject({
      code: 'PREF_INVALID_VALUE',
    });
  });
});

// ---------------------------------------------------------------------------
// Theme commands
// ---------------------------------------------------------------------------

describe('getThemes', () => {
  it('calls get_themes with no args', async () => {
    mockInvoke.mockResolvedValueOnce([]);
    await getThemes();
    expectInvoke('get_themes');
  });
});

describe('saveTheme', () => {
  it('calls save_theme with theme wrapper', async () => {
    const theme = {
      name: 'My Theme',
      palette: Array(16).fill('#000000') as [
        string,
        string,
        string,
        string,
        string,
        string,
        string,
        string,
        string,
        string,
        string,
        string,
        string,
        string,
        string,
        string,
      ],
      foreground: '#ffffff',
      background: '#000000',
      cursorColor: '#ffffff',
      selectionBg: '#555555',
    };
    mockInvoke.mockResolvedValueOnce(undefined);
    await saveTheme(theme);
    expectInvoke('save_theme', { theme });
  });
});

describe('deleteTheme', () => {
  it('calls delete_theme with name', async () => {
    mockInvoke.mockResolvedValueOnce(undefined);
    await deleteTheme('My Theme');
    expectInvoke('delete_theme', { name: 'My Theme' });
  });
});

// ---------------------------------------------------------------------------
// Clipboard commands
// ---------------------------------------------------------------------------

describe('copyToClipboard', () => {
  it('calls copy_to_clipboard with text', async () => {
    mockInvoke.mockResolvedValueOnce(undefined);
    await copyToClipboard('hello world');
    expectInvoke('copy_to_clipboard', { text: 'hello world' });
  });
});

describe('getClipboard', () => {
  it('calls get_clipboard with no args', async () => {
    mockInvoke.mockResolvedValueOnce('clipboard text');
    const result = await getClipboard();
    expectInvoke('get_clipboard');
    expect(result).toBe('clipboard text');
  });
});

// ---------------------------------------------------------------------------
// URL / misc commands
// ---------------------------------------------------------------------------

describe('openUrl', () => {
  it('calls open_url with url and optional paneId', async () => {
    mockInvoke.mockResolvedValueOnce(undefined);
    await openUrl('https://example.com', 'pane-1');
    expectInvoke('open_url', { url: 'https://example.com', paneId: 'pane-1' });
  });

  it('accepts url without paneId', async () => {
    mockInvoke.mockResolvedValueOnce(undefined);
    await openUrl('https://example.com');
    expectInvoke('open_url', { url: 'https://example.com', paneId: undefined });
  });

  it('propagates rejection for rejected URLs', async () => {
    mockInvoke.mockRejectedValueOnce({ code: 'INTERNAL_ERROR', message: 'rejected' });
    await expect(openUrl('file:///etc/passwd')).rejects.toMatchObject({
      code: 'INTERNAL_ERROR',
    });
  });
});

describe('markContextMenuUsed', () => {
  it('calls mark_context_menu_used with no args', async () => {
    mockInvoke.mockResolvedValueOnce(undefined);
    await markContextMenuUsed();
    expectInvoke('mark_context_menu_used');
  });
});

// ---------------------------------------------------------------------------
// Window commands
// ---------------------------------------------------------------------------

describe('toggleFullscreen', () => {
  it('calls toggle_fullscreen with no args and returns FullscreenState', async () => {
    mockInvoke.mockResolvedValueOnce({ isFullscreen: true });
    const result = await toggleFullscreen();
    expectInvoke('toggle_fullscreen');
    expect(result).toEqual({ isFullscreen: true });
  });
});

describe('hasForegroundProcess', () => {
  it('calls has_foreground_process with paneId and returns boolean', async () => {
    mockInvoke.mockResolvedValueOnce(true);
    const result = await hasForegroundProcess('pane-1');
    expectInvoke('has_foreground_process', { paneId: 'pane-1' });
    expect(result).toBe(true);
  });

  it('returns false when shell is idle', async () => {
    mockInvoke.mockResolvedValueOnce(false);
    const result = await hasForegroundProcess('pane-1');
    expect(result).toBe(false);
  });
});
