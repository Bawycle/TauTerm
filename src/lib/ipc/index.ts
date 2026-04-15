// SPDX-License-Identifier: MPL-2.0

/**
 * Barrel module for the IPC layer.
 *
 * Re-exports generated bindings (`bindings.ts`) with an API surface that is
 * **compatible** with the existing hand-written wrappers (`commands.ts`,
 * `events.ts`, `types.ts`).  Callers keep their current import shapes:
 *
 *   import { createTab, closeTab } from '$lib/ipc';
 *   import { onScreenUpdate }      from '$lib/ipc';
 *   import type { PaneState }      from '$lib/ipc';
 *
 * The generated `commands.*` methods return a discriminated result envelope
 * (`{ status: "ok", data } | { status: "error", error }`).  The adapter
 * functions below unwrap that envelope so callers still see a plain
 * `Promise<T>` that throws a `TauTermError` on failure — matching the
 * behaviour of the hand-written wrappers.
 *
 * The generated `events.*` entries expose a Tauri `listen` callback that
 * delivers the full `Event<T>` object.  The `onXxx` adapters below unwrap
 * `.payload` so handlers receive the payload directly — again matching the
 * hand-written API.
 */

import { commands, events } from './bindings';
import type { TauTermError } from './bindings';

// Re-export the error helpers (not generated — hand-written, stays)
export { isTauTermError } from './errors';
export type { TauTermErrorCode } from './errors';

// ---------------------------------------------------------------------------
// Result-envelope unwrapper
// ---------------------------------------------------------------------------

/**
 * Unwrap the `typedError` envelope produced by Specta-generated commands.
 * On `status: "ok"` returns `data`; on `status: "error"` throws `error`.
 */
async function unwrap<T>(
  result: Promise<{ status: 'ok'; data: T } | { status: 'error'; error: TauTermError }>,
): Promise<T> {
  const r = await result;
  if (r.status === 'error') throw r.error;
  return r.data;
}

/** Same as `unwrap` but maps `null` data to `void` for commands that return nothing. */
async function unwrapVoid(
  result: Promise<{ status: 'ok'; data: null } | { status: 'error'; error: TauTermError }>,
): Promise<void> {
  const r = await result;
  if (r.status === 'error') throw r.error;
}

// ---------------------------------------------------------------------------
// Command adapters — same signatures as the hand-written commands.ts
// ---------------------------------------------------------------------------

// Session commands
export function getSessionState() {
  return unwrap(commands.getSessionState());
}

export function createTab(config: Parameters<typeof commands.createTab>[0]) {
  return unwrap(commands.createTab(config));
}

export function closeTab(tabId: Parameters<typeof commands.closeTab>[0]) {
  return unwrapVoid(commands.closeTab(tabId));
}

export function renameTab(
  tabId: Parameters<typeof commands.renameTab>[0],
  label: Parameters<typeof commands.renameTab>[1],
) {
  return unwrap(commands.renameTab(tabId, label));
}

export function reorderTab(
  tabId: Parameters<typeof commands.reorderTab>[0],
  newOrder: Parameters<typeof commands.reorderTab>[1],
) {
  return unwrapVoid(commands.reorderTab(tabId, newOrder));
}

export function setActiveTab(tabId: Parameters<typeof commands.setActiveTab>[0]) {
  return unwrapVoid(commands.setActiveTab(tabId));
}

// Pane commands
export function splitPane(
  paneId: Parameters<typeof commands.splitPane>[0],
  direction: Parameters<typeof commands.splitPane>[1],
) {
  return unwrap(commands.splitPane(paneId, direction));
}

export function closePane(paneId: Parameters<typeof commands.closePane>[0]) {
  return unwrap(commands.closePane(paneId));
}

export function setActivePane(paneId: Parameters<typeof commands.setActivePane>[0]) {
  return unwrapVoid(commands.setActivePane(paneId));
}

export function setPaneLabel(
  paneId: Parameters<typeof commands.setPaneLabel>[0],
  label: Parameters<typeof commands.setPaneLabel>[1],
) {
  return unwrap(commands.setPaneLabel(paneId, label));
}

// Terminal I/O commands
export function sendInput(
  paneId: Parameters<typeof commands.sendInput>[0],
  data: Parameters<typeof commands.sendInput>[1],
) {
  return unwrapVoid(commands.sendInput(paneId, data));
}

export function getPaneScreenSnapshot(
  paneId: Parameters<typeof commands.getPaneScreenSnapshot>[0],
) {
  return unwrap(commands.getPaneScreenSnapshot(paneId));
}

export function resizePane(
  paneId: Parameters<typeof commands.resizePane>[0],
  cols: Parameters<typeof commands.resizePane>[1],
  rows: Parameters<typeof commands.resizePane>[2],
  pixelWidth: Parameters<typeof commands.resizePane>[3],
  pixelHeight: Parameters<typeof commands.resizePane>[4],
) {
  return unwrapVoid(commands.resizePane(paneId, cols, rows, pixelWidth, pixelHeight));
}

// Scroll commands
export function scrollPane(
  paneId: Parameters<typeof commands.scrollPane>[0],
  offset: Parameters<typeof commands.scrollPane>[1],
) {
  return unwrap(commands.scrollPane(paneId, offset));
}

export function scrollToBottom(paneId: Parameters<typeof commands.scrollToBottom>[0]) {
  return unwrapVoid(commands.scrollToBottom(paneId));
}

// Search commands
export function searchPane(
  paneId: Parameters<typeof commands.searchPane>[0],
  query: Parameters<typeof commands.searchPane>[1],
) {
  return unwrap(commands.searchPane(paneId, query));
}

// SSH commands
export function openSshConnection(
  paneId: Parameters<typeof commands.openSshConnection>[0],
  connectionId: Parameters<typeof commands.openSshConnection>[1],
  pixelWidth?: Parameters<typeof commands.openSshConnection>[2],
  pixelHeight?: Parameters<typeof commands.openSshConnection>[3],
) {
  return unwrapVoid(
    commands.openSshConnection(paneId, connectionId, pixelWidth ?? null, pixelHeight ?? null),
  );
}

export function closeSshConnection(paneId: Parameters<typeof commands.closeSshConnection>[0]) {
  return unwrapVoid(commands.closeSshConnection(paneId));
}

export function reconnectSsh(paneId: Parameters<typeof commands.reconnectSsh>[0]) {
  return unwrapVoid(commands.reconnectSsh(paneId));
}

export function acceptHostKey(paneId: Parameters<typeof commands.acceptHostKey>[0]) {
  return unwrapVoid(commands.acceptHostKey(paneId));
}

export function rejectHostKey(paneId: Parameters<typeof commands.rejectHostKey>[0]) {
  return unwrapVoid(commands.rejectHostKey(paneId));
}

export function provideCredentials(
  paneId: Parameters<typeof commands.provideCredentials>[0],
  credentials: Parameters<typeof commands.provideCredentials>[1],
) {
  return unwrapVoid(commands.provideCredentials(paneId, credentials));
}

export function providePassphrase(
  paneId: Parameters<typeof commands.providePassphrase>[0],
  passphrase: Parameters<typeof commands.providePassphrase>[1],
  saveInKeychain: Parameters<typeof commands.providePassphrase>[2],
) {
  return unwrapVoid(commands.providePassphrase(paneId, passphrase, saveInKeychain));
}

export function dismissSshAlgorithmWarning(
  paneId: Parameters<typeof commands.dismissSshAlgorithmWarning>[0],
) {
  return unwrapVoid(commands.dismissSshAlgorithmWarning(paneId));
}

// Connection management commands
export function getConnections() {
  return unwrap(commands.getConnections());
}

export function saveConnection(config: Parameters<typeof commands.saveConnection>[0]) {
  return unwrap(commands.saveConnection(config));
}

export function deleteConnection(connectionId: Parameters<typeof commands.deleteConnection>[0]) {
  return unwrapVoid(commands.deleteConnection(connectionId));
}

export function duplicateConnection(
  connectionId: Parameters<typeof commands.duplicateConnection>[0],
) {
  return unwrap(commands.duplicateConnection(connectionId));
}

export function storeConnectionPassword(
  connectionId: Parameters<typeof commands.storeConnectionPassword>[0],
  username: Parameters<typeof commands.storeConnectionPassword>[1],
  password: Parameters<typeof commands.storeConnectionPassword>[2],
) {
  return unwrapVoid(commands.storeConnectionPassword(connectionId, username, password));
}

// Preferences commands
export function getPreferences() {
  return unwrap(commands.getPreferences());
}

export function updatePreferences(patch: Parameters<typeof commands.updatePreferences>[0]) {
  return unwrap(commands.updatePreferences(patch));
}

// Theme commands
export function getThemes() {
  return unwrap(commands.getThemes());
}

export function saveTheme(theme: Parameters<typeof commands.saveTheme>[0]) {
  return unwrapVoid(commands.saveTheme(theme));
}

export function deleteTheme(name: Parameters<typeof commands.deleteTheme>[0]) {
  return unwrapVoid(commands.deleteTheme(name));
}

// Clipboard commands
export function copyToClipboard(text: Parameters<typeof commands.copyToClipboard>[0]) {
  return unwrapVoid(commands.copyToClipboard(text));
}

export function getClipboard() {
  return unwrap(commands.getClipboard());
}

// URL / misc commands
export function openUrl(url: Parameters<typeof commands.openUrl>[0], paneId?: string) {
  return unwrapVoid(commands.openUrl(url, paneId ?? null));
}

export function markContextMenuUsed() {
  return unwrapVoid(commands.markContextMenuUsed());
}

// Flow-control commands
export function frameAck(paneId: Parameters<typeof commands.frameAck>[0]): void {
  void unwrapVoid(commands.frameAck(paneId));
}

// Window commands
export function toggleFullscreen() {
  return unwrap(commands.toggleFullscreen());
}

export function hasForegroundProcess(paneId: Parameters<typeof commands.hasForegroundProcess>[0]) {
  return unwrap(commands.hasForegroundProcess(paneId));
}

// ---------------------------------------------------------------------------
// Event adapters — same signatures as the hand-written events.ts
//
// Each `onXxx(handler)` subscribes via the generated event's `.listen()` and
// unwraps `.payload` so the handler receives the payload directly.
// Returns `Promise<() => void>` (the unsubscribe function).
// ---------------------------------------------------------------------------

export function onSessionStateChanged(
  handler: (event: import('./bindings').SessionStateChangedEvent) => void,
): Promise<() => void> {
  return events.sessionStateChanged.listen((e) => handler(e.payload));
}

export function onSshStateChanged(
  handler: (event: import('./bindings').SshStateChangedEvent) => void,
): Promise<() => void> {
  return events.sshStateChanged.listen((e) => handler(e.payload));
}

export function onSshWarning(
  handler: (event: import('./bindings').SshWarningEvent) => void,
): Promise<() => void> {
  return events.sshWarning.listen((e) => handler(e.payload));
}

export function onSshReconnected(
  handler: (event: import('./bindings').SshReconnectedEvent) => void,
): Promise<() => void> {
  return events.sshReconnected.listen((e) => handler(e.payload));
}

export function onHostKeyPrompt(
  handler: (event: import('./bindings').HostKeyPromptEvent) => void,
): Promise<() => void> {
  return events.hostKeyPrompt.listen((e) => handler(e.payload));
}

export function onCredentialPrompt(
  handler: (event: import('./bindings').CredentialPromptEvent) => void,
): Promise<() => void> {
  return events.credentialPrompt.listen((e) => handler(e.payload));
}

export function onPassphrasePrompt(
  handler: (event: import('./bindings').PassphrasePromptEvent) => void,
): Promise<() => void> {
  return events.passphrasePrompt.listen((e) => handler(e.payload));
}

export function onNotificationChanged(
  handler: (event: import('./bindings').NotificationChangedEvent) => void,
): Promise<() => void> {
  return events.notificationChanged.listen((e) => handler(e.payload));
}

export function onModeStateChanged(
  handler: (event: import('./bindings').ModeStateChangedEvent) => void,
): Promise<() => void> {
  return events.modeStateChanged.listen((e) => handler(e.payload));
}

export function onScreenUpdate(
  handler: (event: import('./bindings').ScreenUpdateEvent) => void,
): Promise<() => void> {
  return events.screenUpdate.listen((e) => handler(e.payload));
}

export function onScrollPositionChanged(
  handler: (event: import('./bindings').ScrollPositionChangedEvent) => void,
): Promise<() => void> {
  return events.scrollPositionChanged.listen((e) => handler(e.payload));
}

export function onCursorStyleChanged(
  handler: (event: import('./bindings').CursorStyleChangedEvent) => void,
): Promise<() => void> {
  return events.cursorStyleChanged.listen((e) => handler(e.payload));
}

export function onBellTriggered(
  handler: (event: import('./bindings').BellTriggeredEvent) => void,
): Promise<() => void> {
  return events.bellTriggered.listen((e) => handler(e.payload));
}

export function onOsc52WriteRequested(
  handler: (event: import('./bindings').Osc52WriteRequestedEvent) => void,
): Promise<() => void> {
  return events.osc52WriteRequested.listen((e) => handler(e.payload));
}

export function onPreferencesChanged(
  handler: (event: import('./bindings').PreferencesChangedEvent) => void,
): Promise<() => void> {
  return events.preferencesChanged.listen((e) => handler(e.payload));
}

export function onFullscreenStateChanged(
  handler: (event: import('./bindings').FullscreenStateChangedEvent) => void,
): Promise<() => void> {
  return events.fullscreenStateChanged.listen((e) => handler(e.payload));
}

// ---------------------------------------------------------------------------
// Type re-exports from generated bindings
//
// The generated file uses `type` aliases; the hand-written file used
// `interface`.  Both are structurally compatible for read-only consumption.
//
// Some generated types have Serialize/Deserialize variants (e.g.
// `CellAttrsDto = CellAttrsDto_Serialize | CellAttrsDto_Deserialize`).
// The frontend only *reads* data from the backend, so the `_Deserialize`
// variant is the one that matters.  We re-export both the union and the
// deserialize-specific variant so callers can choose.
// ---------------------------------------------------------------------------

export type {
  // Shared primitives
  PaneId,
  TabId,
  ConnectionId,

  // Session / layout types
  SessionState,
  TabState,
  PaneNode,
  PaneState,
  PaneLifecycleState,
  SplitDirection,

  // Screen rendering types
  ScreenSnapshot,
  SnapshotCell,
  CellUpdate,
  CellUpdate_Deserialize,
  CellAttrsDto,
  CellAttrsDto_Deserialize,
  CursorState,
  Color,
  ColorDto,

  // Scroll / search types
  ScrollPositionState,
  SearchQuery,
  SearchMatch,

  // SSH types
  SshConnectionConfig,
  SshLifecycleState,
  Credentials,

  // Preferences types
  Preferences,
  PreferencesPatch,
  AppearancePrefs,
  AppearancePatch,
  TerminalPrefs,
  TerminalPatch,
  KeyboardPrefs,
  KeyboardPatch,
  Language,
  CursorStyle,
  BellType,
  FullscreenChromeBehavior,
  FontFamily,
  ThemeName,
  WordDelimiters,
  SshHost,
  SshLabel,
  SshUsername,
  SshIdentityPath,

  // Theme types
  UserTheme,

  // Window types
  FullscreenState,
  CreateTabConfig,

  // Notification types
  PaneNotificationDto as PaneNotification,

  // Error type
  TauTermError,

  // Event payload types
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
  Osc52WriteRequestedEvent,
  PreferencesChangedEvent,
  FullscreenStateChangedEvent,
  MouseReportingMode,
  MouseEncoding,
} from './bindings';
