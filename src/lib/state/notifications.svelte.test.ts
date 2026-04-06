// SPDX-License-Identifier: MPL-2.0

/**
 * Tests for src/lib/state/notifications.svelte.ts
 *
 * Covers:
 *   - applyNotificationChanged (bell, backgroundOutput, processExited, null)
 *   - clearTabNotification
 *   - isPaneProcessActive
 */

import { describe, it, expect, beforeEach } from 'vitest';

// Reset module state between describe blocks by re-importing.
// Module-level $state persists within a test run, so we clear explicitly.

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

  it('adds pane to terminatedPanes on processExited', async () => {
    const { applyNotificationChanged, terminatedPanes } = await import('./notifications.svelte');

    applyNotificationChanged({
      tabId: 'tab-1',
      paneId: 'pane-exit',
      notification: { type: 'processExited', exitCode: 0 },
    });

    expect(terminatedPanes.has('pane-exit')).toBe(true);
  });

  it('clears both badge and terminated state on null notification', async () => {
    const { applyNotificationChanged, tabNotifications, terminatedPanes } = await import(
      './notifications.svelte'
    );

    // First set terminated
    applyNotificationChanged({
      tabId: 'tab-1',
      paneId: 'pane-1',
      notification: { type: 'processExited', exitCode: 1 },
    });

    // Then clear
    applyNotificationChanged({
      tabId: 'tab-1',
      paneId: 'pane-1',
      notification: null,
    });

    expect(tabNotifications.has('tab-1')).toBe(false);
    expect(terminatedPanes.has('pane-1')).toBe(false);
  });
});

describe('notifications.svelte.ts — clearTabNotification', () => {
  it('removes the tab from the notification map', async () => {
    const { applyNotificationChanged, clearTabNotification, tabNotifications } = await import(
      './notifications.svelte'
    );
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

describe('notifications.svelte.ts — isPaneProcessActive', () => {
  it('returns true when pane is not in terminatedPanes', async () => {
    const { isPaneProcessActive, terminatedPanes } = await import('./notifications.svelte');
    terminatedPanes.clear();

    expect(isPaneProcessActive('any-pane')).toBe(true);
  });

  it('returns false when pane is in terminatedPanes', async () => {
    const { applyNotificationChanged, isPaneProcessActive } = await import(
      './notifications.svelte'
    );

    applyNotificationChanged({
      tabId: 'tab-1',
      paneId: 'dead-pane',
      notification: { type: 'processExited', exitCode: 127 },
    });

    expect(isPaneProcessActive('dead-pane')).toBe(false);
  });
});
