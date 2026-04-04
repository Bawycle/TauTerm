// SPDX-License-Identifier: MPL-2.0

/**
 * Smoke tests for src/lib/ipc/types.ts.
 *
 * Goal: verify that the IPC type module is importable and that key structural
 * invariants hold at the type level.  These tests are intentionally
 * lightweight — the module is a pure type contract with no runtime logic of
 * its own.  Deeper behavioural tests belong in the modules that consume these
 * types.
 */

import { describe, it, expect } from 'vitest';
import type {
  SessionState,
  TabState,
  PaneState,
  PaneNode,
  SshLifecycleState,
  PaneNotification,
  ScreenUpdateEvent,
  CellUpdate,
  CellAttrsDto,
  ScrollPositionChangedEvent,
  SessionStateChangedEvent,
  SshStateChangedEvent,
  NotificationChangedEvent,
} from '$lib/ipc/types';

// ---------------------------------------------------------------------------
// The import above is the primary smoke test: if any type is broken or the
// module fails to resolve, the test file will not compile and the suite fails.
// The runtime assertions below validate structural shapes at the object level.
// ---------------------------------------------------------------------------

describe('IPC types — structural smoke tests', () => {
  it('SessionState shape is constructable with required fields', () => {
    const state: SessionState = {
      tabs: [],
      activeTabId: 'tab-1',
    };
    expect(state.tabs).toEqual([]);
    expect(state.activeTabId).toBe('tab-1');
  });

  it('PaneNode leaf variant has expected discriminant', () => {
    const node: PaneNode = {
      type: 'leaf',
      paneId: 'pane-1',
      state: {
        id: 'pane-1',
        sessionType: 'local',
        processTitle: 'bash',
        cwd: '/home/user',
        sshConnectionId: null,
        sshState: null,
        notification: null,
      },
    };
    expect(node.type).toBe('leaf');
  });

  it('PaneNode split variant has expected discriminant and ratio', () => {
    const leaf: PaneNode = {
      type: 'leaf',
      paneId: 'p1',
      state: {
        id: 'p1',
        sessionType: 'local',
        processTitle: 'sh',
        cwd: '/',
        sshConnectionId: null,
        sshState: null,
        notification: null,
      },
    };
    const node: PaneNode = {
      type: 'split',
      direction: 'horizontal',
      ratio: 0.5,
      first: leaf,
      second: leaf,
    };
    expect(node.type).toBe('split');
    if (node.type === 'split') {
      expect(node.ratio).toBe(0.5);
      expect(node.direction).toBe('horizontal');
    }
  });

  it('SshLifecycleState connecting variant has correct discriminant', () => {
    const state: SshLifecycleState = { type: 'connecting' };
    expect(state.type).toBe('connecting');
  });

  it('SshLifecycleState closed variant has no extra fields', () => {
    const state: SshLifecycleState = { type: 'closed' };
    expect(state.type).toBe('closed');
  });

  it('PaneNotification backgroundOutput type has no exitCode', () => {
    const notif: PaneNotification = { type: 'backgroundOutput' };
    expect(notif.type).toBe('backgroundOutput');
  });

  it('PaneNotification processExited type can carry exitCode', () => {
    const notif: PaneNotification = { type: 'processExited', exitCode: 0 };
    expect(notif.exitCode).toBe(0);
  });

  it('CellAttrsDto has all required fields', () => {
    const attrs: CellAttrsDto = {
      bold: false,
      dim: false,
      italic: false,
      underline: 0,
      blink: false,
      inverse: false,
      hidden: false,
      strikethrough: false,
    };
    expect(typeof attrs.bold).toBe('boolean');
    expect(typeof attrs.underline).toBe('number');
  });

  it('ScreenUpdateEvent has cells and cursor', () => {
    const event: ScreenUpdateEvent = {
      paneId: 'p1',
      cells: [],
      cursor: { row: 0, col: 0, visible: true, shape: 1, blink: false },
    };
    expect(event.cursor.row).toBe(0);
  });

  it('ScrollPositionChangedEvent has offset and scrollbackLines', () => {
    const event: ScrollPositionChangedEvent = {
      paneId: 'p1',
      offset: 10,
      scrollbackLines: 200,
    };
    expect(event.offset).toBe(10);
    expect(event.scrollbackLines).toBe(200);
  });

  it('SessionStateChangedEvent tab-closed has no tab field by convention', () => {
    const event: SessionStateChangedEvent = {
      changeType: 'tab-closed',
      activeTabId: 'tab-2',
    };
    expect(event.changeType).toBe('tab-closed');
    expect(event.tab).toBeUndefined();
  });

  it('SshStateChangedEvent carries paneId and state', () => {
    const event: SshStateChangedEvent = {
      paneId: 'p1',
      state: { type: 'connected' },
    };
    expect(event.paneId).toBe('p1');
    expect(event.state.type).toBe('connected');
  });

  it('NotificationChangedEvent allows null notification for clear', () => {
    const event: NotificationChangedEvent = {
      tabId: 'tab-1',
      paneId: 'p1',
      notification: null,
    };
    expect(event.notification).toBeNull();
  });

  // Verify that CellUpdate round-trips a plain object correctly.
  it('CellUpdate object matches expected field set', () => {
    const cell: CellUpdate = {
      row: 0,
      col: 5,
      content: 'A',
      attrs: {
        bold: true,
        italic: false,
        underline: 0,
        blink: false,
        inverse: false,
        dim: false,
        hidden: false,
        strikethrough: false,
      },
    };
    expect(cell.content).toBe('A');
    expect(cell.attrs.bold).toBe(true);
  });

  // Verify TabState shape.
  it('TabState has all required fields', () => {
    const leaf: PaneNode = {
      type: 'leaf',
      paneId: 'p1',
      state: {
        id: 'p1',
        sessionType: 'local',
        processTitle: 'bash',
        cwd: '/',
        sshConnectionId: null,
        sshState: null,
        notification: null,
      },
    };
    const tab: TabState = {
      id: 'tab-1',
      label: null,
      activePaneId: 'p1',
      order: 0,
      layout: leaf,
    };
    expect(tab.id).toBe('tab-1');
    expect(tab.label).toBeNull();
    expect(tab.order).toBe(0);
  });
});
