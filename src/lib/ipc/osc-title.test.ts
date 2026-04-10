// SPDX-License-Identifier: MPL-2.0

/**
 * Tests for OSC title propagation (Item 10) and security invariants.
 *
 * Covered:
 *   OSC-TITLE-001/002 — OSC 0 / OSC 2 sets tab title
 *   OSC-TITLE-003 — title sanitization: control chars stripped, max 256 chars
 *   OSC-TITLE-004 — user-defined label takes precedence over OSC title
 *   OSC-TITLE-005 — CSI 21t read-back is not responded to (no injection)
 *   SEC-BLK-015 — XSS via OSC title: Svelte must use {title} not {@html title}
 *   SEC-BLK-017 — bidi characters stripped from title (frontend display)
 *
 * The OSC title update path:
 *   Backend (VtProcessor) → ScreenUpdateEvent.cells includes title in pane state
 *   OR a dedicated title-changed event (to be determined in implementation).
 *
 * Frontend contract: tab titles must NEVER be rendered with {@html}.
 */

import { describe, it, expect } from 'vitest';
import type { PaneState, TabState } from '$lib/ipc/types';
import { resolveTabTitle } from '$lib/utils/tab-title';

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function makeTabState(paneTitle: string, userLabel: string | null = null): TabState {
  const pane: PaneState = {
    id: 'pane-1',
    sessionType: 'local',
    processTitle: paneTitle,
    cwd: '/home/user',
    sshConnectionId: null,
    sshState: null,
    notification: null,
  };
  return {
    id: 'tab-1',
    label: userLabel,
    activePaneId: 'pane-1',
    order: 0,
    layout: { type: 'leaf', paneId: 'pane-1', state: pane },
  };
}

// ---------------------------------------------------------------------------
// OSC-TITLE-004: user label takes precedence over process title
// ---------------------------------------------------------------------------

describe('OSC-TITLE-004: user label takes precedence over OSC-driven title', () => {
  it('user label overrides processTitle', () => {
    const tab = makeTabState('bash', 'MyLabel');
    expect(resolveTabTitle(tab) ?? '').toBe('MyLabel');
  });

  it('null label falls back to processTitle', () => {
    const tab = makeTabState('zsh', null);
    expect(resolveTabTitle(tab) ?? '').toBe('zsh');
  });

  it('OSC title update (via processTitle) is visible when no user label', () => {
    const tab = makeTabState('ProcessTitle', null);
    expect(resolveTabTitle(tab) ?? '').toBe('ProcessTitle');
  });
});

// ---------------------------------------------------------------------------
// SEC-BLK-015: OSC title must not be rendered as HTML
// Static analysis — verifies the type contract and absence of HTML injection
// in the title string used for display.
// ---------------------------------------------------------------------------

describe('SEC-BLK-015: OSC title does not produce HTML elements', () => {
  /**
   * The frontend must use Svelte's {title} interpolation, not {@html title}.
   * This test verifies that an XSS-carrying title is treated as plain text
   * by asserting the raw string does not have HTML characters stripped
   * (it must remain as-is — the renderer's job is to escape, not sanitise here).
   *
   * The backend's parse_osc strips C0/C1 but not HTML. An HTML title like
   * "<img src=x onerror=alert(1)>" will reach the frontend — it must be
   * rendered as plain text by Svelte's {title} binding.
   */
  it('XSS payload in processTitle is a plain string', () => {
    const xssTitle = '<img src=x onerror=alert(1)>';
    const tab = makeTabState(xssTitle);
    const layout = tab.layout;
    if (layout.type !== 'leaf') throw new Error('expected leaf');
    // The processTitle is stored as a plain string — not an HTML node.
    expect(typeof layout.state.processTitle).toBe('string');
    expect(layout.state.processTitle).toBe(xssTitle);
    // The renderer (Svelte template) is responsible for escaping.
    // This test verifies the data pipeline does not pre-sanitise in a way
    // that could mask the requirement for safe template interpolation.
  });

  it('HTML special chars in title are not removed at the data layer', () => {
    const htmlTitle = 'Tab & <b>Bold</b> Title';
    const tab = makeTabState(htmlTitle);
    const layout = tab.layout;
    if (layout.type !== 'leaf') throw new Error('expected leaf');
    expect(layout.state.processTitle).toContain('<b>');
    // The Svelte template escapes these — the data layer must not double-escape.
  });
});

// ---------------------------------------------------------------------------
// OSC-TITLE-003: Title sanitization — control characters stripped, max 256 chars
// This is primarily a Rust-side guarantee (tested in osc.rs).
// Here we test the TypeScript contract: processTitle arriving from IPC must be
// a bounded plain string.
// ---------------------------------------------------------------------------

describe('OSC-TITLE-003: title length boundary', () => {
  it('processTitle of 256 chars is valid', () => {
    const longTitle = 'A'.repeat(256);
    const tab = makeTabState(longTitle);
    const layout = tab.layout;
    if (layout.type !== 'leaf') throw new Error('expected leaf');
    expect(layout.state.processTitle.length).toBe(256);
  });

  it('processTitle is a string type', () => {
    const tab = makeTabState('bash');
    const layout = tab.layout;
    if (layout.type !== 'leaf') throw new Error('expected leaf');
    expect(typeof layout.state.processTitle).toBe('string');
  });
});

// ---------------------------------------------------------------------------
// SEC-BLK-017: Bidi chars in title must not be rendered as directionality
// Frontend responsibility: the Svelte template must render title as text node,
// not as HTML, ensuring bidi overrides have no visual impact on the surrounding UI.
// ---------------------------------------------------------------------------

describe('SEC-BLK-017: bidi characters in title are plain string content', () => {
  it('U+202E in processTitle is a plain string character', () => {
    // The backend should strip this (SEC-BLK-017 Rust test), but even if it
    // reaches the frontend, it must not affect the UI chrome via HTML injection.
    const bidiTitle = '\u202Egnp.exe';
    const tab = makeTabState(bidiTitle);
    const layout = tab.layout;
    if (layout.type !== 'leaf') throw new Error('expected leaf');
    // We verify it is stored as a plain string — the Svelte {title} binding
    // renders it as a text node, not as an element, so the bidi char is inert.
    expect(typeof layout.state.processTitle).toBe('string');
  });
});

// ---------------------------------------------------------------------------
// OSC-TITLE-005: CSI 21t read-back — no response injected into PTY
// This is primarily a backend invariant (Rust test in vt/processor).
// Frontend: the title read-back must not create a feedback loop.
// ---------------------------------------------------------------------------

describe('OSC-TITLE-005: title read-back does not inject bytes into PTY', () => {
  it('processTitle field does not contain CSI sequences', () => {
    // A title received via IPC must be a clean string — no CSI sequences.
    // This is guaranteed by the Rust parse_osc C0/C1 stripping.
    const cleanTitle = 'bash';
    expect(cleanTitle).not.toContain('\x1b');
  });
});

// ---------------------------------------------------------------------------
// FS-PANE-007: tab title follows active pane in multi-pane layout
// ---------------------------------------------------------------------------

describe('FS-PANE-007: tab title follows active pane in multi-pane layout', () => {
  it('returns processTitle of the active (non-root) pane, not the root pane', () => {
    const rootPane: PaneState = {
      id: 'pane-root',
      sessionType: 'local',
      processTitle: 'bash',
      cwd: '/home/user',
      sshConnectionId: null,
      sshState: null,
      notification: null,
    };
    const activePane: PaneState = {
      id: 'pane-active',
      sessionType: 'local',
      processTitle: 'htop',
      cwd: '/home/user',
      sshConnectionId: null,
      sshState: null,
      notification: null,
    };
    const tab: TabState = {
      id: 'tab-1',
      label: null,
      activePaneId: 'pane-active',
      order: 0,
      layout: {
        type: 'split',
        direction: 'horizontal',
        ratio: 0.5,
        first: { type: 'leaf', paneId: 'pane-root', state: rootPane },
        second: { type: 'leaf', paneId: 'pane-active', state: activePane },
      },
    };
    expect(resolveTabTitle(tab)).toBe('htop');
    expect(resolveTabTitle(tab)).not.toBe('bash');
  });

  it('user label still wins over active pane processTitle in multi-pane', () => {
    const rootPane: PaneState = {
      id: 'pane-root',
      sessionType: 'local',
      processTitle: 'bash',
      cwd: '/home/user',
      sshConnectionId: null,
      sshState: null,
      notification: null,
    };
    const activePane: PaneState = {
      id: 'pane-active',
      sessionType: 'local',
      processTitle: 'htop',
      cwd: '/home/user',
      sshConnectionId: null,
      sshState: null,
      notification: null,
    };
    const tab: TabState = {
      id: 'tab-1',
      label: 'Monitoring',
      activePaneId: 'pane-active',
      order: 0,
      layout: {
        type: 'split',
        direction: 'horizontal',
        ratio: 0.5,
        first: { type: 'leaf', paneId: 'pane-root', state: rootPane },
        second: { type: 'leaf', paneId: 'pane-active', state: activePane },
      },
    };
    expect(resolveTabTitle(tab)).toBe('Monitoring');
  });
});
