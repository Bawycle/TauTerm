// SPDX-License-Identifier: MPL-2.0
// Shared test fixtures for component tests

import type { TabState, PaneState } from '$lib/ipc/types';

export function makePaneState(overrides: Partial<PaneState> = {}): PaneState {
  return {
    id: 'pane-1',
    sessionType: 'local',
    processTitle: 'bash',
    cwd: '/home/user',
    sshConnectionId: null,
    sshState: null,
    notification: null,
    ...overrides,
  };
}

export function makeTab(overrides: Partial<TabState> = {}): TabState {
  const pane = makePaneState();
  return {
    id: 'tab-1',
    label: null,
    activePaneId: 'pane-1',
    order: 0,
    layout: { type: 'leaf', paneId: 'pane-1', state: pane },
    ...overrides,
  };
}

export const basePrefs = {
  appearance: {
    fontFamily: 'monospace',
    fontSize: 13,
    cursorStyle: 'block',
    cursorBlinkMs: 530,
    themeName: 'umbra',
    opacity: 1.0,
    language: 'en',
    contextMenuHintShown: true, // suppress hint overlay in tests
  },
  terminal: {
    scrollbackLines: 10000,
    allowOsc52Write: false,
    wordDelimiters: ' ,;:.{}[]()"`|\\/',
    bellType: 'visual',
    confirmMultilinePaste: true,
  },
  keyboard: { bindings: {} },
  connections: [],
  themes: [],
};
