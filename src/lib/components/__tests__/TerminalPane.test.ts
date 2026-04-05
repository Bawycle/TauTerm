// SPDX-License-Identifier: MPL-2.0

/**
 * TerminalPane scroll-to-bottom integration tests.
 *
 * Covered (passing):
 *   TPSC-FN-001 — ScrollToBottomButton absent on initial render (scrollOffset=0)
 *   TPSC-STRUCT-001 — TerminalPane mounts without errors
 *   TPSC-STRUCT-002 — viewport element has expected CSS class
 *   TPSC-STRUCT-003 — pane element has data-pane-id attribute
 *
 * E2E-deferred (require capturing IPC listen() handlers — not feasible in jsdom
 * because vitest module aliases prevent vi.mock from intercepting the listen binding
 * already captured by the Svelte component at import time):
 *   TPSC-FN-002 — ScrollToBottomButton present when scrollOffset > 0 (scroll-position-changed event)
 *   TPSC-FN-003 — button appears after positive offset event
 *   TPSC-FN-004 — button disappears after offset=0 event
 *   TPSC-FN-005 — clicking button invokes scroll_pane with offset 0
 *   TPSC-FN-006 — screen-update event updates scrollbackLines
 *   TPSC-FN-007 — events for different paneId are ignored
 */

import { describe, it, expect, vi, afterEach, beforeEach } from 'vitest';
import { mount, unmount } from 'svelte';
import { flushSync } from 'svelte';
import TerminalPane from '../TerminalPane.svelte';

// ---------------------------------------------------------------------------
// JSDOM polyfills
// ---------------------------------------------------------------------------

// jsdom does not implement ResizeObserver — stub it to a no-op.
class ResizeObserverStub {
  observe() {}
  unobserve() {}
  disconnect() {}
}
if (typeof globalThis.ResizeObserver === 'undefined') {
  globalThis.ResizeObserver = ResizeObserverStub as unknown as typeof ResizeObserver;
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

async function mountPane(props?: {
  paneId?: string;
  tabId?: string;
  active?: boolean;
}): Promise<{ container: HTMLElement; instance: ReturnType<typeof mount> }> {
  const container = document.createElement('div');
  document.body.appendChild(container);
  const instance = mount(TerminalPane, {
    target: container,
    props: {
      paneId: props?.paneId ?? 'test-pane-1',
      tabId: props?.tabId ?? 'test-tab-1',
      active: props?.active ?? true,
    },
  });
  await Promise.resolve();
  await Promise.resolve();
  await Promise.resolve();
  flushSync();
  return { container, instance };
}

const instances: ReturnType<typeof mount>[] = [];

beforeEach(() => {
  vi.restoreAllMocks();
});

afterEach(() => {
  instances.forEach((i) => {
    try {
      unmount(i);
    } catch {
      /* ignore */
    }
  });
  instances.length = 0;
  document.body.innerHTML = '';
});

// ---------------------------------------------------------------------------
// Structural / mount tests
// ---------------------------------------------------------------------------

describe('TPSC-STRUCT-001: TerminalPane mounts without errors', () => {
  it('mounts without throwing', async () => {
    const { container, instance } = await mountPane();
    instances.push(instance);
    expect(container.querySelector('.terminal-pane')).not.toBeNull();
  });
});

describe('TPSC-STRUCT-002: viewport element has expected CSS class', () => {
  it('renders .terminal-grid viewport element', async () => {
    const { container, instance } = await mountPane();
    instances.push(instance);
    expect(container.querySelector('.terminal-grid')).not.toBeNull();
  });
});

describe('TPSC-STRUCT-003: pane element has data-pane-id attribute', () => {
  it('sets data-pane-id from the paneId prop', async () => {
    const { container, instance } = await mountPane({ paneId: 'my-pane-42' });
    instances.push(instance);
    const pane = container.querySelector('[data-pane-id="my-pane-42"]');
    expect(pane).not.toBeNull();
  });
});

describe('TPSC-FN-001: ScrollToBottomButton absent at initial render (scrollOffset=0)', () => {
  it('does not render .scroll-to-bottom-btn on initial mount', async () => {
    const { container, instance } = await mountPane();
    instances.push(instance);
    // scrollOffset starts at 0 — button must not be rendered
    expect(container.querySelector('.scroll-to-bottom-btn')).toBeNull();
  });
});

// ---------------------------------------------------------------------------
// E2E-deferred scenarios
// These require firing IPC listen() event handlers from inside jsdom tests,
// which is not feasible due to the vitest alias/vi.mock interception order.
// They will be covered in E2E tests (WebdriverIO + real Tauri backend).
// ---------------------------------------------------------------------------

describe('TPSC-FN-002 [E2E-deferred]: ScrollToBottomButton present when scrollOffset > 0', () => {
  it.todo('renders scroll-to-bottom button after scroll-position-changed event with offset > 0');
});

describe('TPSC-FN-003 [E2E-deferred]: scroll-position-changed makes button appear', () => {
  it.todo('button appears after receiving scroll event with positive offset');
});

describe('TPSC-FN-004 [E2E-deferred]: scroll-position-changed offset=0 hides button', () => {
  it.todo('button disappears after scrolling back to bottom (offset=0 event)');
});

describe('TPSC-FN-005 [E2E-deferred]: clicking button invokes scroll_pane', () => {
  it.todo('calls invoke("scroll_pane", { paneId, offset: 0 }) on button click');
});

describe('TPSC-FN-006 [E2E-deferred]: screen-update event updates scrollbackLines', () => {
  it.todo('scrollbackLines is updated when screen-update event carries the field');
});

describe('TPSC-FN-007 [E2E-deferred]: events for different paneId are ignored', () => {
  it.todo('scroll-position-changed for a different pane does not show button');
});
