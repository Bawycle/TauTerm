// SPDX-License-Identifier: MPL-2.0

/**
 * terminatedPanes Set management — close confirmation logic tests (FS-PTY-005/006/008).
 *
 * This file mirrors the pure logic of notifications.svelte.ts using a local
 * copy of the update function. It does not import the Svelte module directly
 * to avoid $state reactive context issues in the test runner.
 *
 * FS-PTY-005 (updated behaviour):
 *   - Clean exit (exitCode === 0, signalName === null) → auto-close, NOT added
 *     to terminatedPanes.
 *   - Non-zero exit or signal termination → added to terminatedPanes (banner shown).
 *
 * Covered:
 *   TEST-SPRINT-007a — processExited with non-zero exit adds paneId to terminatedPanes
 *   TEST-SPRINT-007a — processExited with exitCode 0 does NOT add paneId (auto-close)
 *   TEST-SPRINT-007b — notification null clears paneId from terminatedPanes
 *   TEST-SPRINT-007c — other notification types do not add to terminatedPanes
 *   TEST-SPRINT-007d — isPaneProcessActive: pane absent from set → true (active)
 *   TEST-SPRINT-007e — isPaneProcessActive: pane present in set → false (terminated)
 *   TEST-SPRINT-007f — multiple panes: adding one does not affect others
 *   TEST-SPRINT-007g — restarted pane (null notification) is removed from set
 */

import { describe, it, expect } from 'vitest';
import type { NotificationChangedEvent } from '$lib/ipc/types';

// ---------------------------------------------------------------------------
// Mirror of notifications.svelte.ts terminatedPanes logic (FS-PTY-005)
// ---------------------------------------------------------------------------

/**
 * Returns the updated terminatedPanes set and whether an auto-close is needed.
 * Clean exit (exitCode 0, no signal) → autoClose: true, pane NOT in set.
 * Non-zero exit or signal → autoClose: false, pane added to set.
 */
function updatePaneNotification(
  terminatedPanes: Set<string>,
  ev: Pick<NotificationChangedEvent, 'paneId' | 'notification'>,
): { set: Set<string>; autoClose: boolean } {
  const next = new Set(terminatedPanes);
  let autoClose = false;

  if (ev.notification?.type === 'processExited') {
    const { exitCode, signalName } = ev.notification;
    if (exitCode === 0 && signalName === null) {
      // Clean exit: auto-close, no banner, NOT added to terminatedPanes.
      autoClose = true;
    } else {
      // Non-zero exit or signal: show banner.
      next.add(ev.paneId);
    }
  } else if (ev.notification === null) {
    // Notification cleared — pane may have been restarted.
    next.delete(ev.paneId);
  }
  // 'bell' and 'backgroundOutput' do not affect terminatedPanes.
  return { set: next, autoClose };
}

function isPaneProcessActive(terminatedPanes: Set<string>, paneId: string): boolean {
  return !terminatedPanes.has(paneId);
}

// ---------------------------------------------------------------------------
// TEST-SPRINT-007a — processExited notification behaviour
// ---------------------------------------------------------------------------

describe('TEST-SPRINT-007a: processExited with non-zero exit adds paneId to terminatedPanes', () => {
  it('adds paneId when exit code is non-zero', () => {
    // TEST-SPRINT-007a: exit code 1 must mark pane as terminated (banner)
    const set = new Set<string>();
    const ev: Pick<NotificationChangedEvent, 'paneId' | 'notification'> = {
      paneId: 'pane-1',
      notification: { type: 'processExited', exitCode: 1, signalName: null },
    };
    const { set: next, autoClose } = updatePaneNotification(set, ev);
    expect(next.has('pane-1')).toBe(true);
    expect(autoClose).toBe(false);
  });

  it('adds paneId for exit code 127', () => {
    // TEST-SPRINT-007a: non-zero exit codes must mark pane as terminated
    const set = new Set<string>();
    const ev: Pick<NotificationChangedEvent, 'paneId' | 'notification'> = {
      paneId: 'pane-2',
      notification: { type: 'processExited', exitCode: 127, signalName: null },
    };
    const { set: next, autoClose } = updatePaneNotification(set, ev);
    expect(next.has('pane-2')).toBe(true);
    expect(autoClose).toBe(false);
  });

  it('adds paneId when terminated by a signal (SIGKILL)', () => {
    // TEST-SPRINT-007a: signal termination must mark pane as terminated
    const set = new Set<string>();
    const ev: Pick<NotificationChangedEvent, 'paneId' | 'notification'> = {
      paneId: 'pane-3',
      notification: { type: 'processExited', exitCode: null, signalName: 'SIGKILL' },
    };
    const { set: next, autoClose } = updatePaneNotification(set, ev);
    expect(next.has('pane-3')).toBe(true);
    expect(autoClose).toBe(false);
  });
});

describe('TEST-SPRINT-007a: clean exit (exitCode 0) triggers auto-close, NOT added to terminatedPanes', () => {
  it('does NOT add paneId when exitCode is 0 and signalName is null', () => {
    // FS-PTY-005: clean exit → auto-close, no banner
    const set = new Set<string>();
    const ev: Pick<NotificationChangedEvent, 'paneId' | 'notification'> = {
      paneId: 'pane-clean',
      notification: { type: 'processExited', exitCode: 0, signalName: null },
    };
    const { set: next, autoClose } = updatePaneNotification(set, ev);
    expect(next.has('pane-clean')).toBe(false);
    expect(autoClose).toBe(true);
  });

  it('does not mutate the original set', () => {
    // TEST-SPRINT-007a: immutability check (mirrors Svelte $state semantics)
    const set = new Set<string>();
    const ev: Pick<NotificationChangedEvent, 'paneId' | 'notification'> = {
      paneId: 'pane-3',
      notification: { type: 'processExited', exitCode: 1, signalName: null },
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
    const { set: next } = updatePaneNotification(set, ev);
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
    const { set: next } = updatePaneNotification(set, ev);
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
    const { set: next } = updatePaneNotification(set, ev);
    expect(next.has('pane-1')).toBe(false);
  });

  it('backgroundOutput notification does not add paneId', () => {
    // TEST-SPRINT-007c
    const set = new Set<string>();
    const ev: Pick<NotificationChangedEvent, 'paneId' | 'notification'> = {
      paneId: 'pane-1',
      notification: { type: 'backgroundOutput' },
    };
    const { set: next } = updatePaneNotification(set, ev);
    expect(next.has('pane-1')).toBe(false);
  });

  it('bell notification does not remove pre-existing terminated pane', () => {
    // TEST-SPRINT-007c: bell on an already-terminated pane must not clear it
    const set = new Set(['pane-1']);
    const ev: Pick<NotificationChangedEvent, 'paneId' | 'notification'> = {
      paneId: 'pane-1',
      notification: { type: 'bell' },
    };
    const { set: next } = updatePaneNotification(set, ev);
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
  it('clean exit of one pane (auto-close) does not affect another', () => {
    // TEST-SPRINT-007f: clean exit → auto-close, not in terminatedPanes
    let set = new Set<string>();
    let autoClose: boolean;
    ({ set, autoClose } = updatePaneNotification(set, {
      paneId: 'pane-1',
      notification: { type: 'processExited', exitCode: 0, signalName: null },
    }));
    // pane-1 triggers auto-close, NOT in terminatedPanes
    expect(autoClose).toBe(true);
    expect(isPaneProcessActive(set, 'pane-1')).toBe(true);
    expect(isPaneProcessActive(set, 'pane-2')).toBe(true);
  });

  it('non-zero exit of one pane does not affect another', () => {
    // TEST-SPRINT-007f: non-zero exit → banner, in terminatedPanes
    let set = new Set<string>();
    ({ set } = updatePaneNotification(set, {
      paneId: 'pane-1',
      notification: { type: 'processExited', exitCode: 1, signalName: null },
    }));
    expect(isPaneProcessActive(set, 'pane-1')).toBe(false);
    expect(isPaneProcessActive(set, 'pane-2')).toBe(true);
  });

  it('two panes can terminate with non-zero exit independently', () => {
    // TEST-SPRINT-007f
    let set = new Set<string>();
    ({ set } = updatePaneNotification(set, {
      paneId: 'pane-1',
      notification: { type: 'processExited', exitCode: 1, signalName: null },
    }));
    ({ set } = updatePaneNotification(set, {
      paneId: 'pane-2',
      notification: { type: 'processExited', exitCode: 2, signalName: null },
    }));
    expect(set.size).toBe(2);
    expect(isPaneProcessActive(set, 'pane-1')).toBe(false);
    expect(isPaneProcessActive(set, 'pane-2')).toBe(false);
  });
});

// ---------------------------------------------------------------------------
// TEST-SPRINT-007g — restarted pane is removed from set
// ---------------------------------------------------------------------------

describe('TEST-SPRINT-007g: restarted pane (null notification) is cleared from terminatedPanes', () => {
  it('null notification after processExited (non-zero) removes pane from set', () => {
    // TEST-SPRINT-007g: sequence — non-zero exit → banner → restart → null notification
    let set = new Set<string>();
    // Process exits with non-zero code
    ({ set } = updatePaneNotification(set, {
      paneId: 'pane-1',
      notification: { type: 'processExited', exitCode: 1, signalName: null },
    }));
    expect(isPaneProcessActive(set, 'pane-1')).toBe(false);
    // Process restarts, notification cleared
    ({ set } = updatePaneNotification(set, {
      paneId: 'pane-1',
      notification: null,
    }));
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
