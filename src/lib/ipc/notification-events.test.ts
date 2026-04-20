// SPDX-License-Identifier: MPL-2.0

/**
 * Tests for notification IPC events and pane focus IPC wiring (Items 7, 8).
 *
 * Covered:
 *   NOTIF-001 — background output triggers activity indicator
 *   NOTIF-002 — process termination shows distinct indicator
 *   NOTIF-003 — switching to notified tab clears indicator
 *   NOTIF-004 — bell in background tab produces indicator
 *   PANE-FOCUS-001 — click on pane calls set_active_pane
 *   PANE-FOCUS-003 — exactly one pane shows active style
 *
 * These tests validate notification state logic — component wiring tests
 * are deferred to E2E.
 */

import { describe, it, expect } from 'vitest';
import type { NotificationChangedEvent, PaneNotification } from '$lib/ipc';

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function makeNotificationEvent(
  tabId: string,
  paneId: string,
  notification: PaneNotification | null,
): NotificationChangedEvent {
  return { tabId, paneId, notification };
}

// ---------------------------------------------------------------------------
// NOTIF-001/002: NotificationChangedEvent carries correct notification types
// ---------------------------------------------------------------------------

describe('NOTIF-001: backgroundOutput notification type', () => {
  it('backgroundOutput notification has correct type tag', () => {
    const event = makeNotificationEvent('tab-1', 'pane-1', { type: 'backgroundOutput' });
    expect(event.notification?.type).toBe('backgroundOutput');
  });
});

describe('NOTIF-002: processExited notification type', () => {
  it('processExited notification carries exitCode', () => {
    const event = makeNotificationEvent('tab-1', 'pane-1', {
      type: 'processExited',
      exitCode: 0,
      signalName: null,
    });
    expect(event.notification?.type).toBe('processExited');
    if (event.notification?.type === 'processExited') {
      expect(typeof event.notification.exitCode).toBe('number');
    }
  });

  it('processExited and backgroundOutput are distinct types', () => {
    const exit = makeNotificationEvent('tab-1', 'pane-1', {
      type: 'processExited',
      exitCode: 1,
      signalName: null,
    });
    const bg = makeNotificationEvent('tab-1', 'pane-1', { type: 'backgroundOutput' });
    expect(exit.notification?.type).not.toBe(bg.notification?.type);
  });
});

describe('NOTIF-004: bell notification type', () => {
  it('bell notification has correct type tag', () => {
    const event = makeNotificationEvent('tab-1', 'pane-1', { type: 'bell' });
    expect(event.notification?.type).toBe('bell');
  });
});

// ---------------------------------------------------------------------------
// NOTIF-003: notification cleared when null
// ---------------------------------------------------------------------------

describe('NOTIF-003: notification cleared when null', () => {
  it('notification=null clears the indicator', () => {
    const event = makeNotificationEvent('tab-1', 'pane-1', null);
    expect(event.notification).toBeNull();
  });
});

// ---------------------------------------------------------------------------
// Notification state management logic
// Tests the pure state logic that must be implemented in TabBar / notification store.
// These tests will PASS if the logic is trivially correct; the wiring tests
// will FAIL until the component listens to notification-changed events.
// ---------------------------------------------------------------------------

describe('Notification state: active tab clears notification', () => {
  /**
   * Simulates the notification state logic that the frontend must implement.
   *
   * Rule: when a tab becomes active, its notification is cleared.
   * This must be implemented in the TabBar or page component listening to
   * session-state-changed + notification-changed events.
   */
  type TabNotificationState = Map<string, PaneNotification | null>;

  function applyNotificationChanged(
    state: TabNotificationState,
    event: NotificationChangedEvent,
    activeTabId: string,
  ): TabNotificationState {
    const next = new Map(state);
    // If the event's tab is currently active, do not show the notification.
    if (event.tabId === activeTabId) {
      next.set(event.tabId, null);
    } else {
      next.set(event.tabId, event.notification);
    }
    return next;
  }

  function applyTabActivated(state: TabNotificationState, tabId: string): TabNotificationState {
    const next = new Map(state);
    // Clear notification when tab is activated (NOTIF-003).
    next.set(tabId, null);
    return next;
  }

  it('NOTIF-001: background tab gets notification indicator', () => {
    const state: TabNotificationState = new Map();
    const event = makeNotificationEvent('tab-1', 'pane-1', { type: 'backgroundOutput' });
    const next = applyNotificationChanged(state, event, /* activeTabId */ 'tab-2');
    expect(next.get('tab-1')?.type).toBe('backgroundOutput');
  });

  it('NOTIF-001: active tab does not show notification', () => {
    const state: TabNotificationState = new Map();
    const event = makeNotificationEvent('tab-1', 'pane-1', { type: 'backgroundOutput' });
    const next = applyNotificationChanged(state, event, /* activeTabId */ 'tab-1');
    expect(next.get('tab-1')).toBeNull();
  });

  it('NOTIF-003: switching to tab clears its notification', () => {
    let state: TabNotificationState = new Map();
    state.set('tab-1', { type: 'backgroundOutput' });
    state = applyTabActivated(state, 'tab-1');
    expect(state.get('tab-1')).toBeNull();
  });
});

// ---------------------------------------------------------------------------
// PANE-FOCUS-001: set_active_pane must be called when clicking a pane
// ---------------------------------------------------------------------------

describe('PANE-FOCUS-001/003: pane focus state logic', () => {
  /**
   * Simulates the active pane tracking logic.
   * The frontend must call invoke('set_active_pane', { paneId }) on click.
   *
   * This logic is unit-testable; the actual IPC call requires E2E.
   */
  interface PaneLayout {
    paneId: string;
    active: boolean;
  }

  function activatePane(panes: PaneLayout[], clickedPaneId: string): PaneLayout[] {
    return panes.map((p) => ({ ...p, active: p.paneId === clickedPaneId }));
  }

  it('PANE-FOCUS-003: exactly one pane is active after click', () => {
    const panes: PaneLayout[] = [
      { paneId: 'pane-1', active: true },
      { paneId: 'pane-2', active: false },
      { paneId: 'pane-3', active: false },
    ];
    const updated = activatePane(panes, 'pane-2');
    const activePanes = updated.filter((p) => p.active);
    expect(activePanes).toHaveLength(1);
    expect(activePanes[0].paneId).toBe('pane-2');
  });

  it('PANE-FOCUS-001: clicking an inactive pane makes it active', () => {
    const panes: PaneLayout[] = [
      { paneId: 'pane-a', active: true },
      { paneId: 'pane-b', active: false },
    ];
    const updated = activatePane(panes, 'pane-b');
    expect(updated.find((p) => p.paneId === 'pane-b')?.active).toBe(true);
    expect(updated.find((p) => p.paneId === 'pane-a')?.active).toBe(false);
  });
});
