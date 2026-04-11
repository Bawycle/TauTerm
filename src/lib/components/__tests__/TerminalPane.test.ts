// SPDX-License-Identifier: MPL-2.0

/**
 * TerminalPane scroll-to-bottom integration tests.
 *
 * Covered (passing):
 *   TPSC-FN-001 — ScrollToBottomButton absent on initial render (scrollOffset=0)
 *   TPSC-STRUCT-001 — TerminalPane mounts without errors
 *   TPSC-STRUCT-002 — viewport element has expected CSS class
 *   TPSC-STRUCT-003 — pane element has data-pane-id attribute
 *   CURSOR-UNFOCUSED-001 — cursor carries --unfocused class when active=false
 *   CURSOR-UNFOCUSED-002 — cursor does NOT carry --unfocused class when active=true
 *   SSH-OVERLAY-REACT-001 — connecting overlay renders when sshStates is set to 'connecting'
 *   SSH-OVERLAY-REACT-002 — connecting overlay renders when sshStates is set to 'authenticating'
 *   SSH-OVERLAY-REACT-003 — connecting overlay disappears when sshStates transitions to 'connected'
 *   SSH-OVERLAY-REACT-004 — overlay ignores state for a different paneId
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
import { mount, unmount, tick } from 'svelte';
import { flushSync } from 'svelte';
import TerminalPane from '../TerminalPane.svelte';
import { applySshStateChanged, sshStates } from '$lib/state/ssh.svelte';

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

// ---------------------------------------------------------------------------
// C1-REGRESSION: TerminalPane keeps role="region" + aria-label (WCAG 1.3.6)
//
// TerminalPane must retain role="region" with an aria-label — it is a named
// landmark representing a pane in a split layout.  Only the pane-area wrapper
// in TerminalView lost its unnamed role="region"; this is a distinct element.
// This test prevents accidental removal during future refactors.
// ---------------------------------------------------------------------------

describe('C1-REGRESSION: TerminalPane retains role="region" landmark', () => {
  it('pane root element has role="region"', async () => {
    const { container, instance } = await mountPane();
    instances.push(instance);
    const region = container.querySelector('[role="region"]');
    expect(region).not.toBeNull();
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
// CSS cascade priority: selection must override search-match (F1)
// ---------------------------------------------------------------------------

describe('CSS-PRIORITY-001: selection classes declared after search-match in stylesheet', () => {
  /**
   * Both .terminal-pane__cell--selected and .terminal-pane__cell--search-match
   * use !important. With equal specificity, the last declaration in source order
   * wins. Selection must be declared AFTER search-match so it takes priority
   * when a cell is both selected and a search match.
   *
   * This is a static source-order check — the only reliable approach in jsdom
   * since it does not compute cascade priorities for scoped <style> blocks.
   */
  it('--selected is declared after --search-match in TerminalPane.svelte source', async () => {
    // CSS styles are in TerminalPane.svelte (as :global() rules).
    // The order check uses indexOf for --search-match and lastIndexOf for --selected
    // to verify cascade priority regardless of :global() wrapper syntax.
    const fs = await import('fs');
    const path = await import('path');
    const filePath = path.resolve(process.cwd(), 'src/lib/components/TerminalPane.svelte');
    const source = fs.readFileSync(filePath, 'utf-8');
    const searchMatchIdx = source.indexOf('terminal-pane__cell--search-match');
    // Match both `.terminal-pane__cell--selected {` and `:global(.terminal-pane__cell--selected)`
    const selectedIdx = source.lastIndexOf('terminal-pane__cell--selected');
    expect(searchMatchIdx).toBeGreaterThan(-1);
    expect(selectedIdx).toBeGreaterThan(-1);
    expect(selectedIdx).toBeGreaterThan(searchMatchIdx);
  });
});

// ---------------------------------------------------------------------------
// F4 — Blink: .terminal-pane__cell--blink class must be declared in TerminalPane.svelte
// ---------------------------------------------------------------------------

describe('F4-CSS-001: --blink CSS class exists in TerminalPane.svelte source', () => {
  it('declares .terminal-pane__cell--blink rule', async () => {
    const fs = await import('fs');
    const path = await import('path');
    const filePath = path.resolve(process.cwd(), 'src/lib/components/TerminalPane.svelte');
    const source = fs.readFileSync(filePath, 'utf-8');
    expect(source).toContain('terminal-pane__cell--blink');
  });

  it('declares @keyframes term-blink animation', async () => {
    const fs = await import('fs');
    const path = await import('path');
    const filePath = path.resolve(process.cwd(), 'src/lib/components/TerminalPane.svelte');
    const source = fs.readFileSync(filePath, 'utf-8');
    expect(source).toContain('@keyframes term-blink');
  });

  it('includes prefers-reduced-motion: reduce guard for blink', async () => {
    const fs = await import('fs');
    const path = await import('path');
    const filePath = path.resolve(process.cwd(), 'src/lib/components/TerminalPane.svelte');
    const source = fs.readFileSync(filePath, 'utf-8');
    // Must have a reduced-motion block that disables the blink animation
    const reducedMotionIdx = source.indexOf('prefers-reduced-motion: reduce');
    const blinkAnimNoneIdx = source.indexOf('animation: none', reducedMotionIdx);
    expect(reducedMotionIdx).toBeGreaterThan(-1);
    expect(blinkAnimNoneIdx).toBeGreaterThan(reducedMotionIdx);
  });

  it('cell.blink === true produces --blink class binding in template', async () => {
    // Template is in TerminalPaneViewport.svelte (refactored from TerminalPane.svelte).
    // CSS rule remains in TerminalPane.svelte as :global().
    const fs = await import('fs');
    const path = await import('path');
    const viewportPath = path.resolve(
      process.cwd(),
      'src/lib/components/TerminalPaneViewport.svelte',
    );
    const panePath = path.resolve(process.cwd(), 'src/lib/components/TerminalPane.svelte');
    const viewportSource = fs.readFileSync(viewportPath, 'utf-8');
    const paneSource = fs.readFileSync(panePath, 'utf-8');
    // The template must bind --blink class based on cell.blink
    expect(viewportSource).toContain('cell.blink');
    expect(viewportSource + paneSource).toContain('terminal-pane__cell--blink');
  });
});

// ---------------------------------------------------------------------------
// P-OPT-3 — CSS containment: `contain: strict` on .terminal-pane__viewport
//
// jsdom does not compute styles from Svelte <style> blocks (getComputedStyle
// returns empty strings for :global() rules). The test verifies the CSS rule
// is declared in the source file — the only reliable approach in jsdom.
// Runtime effect (WebKitGTK repaint scope reduction) is measured by the E2E
// benchmark in tests/e2e/perf-p12a-frame-render.spec.ts.
// ---------------------------------------------------------------------------

describe('P-OPT-3-CSS-001: contain:strict declared on .terminal-pane__viewport', () => {
  it('declares contain: strict inside the .terminal-pane__viewport rule', async () => {
    const fs = await import('fs');
    const path = await import('path');
    const filePath = path.resolve(process.cwd(), 'src/lib/components/TerminalPane.svelte');
    const source = fs.readFileSync(filePath, 'utf-8');

    // Anchor on the two adjacent selectors to extract only the viewport block.
    // Using selector names as delimiters is more stable than searching for the
    // next :global( occurrence, which would break if a nested :global() were added.
    const viewportStart = source.indexOf('terminal-pane__viewport)');
    const viewportEnd = source.indexOf('terminal-pane__row)', viewportStart + 1);
    const viewportBlock = source.slice(viewportStart, viewportEnd);

    expect(viewportStart).toBeGreaterThan(-1);
    expect(viewportEnd).toBeGreaterThan(viewportStart);
    expect(viewportBlock).toContain('contain: strict');
  });

  it('renders the .terminal-pane__viewport element (CSS selector target exists in DOM)', async () => {
    // Confirms the element targeted by the :global(.terminal-pane__viewport) selector
    // is present in the rendered DOM. jsdom cannot compute the style, but we verify
    // the element exists so that the selector is not targeting a phantom.
    const { container, instance } = await mountPane();
    instances.push(instance);
    expect(container.querySelector('.terminal-pane__viewport')).not.toBeNull();
  });
});

// ---------------------------------------------------------------------------
// F9 — Strikethrough: --strikethrough class must be declared, not text-decoration
// ---------------------------------------------------------------------------

describe('F9-CSS-001: --strikethrough class uses ::after pseudo-element positioning', () => {
  it('declares .terminal-pane__cell--strikethrough rule', async () => {
    const fs = await import('fs');
    const path = await import('path');
    const filePath = path.resolve(process.cwd(), 'src/lib/components/TerminalPane.svelte');
    const source = fs.readFileSync(filePath, 'utf-8');
    expect(source).toContain('terminal-pane__cell--strikethrough');
  });

  it('uses ::after pseudo-element with --term-strikethrough-position token', async () => {
    const fs = await import('fs');
    const path = await import('path');
    const filePath = path.resolve(process.cwd(), 'src/lib/components/TerminalPane.svelte');
    const source = fs.readFileSync(filePath, 'utf-8');
    expect(source).toContain('--strikethrough::after');
    expect(source).toContain('--term-strikethrough-position');
    expect(source).toContain('--term-strikethrough-thickness');
  });

  it('cell.strikethrough === true produces --strikethrough class binding in template', async () => {
    // Template is in TerminalPaneViewport.svelte (refactored from TerminalPane.svelte).
    // CSS rule remains in TerminalPane.svelte as :global().
    const fs = await import('fs');
    const path = await import('path');
    const viewportPath = path.resolve(
      process.cwd(),
      'src/lib/components/TerminalPaneViewport.svelte',
    );
    const panePath = path.resolve(process.cwd(), 'src/lib/components/TerminalPane.svelte');
    const viewportSource = fs.readFileSync(viewportPath, 'utf-8');
    const paneSource = fs.readFileSync(panePath, 'utf-8');
    expect(viewportSource).toContain('cell.strikethrough');
    expect(viewportSource + paneSource).toContain('terminal-pane__cell--strikethrough');
  });
});

// ---------------------------------------------------------------------------
// CURSOR-UNFOCUSED — .terminal-pane__cursor--unfocused class (TUITC-UX-053)
//
// The cursor element carries --unfocused when active=false, giving it a hollow
// outline (via CSS token --term-cursor-unfocused). When active=true the class
// must be absent.
//
// cursor.visible starts as true in the composable, so the cursor element is
// present in the DOM on the initial render regardless of blink settings.
// ---------------------------------------------------------------------------

describe('CURSOR-UNFOCUSED-001: cursor has --unfocused class when active=false', () => {
  it('renders .terminal-pane__cursor--unfocused when pane is not active', async () => {
    const { container, instance } = await mountPane({ active: false });
    instances.push(instance);
    const cursorEl = container.querySelector('.terminal-pane__cursor');
    expect(cursorEl).not.toBeNull();
    expect(cursorEl!.classList.contains('terminal-pane__cursor--unfocused')).toBe(true);
  });
});

describe('CURSOR-UNFOCUSED-002: cursor does NOT have --unfocused class when active=true', () => {
  it('does not render .terminal-pane__cursor--unfocused when pane is active', async () => {
    const { container, instance } = await mountPane({ active: true });
    instances.push(instance);
    const cursorEl = container.querySelector('.terminal-pane__cursor');
    expect(cursorEl).not.toBeNull();
    expect(cursorEl!.classList.contains('terminal-pane__cursor--unfocused')).toBe(false);
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

// ---------------------------------------------------------------------------
// SSH connecting overlay reactivity (SSH-OVERLAY-REACT-*)
//
// Verifies that TerminalPane reads sshStates directly from the module-level
// $state proxy (not via prop), so mutations to sshStates are reactive.
// This is the regression test for the bug where the overlay never appeared
// because $state(Map) mutations are not tracked when the Map is passed as prop.
// ---------------------------------------------------------------------------

// Flush Svelte reactive updates triggered by module-level $state mutations.
// tick() waits for Svelte to process pending DOM updates after a state change.
async function settle(): Promise<void> {
  await tick();
  flushSync();
  await tick();
}

describe('SSH-OVERLAY-REACT-001: overlay renders when sshStates becomes connecting', () => {
  it('shows .ssh-connecting-overlay when state is set BEFORE mount', async () => {
    // Diagnostic variant: set state BEFORE mounting to verify $derived reads it on first render.
    // This distinguishes "new key added after mount" from "existing key updated after mount".
    const paneId = 'react-test-pane-1';
    applySshStateChanged({ paneId, state: { type: 'connecting' } });

    const { container, instance } = await mountPane({ paneId });
    instances.push(instance);

    expect(container.querySelector('.ssh-connecting-overlay')).not.toBeNull();

    sshStates[paneId] = undefined;
  });

  it('shows .ssh-connecting-overlay when state transitions to connecting AFTER mount', async () => {
    const paneId = 'react-test-pane-1b';
    const { container, instance } = await mountPane({ paneId });
    instances.push(instance);

    expect(container.querySelector('.ssh-connecting-overlay')).toBeNull();

    applySshStateChanged({ paneId, state: { type: 'connecting' } });
    expect(sshStates[paneId]).toEqual({ type: 'connecting' });
    await settle();

    expect(container.querySelector('.ssh-connecting-overlay')).not.toBeNull();

    sshStates[paneId] = undefined;
  });
});

describe('SSH-OVERLAY-REACT-002: overlay renders when sshStates becomes authenticating', () => {
  it('shows .ssh-connecting-overlay when state transitions to authenticating', async () => {
    const paneId = 'react-test-pane-2';
    const { container, instance } = await mountPane({ paneId });
    instances.push(instance);

    applySshStateChanged({ paneId, state: { type: 'authenticating' } });
    await settle();

    expect(container.querySelector('.ssh-connecting-overlay')).not.toBeNull();

    sshStates[paneId] = undefined;
  });
});

describe('SSH-OVERLAY-REACT-003: overlay disappears when sshStates transitions to connected', () => {
  it('hides .ssh-connecting-overlay when state moves from connecting to connected', async () => {
    const paneId = 'react-test-pane-3';
    const { container, instance } = await mountPane({ paneId });
    instances.push(instance);

    applySshStateChanged({ paneId, state: { type: 'connecting' } });
    await settle();
    expect(container.querySelector('.ssh-connecting-overlay')).not.toBeNull();

    applySshStateChanged({ paneId, state: { type: 'connected' } });
    await settle();
    expect(container.querySelector('.ssh-connecting-overlay')).toBeNull();

    sshStates[paneId] = undefined;
  });
});

describe('SSH-OVERLAY-REACT-004: overlay ignores state for a different paneId', () => {
  it('does not show overlay when sshStates is set for a different pane', async () => {
    const paneId = 'react-test-pane-4';
    const otherPaneId = 'react-test-pane-other';
    const { container, instance } = await mountPane({ paneId });
    instances.push(instance);

    applySshStateChanged({ paneId: otherPaneId, state: { type: 'connecting' } });
    await settle();

    expect(container.querySelector('.ssh-connecting-overlay')).toBeNull();

    sshStates[otherPaneId] = undefined;
  });
});

// ---------------------------------------------------------------------------
// LIGA — Font ligature CSS declarations on .terminal-pane__viewport
//
// The span-per-cell DOM model fragments the CSS shaping context at every span
// boundary, so ligatures (e.g. -> => // fi) cannot form in the current renderer
// even with font-feature-settings enabled. The declarations are present because
// they will activate automatically when run-merging groups adjacent same-style
// cells into contiguous text nodes.
//
// jsdom does not compute styles from Svelte <style> blocks — we verify the
// declarations are present in the source file (same pattern as P-OPT-3).
// ---------------------------------------------------------------------------

describe('LIGA-CSS-001: font-feature-settings declared on .terminal-pane__viewport', () => {
  it('viewport CSS block contains font-feature-settings with liga and calt', async () => {
    const fs = await import('fs');
    const path = await import('path');
    const filePath = path.resolve(process.cwd(), 'src/lib/components/TerminalPane.svelte');
    const source = fs.readFileSync(filePath, 'utf-8');

    const viewportStart = source.indexOf('terminal-pane__viewport)');
    const viewportEnd = source.indexOf('terminal-pane__row)', viewportStart + 1);
    const viewportBlock = source.slice(viewportStart, viewportEnd);

    expect(viewportStart).toBeGreaterThan(-1);
    expect(viewportEnd).toBeGreaterThan(viewportStart);
    expect(viewportBlock).toContain('font-feature-settings');
    expect(viewportBlock).toMatch(/'liga'/);
    expect(viewportBlock).toMatch(/'calt'/);
  });
});

describe('LIGA-CSS-002: font-variant-ligatures declared on .terminal-pane__viewport', () => {
  it('viewport CSS block contains font-variant-ligatures: contextual', async () => {
    const fs = await import('fs');
    const path = await import('path');
    const filePath = path.resolve(process.cwd(), 'src/lib/components/TerminalPane.svelte');
    const source = fs.readFileSync(filePath, 'utf-8');

    const viewportStart = source.indexOf('terminal-pane__viewport)');
    const viewportEnd = source.indexOf('terminal-pane__row)', viewportStart + 1);
    const viewportBlock = source.slice(viewportStart, viewportEnd);

    expect(viewportStart).toBeGreaterThan(-1);
    expect(viewportEnd).toBeGreaterThan(viewportStart);
    expect(viewportBlock).toContain('font-variant-ligatures: contextual');
  });
});
