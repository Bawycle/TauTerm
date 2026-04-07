// SPDX-License-Identifier: MPL-2.0

/**
 * Tests for src/lib/state/notifications.svelte.ts
 *
 * Covers:
 *   - applyNotificationChanged return value (NotificationAction — FS-PTY-005)
 *   - applyNotificationChanged side effects on terminatedPanes and tabNotifications
 *   - clearTabNotification
 *
 * Feature 1 (FS-PTY-005): clean exit (exitCode 0, signalName null) must
 *   - return { type: 'autoClose', paneId }
 *   - NOT add the pane to terminatedPanes
 * Non-zero exit or signal must:
 *   - return null
 *   - add the pane to terminatedPanes
 */

import { describe, it, expect, beforeEach } from 'vitest';

describe('notifications.svelte.ts — applyNotificationChanged', () => {
  beforeEach(async () => {
    const { tabNotifications, terminatedPanes } = await import('./notifications.svelte');
    tabNotifications.clear();
    terminatedPanes.clear();
  });

  it('sets tab notification badge on bell', async () => {
    const { applyNotificationChanged, tabNotifications } = await import('./notifications.svelte');

    applyNotificationChanged({
      tabId: 'tab-1',
      paneId: 'pane-1',
      notification: { type: 'bell' },
    });

    expect(tabNotifications.get('tab-1')).toBe(true);
  });

  it('sets tab notification badge on backgroundOutput', async () => {
    const { applyNotificationChanged, tabNotifications } = await import('./notifications.svelte');

    applyNotificationChanged({
      tabId: 'tab-2',
      paneId: 'pane-2',
      notification: { type: 'backgroundOutput' },
    });

    expect(tabNotifications.get('tab-2')).toBe(true);
  });

  // FS-PTY-005: clean exit → auto-close action, NOT added to terminatedPanes
  it('returns autoClose action for exitCode 0 + null signal', async () => {
    const { applyNotificationChanged, terminatedPanes } = await import('./notifications.svelte');

    const action = applyNotificationChanged({
      tabId: 'tab-1',
      paneId: 'pane-exit',
      notification: { type: 'processExited', exitCode: 0, signalName: null },
    });

    expect(action).toEqual({ type: 'autoClose', paneId: 'pane-exit' });
    expect(terminatedPanes.has('pane-exit')).toBe(false);
  });

  // FS-PTY-005/006: non-zero exit → null action, added to terminatedPanes
  it('returns null and adds pane to terminatedPanes for exitCode 1', async () => {
    const { applyNotificationChanged, terminatedPanes } = await import('./notifications.svelte');

    const action = applyNotificationChanged({
      tabId: 'tab-1',
      paneId: 'pane-fail',
      notification: { type: 'processExited', exitCode: 1, signalName: null },
    });

    expect(action).toBeNull();
    expect(terminatedPanes.has('pane-fail')).toBe(true);
  });

  // FS-PTY-005/006: signal kill → null action, added to terminatedPanes
  it('returns null and adds pane to terminatedPanes for SIGKILL', async () => {
    const { applyNotificationChanged, terminatedPanes } = await import('./notifications.svelte');

    const action = applyNotificationChanged({
      tabId: 'tab-1',
      paneId: 'pane-killed',
      notification: { type: 'processExited', exitCode: null, signalName: 'SIGKILL' },
    });

    expect(action).toBeNull();
    expect(terminatedPanes.has('pane-killed')).toBe(true);
  });

  // exitCode null + signalName null is an edge case: treat as abnormal (banner)
  it('returns null and adds pane to terminatedPanes when both exitCode and signalName are null', async () => {
    const { applyNotificationChanged, terminatedPanes } = await import('./notifications.svelte');

    const action = applyNotificationChanged({
      tabId: 'tab-1',
      paneId: 'pane-unknown',
      notification: { type: 'processExited', exitCode: null, signalName: null },
    });

    // exitCode is null (not 0), so clean-exit condition is not met → banner
    expect(action).toBeNull();
    expect(terminatedPanes.has('pane-unknown')).toBe(true);
  });

  it('clears both badge and terminated state on null notification', async () => {
    const { applyNotificationChanged, tabNotifications, terminatedPanes } =
      await import('./notifications.svelte');

    // First set terminated with non-zero exit
    applyNotificationChanged({
      tabId: 'tab-1',
      paneId: 'pane-1',
      notification: { type: 'processExited', exitCode: 1, signalName: null },
    });

    // Then clear (e.g. pane restarted)
    applyNotificationChanged({
      tabId: 'tab-1',
      paneId: 'pane-1',
      notification: null,
    });

    expect(tabNotifications.has('tab-1')).toBe(false);
    expect(terminatedPanes.has('pane-1')).toBe(false);
  });

  it('bell and backgroundOutput do not affect terminatedPanes', async () => {
    const { applyNotificationChanged, terminatedPanes } = await import('./notifications.svelte');

    applyNotificationChanged({
      tabId: 'tab-1',
      paneId: 'pane-bell',
      notification: { type: 'bell' },
    });
    applyNotificationChanged({
      tabId: 'tab-1',
      paneId: 'pane-output',
      notification: { type: 'backgroundOutput' },
    });

    expect(terminatedPanes.has('pane-bell')).toBe(false);
    expect(terminatedPanes.has('pane-output')).toBe(false);
  });
});

describe('notifications.svelte.ts — clearTabNotification', () => {
  it('removes the tab from the notification map', async () => {
    const { applyNotificationChanged, clearTabNotification, tabNotifications } =
      await import('./notifications.svelte');
    tabNotifications.clear();

    applyNotificationChanged({
      tabId: 'tab-A',
      paneId: 'pane-A',
      notification: { type: 'bell' },
    });

    clearTabNotification('tab-A');
    expect(tabNotifications.has('tab-A')).toBe(false);
  });
});
