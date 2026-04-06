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

  it('ScreenUpdateEvent has cells, cursor, and scrollbackLines', () => {
    const event: ScreenUpdateEvent = {
      paneId: 'p1',
      cells: [],
      cursor: { row: 0, col: 0, visible: true, shape: 1, blink: false },
      scrollbackLines: 42,
    };
    expect(event.cursor.row).toBe(0);
    expect(event.scrollbackLines).toBe(42);
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

// ---------------------------------------------------------------------------
// TEST-SPRINT-003 — FS-I18N-006: Language type drift — 'en'/'fr' not 'En'/'Fr'
//
// The Rust backend serializes Language::En as "en" and Language::Fr as "fr"
// (serde rename_all = "camelCase" on a two-letter enum lowercases both).
// The TypeScript type MUST be 'en' | 'fr' — never 'En' | 'Fr'.
// ---------------------------------------------------------------------------

describe('TEST-SPRINT-003: Language IPC type uses lowercase values', () => {
  it('Language "en" is a valid value', () => {
    // TEST-SPRINT-003
    const lang: import('$lib/ipc/types').Language = 'en';
    expect(lang).toBe('en');
  });

  it('Language "fr" is a valid value', () => {
    // TEST-SPRINT-003
    const lang: import('$lib/ipc/types').Language = 'fr';
    expect(lang).toBe('fr');
  });

  it('AppearancePrefs.language field accepts "en"', () => {
    // TEST-SPRINT-003: verify the field type is correct through a full object.
    const prefs: import('$lib/ipc/types').AppearancePrefs = {
      fontFamily: 'monospace',
      fontSize: 14,
      cursorStyle: 'block',
      cursorBlinkMs: 530,
      themeName: 'umbra',
      opacity: 1.0,
      language: 'en',
      contextMenuHintShown: false,
      fullscreen: false,
    };
    expect(prefs.language).toBe('en');
  });

  it('AppearancePrefs.language field accepts "fr"', () => {
    // TEST-SPRINT-003
    const prefs: import('$lib/ipc/types').AppearancePrefs = {
      fontFamily: 'monospace',
      fontSize: 14,
      cursorStyle: 'block',
      cursorBlinkMs: 530,
      themeName: 'umbra',
      opacity: 1.0,
      language: 'fr',
      contextMenuHintShown: false,
      fullscreen: false,
    };
    expect(prefs.language).toBe('fr');
  });
});

// ---------------------------------------------------------------------------
// TEST-SPRINT-004 — BellType serialization contract
//
// The Rust backend serializes BellType as: none/visual/audio/both (camelCase
// applied to single-word variants lowercases them).
// The TypeScript type must match exactly.
// ---------------------------------------------------------------------------

describe('TEST-SPRINT-004: BellType IPC type uses lowercase camelCase values', () => {
  it('BellType "none" is a valid value', () => {
    // TEST-SPRINT-004
    const bell: import('$lib/ipc/types').BellType = 'none';
    expect(bell).toBe('none');
  });

  it('BellType "visual" is a valid value', () => {
    // TEST-SPRINT-004
    const bell: import('$lib/ipc/types').BellType = 'visual';
    expect(bell).toBe('visual');
  });

  it('BellType "audio" is a valid value', () => {
    // TEST-SPRINT-004
    const bell: import('$lib/ipc/types').BellType = 'audio';
    expect(bell).toBe('audio');
  });

  it('BellType "both" is a valid value', () => {
    // TEST-SPRINT-004
    const bell: import('$lib/ipc/types').BellType = 'both';
    expect(bell).toBe('both');
  });

  it('TerminalPrefs.bellType field accepts "visual"', () => {
    // TEST-SPRINT-004: verify field type via full object.
    const prefs: import('$lib/ipc/types').TerminalPrefs = {
      scrollbackLines: 10000,
      allowOsc52Write: false,
      wordDelimiters: ' \t|"\'`&()*,;<=>[]{}~',
      bellType: 'visual',
      confirmMultilinePaste: true,
    };
    expect(prefs.bellType).toBe('visual');
  });

  it('all four BellType values are distinct strings', () => {
    // TEST-SPRINT-004: guard against accidental alias collapse.
    const values: import('$lib/ipc/types').BellType[] = ['none', 'visual', 'audio', 'both'];
    const unique = new Set(values);
    expect(unique.size).toBe(4);
  });
});
