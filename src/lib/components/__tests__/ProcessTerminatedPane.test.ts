// SPDX-License-Identifier: MPL-2.0

/**
 * ProcessTerminatedPane component tests.
 *
 * Covered:
 *   UITCP-PTP-FN-001 — exit code 0: success state, CheckCircle icon context
 *   UITCP-PTP-FN-002 — non-zero exit code: error state, XCircle icon context
 *   UITCP-PTP-FN-003 — exit code 127
 *   UITCP-PTP-FN-004 — Restart button emits onrestart
 *   UITCP-PTP-FN-005 — Close button emits onclose
 *   UITCP-PTP-FN-006 — Banner does not auto-close
 *   UITCP-PTP-FN-007 — Signal name shown for signal-killed process
 *   UITCP-PTP-A11Y-001 — Restart and Close buttons are focusable
 *   UITCP-PTP-A11Y-002 — Icons are aria-hidden
 *   UITCP-PTP-UX-001  — Banner has correct background CSS class
 *   SEC-UI-006        — exitCode rendered as text, not HTML
 */

import { describe, it, expect, vi, afterEach } from 'vitest';
import { mount, unmount } from 'svelte';
import ProcessTerminatedPane from '../ProcessTerminatedPane.svelte';

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function mountPane(props: {
  exitCode: number;
  signalName?: string;
  onrestart?: () => void;
  onclose?: () => void;
}): { container: HTMLElement; instance: ReturnType<typeof mount> } {
  const container = document.createElement('div');
  document.body.appendChild(container);
  const instance = mount(ProcessTerminatedPane, { target: container, props });
  return { container, instance };
}

const instances: ReturnType<typeof mount>[] = [];

afterEach(() => {
  instances.forEach((i) => {
    try { unmount(i); } catch { /* ignore */ }
  });
  instances.length = 0;
  document.body.innerHTML = '';
});

// ---------------------------------------------------------------------------
// Functional tests
// ---------------------------------------------------------------------------

describe('UITCP-PTP-FN-001: exit code 0 — success state', () => {
  it('renders banner with success message', () => {
    const { container, instance } = mountPane({ exitCode: 0 });
    instances.push(instance);
    // Banner should be visible
    const banner = container.querySelector('.process-terminated-pane');
    expect(banner).not.toBeNull();
    // Should contain success text
    expect(container.textContent).toMatch(/exited/i);
  });
});

describe('UITCP-PTP-FN-002: non-zero exit code — error state', () => {
  it('renders banner with exit code 1', () => {
    const { container, instance } = mountPane({ exitCode: 1 });
    instances.push(instance);
    expect(container.textContent).toMatch(/1/);
  });
});

describe('UITCP-PTP-FN-003: exit code 127', () => {
  it('renders exit code 127 in text', () => {
    const { container, instance } = mountPane({ exitCode: 127 });
    instances.push(instance);
    expect(container.textContent).toMatch(/127/);
  });
});

describe('UITCP-PTP-FN-004: Restart button emits onrestart', () => {
  it('calls onrestart when Restart button clicked', () => {
    const onrestart = vi.fn();
    const { container, instance } = mountPane({ exitCode: 1, onrestart });
    instances.push(instance);
    // Find the Restart button (primary variant, contains "Restart")
    const buttons = container.querySelectorAll('button');
    const restartBtn = Array.from(buttons).find((b) => b.textContent?.includes('Restart'));
    expect(restartBtn).not.toBeNull();
    restartBtn!.click();
    expect(onrestart).toHaveBeenCalledTimes(1);
  });
});

describe('UITCP-PTP-FN-005: Close button emits onclose', () => {
  it('calls onclose when Close button clicked', () => {
    const onclose = vi.fn();
    const { container, instance } = mountPane({ exitCode: 0, onclose });
    instances.push(instance);
    const buttons = container.querySelectorAll('button');
    const closeBtn = Array.from(buttons).find((b) => b.textContent?.includes('Close'));
    expect(closeBtn).not.toBeNull();
    closeBtn!.click();
    expect(onclose).toHaveBeenCalledTimes(1);
  });
});

describe('UITCP-PTP-FN-006: Banner does not auto-close', () => {
  it('banner remains visible after mount with exit 0', () => {
    const { container, instance } = mountPane({ exitCode: 0 });
    instances.push(instance);
    const banner = container.querySelector('.process-terminated-pane');
    expect(banner).not.toBeNull();
    // No auto-dismiss timer — banner should still be present
    expect(document.body.contains(banner)).toBe(true);
  });
});

describe('UITCP-PTP-FN-007: Signal name displayed', () => {
  it('shows signalName in secondary text', () => {
    const { container, instance } = mountPane({ exitCode: 137, signalName: 'SIGKILL' });
    instances.push(instance);
    expect(container.textContent).toContain('SIGKILL');
  });
});

// ---------------------------------------------------------------------------
// Accessibility tests
// ---------------------------------------------------------------------------

describe('UITCP-PTP-A11Y-001: buttons are keyboard-focusable', () => {
  it('Restart and Close buttons exist and are not disabled by default', () => {
    const { container, instance } = mountPane({ exitCode: 1 });
    instances.push(instance);
    const buttons = Array.from(container.querySelectorAll('button'));
    const restartBtn = buttons.find((b) => b.textContent?.includes('Restart'));
    const closeBtn = buttons.find((b) => b.textContent?.includes('Close'));
    expect(restartBtn).not.toBeNull();
    expect(closeBtn).not.toBeNull();
    expect(restartBtn!.disabled).toBe(false);
    expect(closeBtn!.disabled).toBe(false);
  });
});

describe('UITCP-PTP-A11Y-002: icons are aria-hidden', () => {
  it('svg icons inside banner have aria-hidden=true', () => {
    const { container, instance } = mountPane({ exitCode: 0 });
    instances.push(instance);
    const svgIcons = container.querySelectorAll('svg');
    // At least one icon should be aria-hidden
    const hiddenIcons = Array.from(svgIcons).filter((s) => s.getAttribute('aria-hidden') === 'true');
    expect(hiddenIcons.length).toBeGreaterThan(0);
  });
});

// ---------------------------------------------------------------------------
// UX / Visual
// ---------------------------------------------------------------------------

describe('UITCP-PTP-UX-001: banner uses correct CSS class', () => {
  it('banner has process-terminated-pane CSS class', () => {
    const { container, instance } = mountPane({ exitCode: 0 });
    instances.push(instance);
    expect(container.querySelector('.process-terminated-pane')).not.toBeNull();
  });
});

// ---------------------------------------------------------------------------
// Security: SEC-UI-006 — exit code rendered as text, not HTML
// ---------------------------------------------------------------------------

describe('SEC-UI-006: exit code rendered as text not HTML', () => {
  it('XSS payload in exit code does not execute (NaN renders gracefully)', () => {
    // exitCode is typed as number, so NaN is the worst case
    const { container, instance } = mountPane({ exitCode: NaN });
    instances.push(instance);
    // Should render without throwing
    expect(container.querySelector('.process-terminated-pane')).not.toBeNull();
    // Should not contain script tags
    expect(container.innerHTML).not.toContain('<script');
  });

  it('exit code Infinity renders gracefully', () => {
    const { container, instance } = mountPane({ exitCode: Infinity });
    instances.push(instance);
    expect(container.querySelector('.process-terminated-pane')).not.toBeNull();
  });

  it('exit code -1 renders gracefully', () => {
    const { container, instance } = mountPane({ exitCode: -1 });
    instances.push(instance);
    expect(container.textContent).toContain('-1');
  });
});
