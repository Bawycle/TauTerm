// SPDX-License-Identifier: MPL-2.0

/**
 * KeyboardShortcutRecorder component tests.
 *
 * Covered:
 *   UITCP-PREF-FN-010 — click field enters recording state
 *   UITCP-PREF-FN-011 — Escape while recording cancels and reverts
 *   UITCP-PREF-FN-012 — key capture and Enter confirmation
 *   UITCP-PREF-FN-013 — conflict detection
 *
 * Note: some state transitions require full browser key event handling.
 * Tests that require JSDOM keyboard simulation are limited; complex focus
 * interaction tests are E2E-deferred.
 */

import { describe, it, expect, vi, afterEach } from 'vitest';
import { mount, unmount, flushSync } from 'svelte';
import KeyboardShortcutRecorder from '../KeyboardShortcutRecorder.svelte';

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function mountRecorder(props: {
  value?: string;
  existingShortcuts?: Record<string, string>;
  actionId?: string;
  disabled?: boolean;
  onchange?: (s: string) => void;
}): { container: HTMLElement; instance: ReturnType<typeof mount> } {
  const container = document.createElement('div');
  document.body.appendChild(container);
  const instance = mount(KeyboardShortcutRecorder, { target: container, props });
  return { container, instance };
}

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
});

// ---------------------------------------------------------------------------
// Functional tests
// ---------------------------------------------------------------------------

describe('KeyboardShortcutRecorder: renders with initial value', () => {
  it('shows initial shortcut value in inactive state', () => {
    const { container, instance } = mountRecorder({ value: 'Ctrl+Shift+T' });
    instances.push(instance);
    expect(container.textContent).toContain('Ctrl+Shift+T');
  });
});

describe('KeyboardShortcutRecorder: renders disabled state', () => {
  it('applies disabled styling when disabled=true', () => {
    const { container, instance } = mountRecorder({ disabled: true });
    instances.push(instance);
    const field = container.querySelector('.shortcut-recorder__field');
    expect(field).not.toBeNull();
    expect(field!.classList.contains('shortcut-recorder__field--disabled')).toBe(true);
  });
});

describe('UITCP-PREF-FN-010: click field enters recording state', () => {
  it('field switches to recording state on click', () => {
    const { container, instance } = mountRecorder({ value: 'Ctrl+Shift+T' });
    instances.push(instance);
    const field = container.querySelector('.shortcut-recorder__field') as HTMLElement;
    flushSync(() => {
      field.click();
    });
    // In recording state, the placeholder text should appear
    expect(container.querySelector('.shortcut-recorder__placeholder')).not.toBeNull();
  });
});

describe('UITCP-PREF-FN-011: Escape while recording cancels and reverts', () => {
  it('pressing Escape returns to inactive state with previous value', () => {
    const { container, instance } = mountRecorder({ value: 'Ctrl+Shift+T' });
    instances.push(instance);
    const field = container.querySelector('.shortcut-recorder__field') as HTMLElement;
    // Enter recording state
    flushSync(() => {
      field.click();
    });
    expect(container.querySelector('.shortcut-recorder__placeholder')).not.toBeNull();
    // Press Escape
    flushSync(() => {
      field.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', bubbles: true }));
    });
    // Should return to inactive — original value visible
    expect(container.querySelector('.shortcut-recorder__value')).not.toBeNull();
    expect(container.textContent).toContain('Ctrl+Shift+T');
  });
});

describe('UITCP-PREF-FN-013: conflict detection', () => {
  it('shows conflict CSS class when shortcut conflicts with existing binding', () => {
    const existingShortcuts = { new_tab: 'Ctrl+Shift+T', close_tab: 'Ctrl+Shift+W' };
    const { container, instance } = mountRecorder({
      value: 'Ctrl+Shift+N',
      existingShortcuts,
      actionId: 'search',
      onchange: vi.fn(),
    });
    instances.push(instance);
    const field = container.querySelector('.shortcut-recorder__field') as HTMLElement;
    // Enter recording state
    flushSync(() => {
      field.click();
    });
    // Simulate pressing Ctrl+Shift+T (conflicts with new_tab)
    flushSync(() => {
      field.dispatchEvent(
        new KeyboardEvent('keydown', {
          key: 'T',
          ctrlKey: true,
          shiftKey: true,
          bubbles: true,
        }),
      );
    });
    // Should show conflict state
    expect(container.querySelector('.shortcut-recorder__field--conflict')).not.toBeNull();
    // Should show conflict message
    expect(container.querySelector('.shortcut-recorder__conflict-message')).not.toBeNull();
    expect(container.textContent).toContain('new_tab');
  });
});

describe('UITCP-PREF-FN-012: key capture and Enter confirmation', () => {
  it('capturing a non-conflicting key and pressing Enter calls onchange', () => {
    const onchange = vi.fn();
    const existingShortcuts = { new_tab: 'Ctrl+Shift+T' };
    const { container, instance } = mountRecorder({
      value: 'Ctrl+Shift+N',
      existingShortcuts,
      actionId: 'my_action',
      onchange,
    });
    instances.push(instance);
    const field = container.querySelector('.shortcut-recorder__field') as HTMLElement;
    // Enter recording
    flushSync(() => {
      field.click();
    });
    // Capture Ctrl+Shift+X (no conflict)
    flushSync(() => {
      field.dispatchEvent(
        new KeyboardEvent('keydown', {
          key: 'X',
          ctrlKey: true,
          shiftKey: true,
          bubbles: true,
        }),
      );
    });
    // Should be in captured state
    expect(container.querySelector('.shortcut-recorder__field--captured')).not.toBeNull();
    // Confirm with Enter — note: state is now 'captured', handleConfirmKeydown handles Enter
    flushSync(() => {
      field.dispatchEvent(new KeyboardEvent('keydown', { key: 'Enter', bubbles: true }));
    });
    expect(onchange).toHaveBeenCalledWith('Ctrl+Shift+X');
  });
});

describe('KeyboardShortcutRecorder: accessible field attributes', () => {
  it('field has role="textbox" and aria-label', () => {
    const { container, instance } = mountRecorder({ value: 'Ctrl+,' });
    instances.push(instance);
    const field = container.querySelector('[role="textbox"]');
    expect(field).not.toBeNull();
    expect(field?.getAttribute('aria-label')).toBeTruthy();
  });
});
