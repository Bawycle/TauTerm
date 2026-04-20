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

import type { TabId, PaneId, PaneNotification, NotificationChangedEvent } from '$lib/ipc';

// ---------------------------------------------------------------------------
// Reactive state — module-level singleton
// ---------------------------------------------------------------------------

/**
 * Per-tab unread notification badge.
 * A tab appears in this map when it has at least one unread notification.
 */
export const tabNotifications = $state<Map<TabId, boolean>>(new Map());

/**
 * Per-pane notification state.
 * Stores the most recent PaneNotification per pane, sourced from
 * NotificationChangedEvent. Used by TabBar to render activity indicators
 * (bell, background output, process exit) on tab items.
 */
export const paneNotifications = $state<Map<PaneId, PaneNotification>>(new Map());

/**
 * Get the current notification for a pane, or null if none.
 */
export function getPaneNotification(paneId: PaneId): PaneNotification | null {
  return paneNotifications.get(paneId) ?? null;
}

/**
 * Set of PaneIds whose PTY process has exited with a non-zero code or signal.
 * A pane absent from this set is either running or exited cleanly (auto-closed).
 * Used by FS-PTY-005/006: terminated banner display.
 * NOT used for FS-PTY-008 close confirmation (use hasForegroundProcess IPC instead).
 *
 * Implementation note: uses a plain Set + a primitive $state version counter rather
 * than $state<Set> directly.  Svelte 5's Set proxy tracking can silently drop
 * reactive subscriptions in production builds (compiler optimisation); a $state<number>
 * counter is a simpler, battle-tested signal that propagates reliably in all builds.
 * The `has()` method reads the counter to register the reactive dependency.
 */
const _terminatedPanesSet = new Set<PaneId>();
let _terminatedPanesVersion = $state(0);

export const terminatedPanes = {
  has(paneId: PaneId): boolean {
    // Reading _terminatedPanesVersion creates a reactive dependency so that
    // Svelte 5 components re-evaluate this expression when the Set changes.
    return _terminatedPanesVersion >= 0 && _terminatedPanesSet.has(paneId);
  },
  add(paneId: PaneId): void {
    _terminatedPanesSet.add(paneId);
    _terminatedPanesVersion++;
  },
  delete(paneId: PaneId): void {
    _terminatedPanesSet.delete(paneId);
    _terminatedPanesVersion++;
  },
  clear(): void {
    _terminatedPanesSet.clear();
    _terminatedPanesVersion++;
  },
  get size(): number {
    return _terminatedPanesVersion >= 0 ? _terminatedPanesSet.size : 0;
  },
};

// ---------------------------------------------------------------------------
// Action type — returned by applyNotificationChanged (FS-PTY-005)
// ---------------------------------------------------------------------------

/**
 * Action to perform after processing a NotificationChangedEvent.
 *
 * - `autoClose`: clean exit (exitCode 0, no signal) — caller must close the pane.
 * - `null`: no structural action needed (banner displayed, or non-exit notification).
 */
export type NotificationAction = { type: 'autoClose'; paneId: PaneId } | null;

// ---------------------------------------------------------------------------
// Updaters — called from event handlers
// ---------------------------------------------------------------------------

/**
 * Apply a NotificationChangedEvent to the notification and terminated-pane state.
 *
 * Returns a NotificationAction:
 *   - { type: 'autoClose', paneId } when the process exited cleanly (FS-PTY-005).
 *     The pane is NOT added to terminatedPanes — it should be closed immediately.
 *   - null in all other cases.
 *
 * Mirrors the logic previously in TerminalView.updatePaneNotification(), extended
 * to support auto-close for clean exits.
 */
export function applyNotificationChanged(ev: NotificationChangedEvent): NotificationAction {
  if (ev.notification !== null) {
    tabNotifications.set(ev.tabId, true);
    paneNotifications.set(ev.paneId, ev.notification);
  } else {
    tabNotifications.delete(ev.tabId);
    paneNotifications.delete(ev.paneId);
  }

  // FS-PTY-005: clean exit → auto-close, no banner.
  if (ev.notification?.type === 'processExited') {
    const { exitCode, signalName } = ev.notification;
    if (exitCode === 0 && signalName === null) {
      // Clean exit: do not add to terminatedPanes; return auto-close action.
      return { type: 'autoClose', paneId: ev.paneId };
    }
    // Non-zero exit or signal → show terminated banner.
    terminatedPanes.add(ev.paneId);
  } else if (ev.notification === null) {
    // Notification cleared — pane may have been restarted.
    terminatedPanes.delete(ev.paneId);
  }

  return null;
}

/**
 * Clear the notification badge for a tab (called when the user switches to it).
 */
export function clearTabNotification(tabId: TabId): void {
  tabNotifications.delete(tabId);
}

/**
 * Clear the per-pane notification (called when a pane is closed or restarted).
 */
export function clearPaneNotification(paneId: PaneId): void {
  paneNotifications.delete(paneId);
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
