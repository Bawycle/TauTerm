// SPDX-License-Identifier: MPL-2.0

/**
 * Reactive notification state — per-tab badge + per-pane terminated tracking.
 *
 * Tracks:
 *   - Tab-level notification badges (has-unread per tabId)
 *   - Set of panes whose PTY process has exited (processExited notification)
 *
 * Updated from notification-changed events.
 * Cleared when the user switches to a tab.
 */

import type { TabId, PaneId, NotificationChangedEvent } from '$lib/ipc/types';

// ---------------------------------------------------------------------------
// Reactive state — module-level singleton
// ---------------------------------------------------------------------------

/**
 * Per-tab unread notification badge.
 * A tab appears in this map when it has at least one unread notification.
 */
export const tabNotifications = $state<Map<TabId, boolean>>(new Map());

/**
 * Set of PaneIds whose PTY process has exited.
 * A pane absent from this set is considered to have a running process.
 * Used by FS-PTY-008 close confirmation logic.
 */
export const terminatedPanes = $state<Set<PaneId>>(new Set());

// ---------------------------------------------------------------------------
// Updaters — called from event handlers
// ---------------------------------------------------------------------------

/**
 * Apply a NotificationChangedEvent to the notification and terminated-pane state.
 * Mirrors the logic previously in TerminalView.updatePaneNotification().
 */
export function applyNotificationChanged(ev: NotificationChangedEvent): void {
  if (ev.notification !== null) {
    tabNotifications.set(ev.tabId, true);
  } else {
    tabNotifications.delete(ev.tabId);
  }

  // FS-PTY-008: track process-terminated state per pane.
  if (ev.notification?.type === 'processExited') {
    terminatedPanes.add(ev.paneId);
  } else if (ev.notification === null) {
    // Notification cleared — pane may have been restarted.
    terminatedPanes.delete(ev.paneId);
  }
}

/**
 * Clear the notification badge for a tab (called when the user switches to it).
 */
export function clearTabNotification(tabId: TabId): void {
  tabNotifications.delete(tabId);
}

// ---------------------------------------------------------------------------
// Read helpers
// ---------------------------------------------------------------------------

/**
 * Returns true if the given pane has a running process (not terminated).
 */
export function isPaneProcessActive(paneId: PaneId): boolean {
  return !terminatedPanes.has(paneId);
}
