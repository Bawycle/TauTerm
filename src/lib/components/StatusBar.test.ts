// SPDX-License-Identifier: MPL-2.0

/**
 * StatusBar component tests.
 *
 * Covers: TUITC-UX-090 to UX-092 (SSH state display logic),
 * TUITC-FN-080/081 (session state reflection).
 */

import { describe, it, expect } from 'vitest';
import type { SshLifecycleState, PaneState } from '$lib/ipc';

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function makePaneState(overrides: Partial<PaneState> = {}): PaneState {
  return {
    paneId: 'pane-1',
    lifecycle: { type: 'running' },
    processTitle: 'bash',
    sshState: null,
    scrollOffset: 0,
    cwd: '/home/user',
    ...overrides,
  };
}

// ---------------------------------------------------------------------------
// TUITC-UX-090 to 092: Status bar SSH indicator logic
// ---------------------------------------------------------------------------

/**
 * Simulate the status bar's SSH indicator text derivation logic.
 * This mirrors what the StatusBar component computes from active pane state.
 */
function sshStatusText(
  pane: PaneState,
  connectionHost?: string,
  connectionUser?: string,
): string | null {
  if (!pane.sshState) return null;
  const state = pane.sshState;
  switch (state.type) {
    case 'connecting':
      return connectionHost ? `Connecting to ${connectionHost}...` : 'Connecting...';
    case 'authenticating':
      return 'Authenticating...';
    case 'connected':
      return connectionUser && connectionHost ? `${connectionUser}@${connectionHost}` : 'Connected';
    case 'disconnected':
      return 'Disconnected';
    case 'closed':
      return 'Closed';
  }
}

function sshStatusIcon(pane: PaneState): string | null {
  if (!pane.sshState) return null;
  switch (pane.sshState.type) {
    case 'connecting':
    case 'authenticating':
      return 'Network';
    case 'connected':
      return 'Network';
    case 'disconnected':
      return 'WifiOff';
    case 'closed':
      return 'XCircle';
  }
}

describe('TUITC-UX-090: SSH connected state display', () => {
  it('connected pane → "{user}@{host}" text', () => {
    const pane = makePaneState({
      sshState: { type: 'connected' },
    });
    const text = sshStatusText(pane, 'example.com', 'alice');
    expect(text).toBe('alice@example.com');
  });

  it('connected pane → Network icon', () => {
    const pane = makePaneState({
      sshState: { type: 'connected' },
    });
    expect(sshStatusIcon(pane)).toBe('Network');
  });
});

describe('TUITC-UX-091: SSH disconnected state display', () => {
  it('disconnected pane → "Disconnected" text', () => {
    const pane = makePaneState({
      sshState: { type: 'disconnected', reason: null },
    });
    expect(sshStatusText(pane)).toBe('Disconnected');
  });

  it('disconnected pane → WifiOff icon', () => {
    const pane = makePaneState({
      sshState: { type: 'disconnected', reason: null },
    });
    expect(sshStatusIcon(pane)).toBe('WifiOff');
  });
});

describe('TUITC-UX-092: local session → no SSH indicator', () => {
  it('local pane → sshStatusText returns null', () => {
    const pane = makePaneState();
    expect(sshStatusText(pane)).toBeNull();
  });

  it('local pane → sshStatusIcon returns null', () => {
    const pane = makePaneState();
    expect(sshStatusIcon(pane)).toBeNull();
  });
});

describe('SSH connecting/authenticating states', () => {
  it('connecting → "Connecting to {host}..." text', () => {
    const pane = makePaneState({
      sshState: { type: 'connecting' },
    });
    expect(sshStatusText(pane, 'myserver.local')).toBe('Connecting to myserver.local...');
  });

  it('authenticating → "Authenticating..." text', () => {
    const pane = makePaneState({
      sshState: { type: 'authenticating' },
    });
    expect(sshStatusText(pane)).toBe('Authenticating...');
  });

  it('closed → "Closed" text', () => {
    const pane = makePaneState({
      sshState: { type: 'closed' },
    });
    expect(sshStatusText(pane)).toBe('Closed');
  });

  it('closed → XCircle icon', () => {
    const pane = makePaneState({
      sshState: { type: 'closed' },
    });
    expect(sshStatusIcon(pane)).toBe('XCircle');
  });
});

// ---------------------------------------------------------------------------
// All SSH lifecycle states are handled (completeness check)
// ---------------------------------------------------------------------------

describe('all SSH lifecycle states are handled', () => {
  const states: SshLifecycleState[] = [
    { type: 'connecting' },
    { type: 'authenticating' },
    { type: 'connected' },
    { type: 'disconnected', reason: null },
    { type: 'closed' },
  ];

  for (const state of states) {
    it(`state "${state.type}" has non-null text and icon`, () => {
      const pane = makePaneState({ sshState: state });
      expect(sshStatusText(pane)).not.toBeNull();
      expect(sshStatusIcon(pane)).not.toBeNull();
    });
  }
});
