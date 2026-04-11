// SPDX-License-Identifier: MPL-2.0

/**
 * Typed wrappers for all IPC event subscriptions (listen() calls).
 *
 * Each function returns an unsubscribe callback (Promise<() => void>) that
 * must be called on component destroy to prevent memory leaks.
 *
 * Event names mirror the Rust backend emit calls in src-tauri/src/events/.
 */

import { listen } from '@tauri-apps/api/event';
import type {
  SessionStateChangedEvent,
  SshStateChangedEvent,
  SshWarningEvent,
  SshReconnectedEvent,
  HostKeyPromptEvent,
  CredentialPromptEvent,
  PassphrasePromptEvent,
  NotificationChangedEvent,
  ModeStateChangedEvent,
  ScreenUpdateEvent,
  ScrollPositionChangedEvent,
  CursorStyleChangedEvent,
  BellTriggeredEvent,
  FullscreenStateChangedEvent,
} from './types';

// ---------------------------------------------------------------------------
// Session events
// ---------------------------------------------------------------------------

export function onSessionStateChanged(
  handler: (event: SessionStateChangedEvent) => void,
): Promise<() => void> {
  return listen<SessionStateChangedEvent>('session-state-changed', (e) => handler(e.payload));
}

// ---------------------------------------------------------------------------
// SSH events
// ---------------------------------------------------------------------------

export function onSshStateChanged(
  handler: (event: SshStateChangedEvent) => void,
): Promise<() => void> {
  return listen<SshStateChangedEvent>('ssh-state-changed', (e) => handler(e.payload));
}

export function onSshWarning(handler: (event: SshWarningEvent) => void): Promise<() => void> {
  return listen<SshWarningEvent>('ssh-warning', (e) => handler(e.payload));
}

export function onSshReconnected(
  handler: (event: SshReconnectedEvent) => void,
): Promise<() => void> {
  return listen<SshReconnectedEvent>('ssh-reconnected', (e) => handler(e.payload));
}

export function onHostKeyPrompt(handler: (event: HostKeyPromptEvent) => void): Promise<() => void> {
  return listen<HostKeyPromptEvent>('host-key-prompt', (e) => handler(e.payload));
}

export function onCredentialPrompt(
  handler: (event: CredentialPromptEvent) => void,
): Promise<() => void> {
  return listen<CredentialPromptEvent>('credential-prompt', (e) => handler(e.payload));
}

export function onPassphrasePrompt(
  handler: (event: PassphrasePromptEvent) => void,
): Promise<() => void> {
  return listen<PassphrasePromptEvent>('passphrase-prompt', (e) => handler(e.payload));
}

// ---------------------------------------------------------------------------
// Notification events
// ---------------------------------------------------------------------------

export function onNotificationChanged(
  handler: (event: NotificationChangedEvent) => void,
): Promise<() => void> {
  return listen<NotificationChangedEvent>('notification-changed', (e) => handler(e.payload));
}

// ---------------------------------------------------------------------------
// Terminal mode events
// ---------------------------------------------------------------------------

export function onModeStateChanged(
  handler: (event: ModeStateChangedEvent) => void,
): Promise<() => void> {
  return listen<ModeStateChangedEvent>('mode-state-changed', (e) => handler(e.payload));
}

// ---------------------------------------------------------------------------
// Screen update events
// ---------------------------------------------------------------------------

export function onScreenUpdate(handler: (event: ScreenUpdateEvent) => void): Promise<() => void> {
  return listen<ScreenUpdateEvent>('screen-update', (e) => handler(e.payload));
}

export function onScrollPositionChanged(
  handler: (event: ScrollPositionChangedEvent) => void,
): Promise<() => void> {
  return listen<ScrollPositionChangedEvent>('scroll-position-changed', (e) => handler(e.payload));
}

export function onCursorStyleChanged(
  handler: (event: CursorStyleChangedEvent) => void,
): Promise<() => void> {
  return listen<CursorStyleChangedEvent>('cursor-style-changed', (e) => handler(e.payload));
}

export function onBellTriggered(handler: (event: BellTriggeredEvent) => void): Promise<() => void> {
  return listen<BellTriggeredEvent>('bell-triggered', (e) => handler(e.payload));
}

// ---------------------------------------------------------------------------
// Fullscreen events
// ---------------------------------------------------------------------------

export function onFullscreenStateChanged(
  handler: (event: FullscreenStateChangedEvent) => void,
): Promise<() => void> {
  return listen<FullscreenStateChangedEvent>('fullscreen-state-changed', (e) => handler(e.payload));
}
