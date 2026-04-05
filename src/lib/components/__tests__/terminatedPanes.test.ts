// SPDX-License-Identifier: MPL-2.0

/**
 * terminatedPanes Set management — close confirmation logic tests (FS-PTY-008).
 *
 * Covered:
 *   TEST-SPRINT-007a — processExited notification adds paneId to terminatedPanes
 *   TEST-SPRINT-007b — notification null clears paneId from terminatedPanes
 *   TEST-SPRINT-007c — other notification types do not add to terminatedPanes
 *   TEST-SPRINT-007d — isPaneProcessActive: pane absent from set → true (active)
 *   TEST-SPRINT-007e — isPaneProcessActive: pane present in set → false (terminated)
 *   TEST-SPRINT-007f — multiple panes: adding one does not affect others
 *   TEST-SPRINT-007g — restarted pane (null notification) is removed from set
 *
 * The `terminatedPanes` Set and `updatePaneNotification` function live in
 * TerminalView.svelte and are not exported. These tests mirror the pure logic.
 *
 * DOM integration (close confirmation dialog shown when isPaneProcessActive is true)
 * is deferred to E2E.
 */

import { describe, it, expect } from 'vitest';
import type { PaneNotification, NotificationChangedEvent } from '$lib/ipc/types';

// ---------------------------------------------------------------------------
// Mirror of TerminalView.svelte terminatedPanes logic
// ---------------------------------------------------------------------------

function updatePaneNotification(
  terminatedPanes: Set<string>,
  ev: Pick<NotificationChangedEvent, 'paneId' | 'notification'>,
): Set<string> {
  const next = new Set(terminatedPanes);
  if (ev.notification?.type === 'processExited') {
    next.add(ev.paneId);
  } else if (ev.notification === null) {
    // Notification cleared — pane may have been restarted.
    next.delete(ev.paneId);
  }
  // 'bell' and 'backgroundOutput' do not affect terminatedPanes.
  return next;
}

function isPaneProcessActive(terminatedPanes: Set<string>, paneId: string): boolean {
  return !terminatedPanes.has(paneId);
}

// ---------------------------------------------------------------------------
// TEST-SPRINT-007a — processExited notification adds paneId
// ---------------------------------------------------------------------------

describe('TEST-SPRINT-007a: processExited notification adds paneId to terminatedPanes', () => {
  it('adds paneId when notification type is processExited', () => {
    // TEST-SPRINT-007a
    const set = new Set<string>();
    const ev: Pick<NotificationChangedEvent, 'paneId' | 'notification'> = {
      paneId: 'pane-1',
      notification: { type: 'processExited', exitCode: 0 },
    };
    const next = updatePaneNotification(set, ev);
    expect(next.has('pane-1')).toBe(true);
  });

  it('adds paneId regardless of exit code', () => {
    // TEST-SPRINT-007a: exit code 1 must also mark pane as terminated
    const set = new Set<string>();
    const ev: Pick<NotificationChangedEvent, 'paneId' | 'notification'> = {
      paneId: 'pane-2',
      notification: { type: 'processExited', exitCode: 127 },
    };
    const next = updatePaneNotification(set, ev);
    expect(next.has('pane-2')).toBe(true);
  });

  it('does not mutate the original set', () => {
    // TEST-SPRINT-007a: immutability check (mirrors Svelte $state semantics)
    const set = new Set<string>();
    const ev: Pick<NotificationChangedEvent, 'paneId' | 'notification'> = {
      paneId: 'pane-3',
      notification: { type: 'processExited', exitCode: 0 },
    };
    updatePaneNotification(set, ev);
    expect(set.has('pane-3')).toBe(false);
  });
});

// ---------------------------------------------------------------------------
// TEST-SPRINT-007b — null notification removes paneId
// ---------------------------------------------------------------------------

describe('TEST-SPRINT-007b: null notification removes paneId from terminatedPanes', () => {
  it('removes paneId when notification is null', () => {
    // TEST-SPRINT-007b: pane was restarted
    const set = new Set(['pane-1', 'pane-2']);
    const ev: Pick<NotificationChangedEvent, 'paneId' | 'notification'> = {
      paneId: 'pane-1',
      notification: null,
    };
    const next = updatePaneNotification(set, ev);
    expect(next.has('pane-1')).toBe(false);
    // pane-2 must still be present
    expect(next.has('pane-2')).toBe(true);
  });

  it('removing a pane not in set is a no-op', () => {
    // TEST-SPRINT-007b
    const set = new Set(['pane-1']);
    const ev: Pick<NotificationChangedEvent, 'paneId' | 'notification'> = {
      paneId: 'pane-99',
      notification: null,
    };
    const next = updatePaneNotification(set, ev);
    expect(next.has('pane-1')).toBe(true);
    expect(next.size).toBe(1);
  });
});

// ---------------------------------------------------------------------------
// TEST-SPRINT-007c — other notification types do not add to terminatedPanes
// ---------------------------------------------------------------------------

describe('TEST-SPRINT-007c: bell and backgroundOutput do not affect terminatedPanes', () => {
  it('bell notification does not add paneId', () => {
    // TEST-SPRINT-007c
    const set = new Set<string>();
    const ev: Pick<NotificationChangedEvent, 'paneId' | 'notification'> = {
      paneId: 'pane-1',
      notification: { type: 'bell' },
    };
    const next = updatePaneNotification(set, ev);
    expect(next.has('pane-1')).toBe(false);
  });

  it('backgroundOutput notification does not add paneId', () => {
    // TEST-SPRINT-007c
    const set = new Set<string>();
    const ev: Pick<NotificationChangedEvent, 'paneId' | 'notification'> = {
      paneId: 'pane-1',
      notification: { type: 'backgroundOutput' },
    };
    const next = updatePaneNotification(set, ev);
    expect(next.has('pane-1')).toBe(false);
  });

  it('bell notification does not remove pre-existing terminated pane', () => {
    // TEST-SPRINT-007c: bell on an already-terminated pane must not clear it
    const set = new Set(['pane-1']);
    const ev: Pick<NotificationChangedEvent, 'paneId' | 'notification'> = {
      paneId: 'pane-1',
      notification: { type: 'bell' },
    };
    const next = updatePaneNotification(set, ev);
    expect(next.has('pane-1')).toBe(true);
  });
});

// ---------------------------------------------------------------------------
// TEST-SPRINT-007d — isPaneProcessActive: absent pane → active
// ---------------------------------------------------------------------------

describe('TEST-SPRINT-007d: isPaneProcessActive returns true for absent paneId', () => {
  it('pane not in terminatedPanes is considered active', () => {
    // TEST-SPRINT-007d
    const set = new Set<string>();
    expect(isPaneProcessActive(set, 'pane-1')).toBe(true);
  });

  it('active pane remains active when another pane terminates', () => {
    // TEST-SPRINT-007d
    const set = new Set(['pane-2']);
    expect(isPaneProcessActive(set, 'pane-1')).toBe(true);
  });
});

// ---------------------------------------------------------------------------
// TEST-SPRINT-007e — isPaneProcessActive: present pane → terminated
// ---------------------------------------------------------------------------

describe('TEST-SPRINT-007e: isPaneProcessActive returns false for present paneId', () => {
  it('pane in terminatedPanes is not active', () => {
    // TEST-SPRINT-007e
    const set = new Set(['pane-1']);
    expect(isPaneProcessActive(set, 'pane-1')).toBe(false);
  });
});

// ---------------------------------------------------------------------------
// TEST-SPRINT-007f — multiple panes isolation
// ---------------------------------------------------------------------------

describe('TEST-SPRINT-007f: multiple panes do not interfere', () => {
  it('terminating one pane does not affect another', () => {
    // TEST-SPRINT-007f
    let set = new Set<string>();
    set = updatePaneNotification(set, {
      paneId: 'pane-1',
      notification: { type: 'processExited', exitCode: 0 },
    });
    expect(isPaneProcessActive(set, 'pane-1')).toBe(false);
    expect(isPaneProcessActive(set, 'pane-2')).toBe(true);
  });

  it('two panes can terminate independently', () => {
    // TEST-SPRINT-007f
    let set = new Set<string>();
    set = updatePaneNotification(set, {
      paneId: 'pane-1',
      notification: { type: 'processExited', exitCode: 0 },
    });
    set = updatePaneNotification(set, {
      paneId: 'pane-2',
      notification: { type: 'processExited', exitCode: 1 },
    });
    expect(set.size).toBe(2);
    expect(isPaneProcessActive(set, 'pane-1')).toBe(false);
    expect(isPaneProcessActive(set, 'pane-2')).toBe(false);
  });
});

// ---------------------------------------------------------------------------
// TEST-SPRINT-007g — restarted pane is removed from set
// ---------------------------------------------------------------------------

describe('TEST-SPRINT-007g: restarted pane (null notification) is cleared from terminatedPanes', () => {
  it('null notification after processExited removes pane from set', () => {
    // TEST-SPRINT-007g: sequence — exited → restarted → notification cleared
    let set = new Set<string>();
    // Process exits
    set = updatePaneNotification(set, {
      paneId: 'pane-1',
      notification: { type: 'processExited', exitCode: 0 },
    });
    expect(isPaneProcessActive(set, 'pane-1')).toBe(false);
    // Process restarts, notification cleared
    set = updatePaneNotification(set, {
      paneId: 'pane-1',
      notification: null,
    });
    expect(isPaneProcessActive(set, 'pane-1')).toBe(true);
  });
});

// ---------------------------------------------------------------------------
// E2E-deferred: DOM integration requiring Tauri runtime
// ---------------------------------------------------------------------------

describe('TEST-SPRINT-007 [E2E-deferred]: close confirmation dialog', () => {
  it.todo('close tab with active process shows confirmation dialog');
  it.todo('close pane with active process shows confirmation dialog');
  it.todo('close tab with terminated process does not show dialog');
  it.todo('confirm dialog executes close action');
  it.todo('cancel dialog aborts close action');
});
