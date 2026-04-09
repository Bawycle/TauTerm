// SPDX-License-Identifier: MPL-2.0

/**
 * Focus management — button mousedown preventDefault tests.
 *
 * Covered:
 *   TEST-FOCUS-006 — ScrollToBottomButton preventDefault on mousedown
 *   TEST-FOCUS-009 — TerminalView SSH button onmousedown (static check)
 *   TEST-FOCUS-010 — TerminalView fullscreen button onmousedown (static check)
 *   TEST-FOCUS-011 — TabBarScroll left arrow preventDefault
 *   TEST-FOCUS-012 — TabBarScroll right arrow preventDefault
 */

import { describe, it, expect, vi, afterEach } from 'vitest';
import { mount, unmount, flushSync } from 'svelte';
import ScrollToBottomButton from '$lib/components/ScrollToBottomButton.svelte';
import TabBarScroll from '$lib/components/TabBarScroll.svelte';

// ---------------------------------------------------------------------------
// JSDOM polyfills
// ---------------------------------------------------------------------------

if (typeof Element.prototype.animate === 'undefined') {
  Object.defineProperty(Element.prototype, 'animate', {
    value: function () {
      return {
        finished: Promise.resolve(),
        cancel() {},
        finish() {},
        addEventListener() {},
        removeEventListener() {},
        dispatchEvent() {
          return true;
        },
      };
    },
    writable: true,
    configurable: true,
  });
}

// ---------------------------------------------------------------------------
// Shared teardown
// ---------------------------------------------------------------------------

const instances: ReturnType<typeof mount>[] = [];

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
  vi.restoreAllMocks();
});

// ---------------------------------------------------------------------------
// TEST-FOCUS-006: ScrollToBottomButton — mousedown calls preventDefault
// ---------------------------------------------------------------------------

describe('TEST-FOCUS-006: ScrollToBottomButton prevents focus steal on mousedown', () => {
  it('calls preventDefault on mousedown event', () => {
    const container = document.createElement('div');
    document.body.appendChild(container);

    const instance = mount(ScrollToBottomButton, {
      target: container,
      props: { onclick: vi.fn() },
    });
    instances.push(instance);

    const btn = container.querySelector('[role="button"]') as HTMLElement;
    expect(btn).not.toBeNull();

    const event = new MouseEvent('mousedown', { bubbles: true, cancelable: true });
    const preventDefaultSpy = vi.spyOn(event, 'preventDefault');

    btn.dispatchEvent(event);
    flushSync();

    expect(preventDefaultSpy).toHaveBeenCalledOnce();
  });

  it('verifies the onmousedown handler is present in the component source', async () => {
    // Static source check: confirms the onmousedown attribute is present in the
    // component template with the correct preventDefault call pattern.
    const fs = await import('fs');
    const path = await import('path');
    const filePath = path.resolve(process.cwd(), 'src/lib/components/ScrollToBottomButton.svelte');
    const source = fs.readFileSync(filePath, 'utf-8');
    expect(source).toContain('onmousedown');
    expect(source).toContain('preventDefault');
  });
});

// ---------------------------------------------------------------------------
// TEST-FOCUS-009: TerminalView SSH toggle button — mousedown calls preventDefault
//
// Mounting TerminalView requires mocking the full Tauri IPC surface (listen,
// invoke, session state), the Paraglide i18n runtime, and several composables.
// That level of scaffolding is disproportionate for testing a single attribute
// on a button. Instead, this test uses static source analysis: it reads the
// TerminalView.svelte source and asserts that the SSH button carries an
// onmousedown handler that calls preventDefault. This is an authoritative check
// because the source file is the single source of truth for the attribute.
// ---------------------------------------------------------------------------

describe('TEST-FOCUS-009: TerminalView SSH button prevents focus steal on mousedown', () => {
  it('TerminalView.svelte SSH toggle button has onmousedown + preventDefault (static check)', async () => {
    const fs = await import('fs');
    const path = await import('path');
    const filePath = path.resolve(process.cwd(), 'src/lib/components/TerminalView.svelte');
    const source = fs.readFileSync(filePath, 'utf-8');

    // Locate the SSH button block by its unique class
    const sshBtnIdx = source.indexOf('terminal-view__ssh-btn');
    expect(sshBtnIdx).toBeGreaterThan(-1);

    // The SSH button block should contain onmousedown + preventDefault within
    // a reasonable proximity (before the next closing angle bracket or button block)
    const sshBtnBlock = source.slice(sshBtnIdx, sshBtnIdx + 800);
    expect(sshBtnBlock).toContain('onmousedown');
    expect(sshBtnBlock).toContain('preventDefault');
  });
});

// ---------------------------------------------------------------------------
// TEST-FOCUS-010: TerminalView fullscreen toggle button — mousedown calls preventDefault
//
// Same rationale as TEST-FOCUS-009: static source analysis is used because
// mounting TerminalView in JSDOM requires prohibitive mock scaffolding.
// ---------------------------------------------------------------------------

describe('TEST-FOCUS-010: TerminalView fullscreen button prevents focus steal on mousedown', () => {
  it('TerminalView.svelte fullscreen toggle button has onmousedown + preventDefault (static check)', async () => {
    const fs = await import('fs');
    const path = await import('path');
    const filePath = path.resolve(process.cwd(), 'src/lib/components/TerminalView.svelte');
    const source = fs.readFileSync(filePath, 'utf-8');

    // Locate the fullscreen button by its unique data-testid attribute
    const fullscreenBtnIdx = source.indexOf('fullscreen-toggle-btn');
    expect(fullscreenBtnIdx).toBeGreaterThan(-1);

    // Extract a window around the fullscreen button and assert onmousedown + preventDefault
    const fullscreenBtnBlock = source.slice(fullscreenBtnIdx - 50, fullscreenBtnIdx + 500);
    expect(fullscreenBtnBlock).toContain('onmousedown');
    expect(fullscreenBtnBlock).toContain('preventDefault');
  });
});

// ---------------------------------------------------------------------------
// TEST-FOCUS-011: TabBarScroll left arrow — mousedown calls preventDefault
//
// TabBarScroll has simple props and can be mounted in JSDOM. The left arrow
// is rendered when canScrollLeft=true. The test dispatches a mousedown event
// on the rendered button and asserts preventDefault was called.
// ---------------------------------------------------------------------------

describe('TEST-FOCUS-011: TabBarScroll left arrow prevents focus steal on mousedown', () => {
  it('calls preventDefault on mousedown on the left scroll arrow', () => {
    const container = document.createElement('div');
    document.body.appendChild(container);

    const instance = mount(TabBarScroll, {
      target: container,
      props: {
        canScrollLeft: true,
        canScrollRight: false,
        leftBadge: null,
        rightBadge: null,
        onScrollLeft: vi.fn(),
        onScrollRight: vi.fn(),
      },
    });
    instances.push(instance);
    flushSync();

    const btn = container.querySelector('.tab-bar__scroll-arrow--left') as HTMLElement | null;
    expect(btn).not.toBeNull();

    const event = new MouseEvent('mousedown', { bubbles: true, cancelable: true });
    const preventDefaultSpy = vi.spyOn(event, 'preventDefault');

    btn!.dispatchEvent(event);
    flushSync();

    expect(preventDefaultSpy).toHaveBeenCalledOnce();
  });
});

// ---------------------------------------------------------------------------
// TEST-FOCUS-012: TabBarScroll right arrow — mousedown calls preventDefault
//
// Mirror of TEST-FOCUS-011 for the right scroll arrow.
// ---------------------------------------------------------------------------

describe('TEST-FOCUS-012: TabBarScroll right arrow prevents focus steal on mousedown', () => {
  it('calls preventDefault on mousedown on the right scroll arrow', () => {
    const container = document.createElement('div');
    document.body.appendChild(container);

    const instance = mount(TabBarScroll, {
      target: container,
      props: {
        canScrollLeft: false,
        canScrollRight: true,
        leftBadge: null,
        rightBadge: null,
        onScrollLeft: vi.fn(),
        onScrollRight: vi.fn(),
      },
    });
    instances.push(instance);
    flushSync();

    const btn = container.querySelector('.tab-bar__scroll-arrow--right') as HTMLElement | null;
    expect(btn).not.toBeNull();

    const event = new MouseEvent('mousedown', { bubbles: true, cancelable: true });
    const preventDefaultSpy = vi.spyOn(event, 'preventDefault');

    btn!.dispatchEvent(event);
    flushSync();

    expect(preventDefaultSpy).toHaveBeenCalledOnce();
  });
});
