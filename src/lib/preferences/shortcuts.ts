// SPDX-License-Identifier: MPL-2.0

/**
 * Single source of truth for default keyboard shortcut bindings.
 *
 * Used by both:
 *   - useTerminalView.io-handlers — shortcut matching at runtime
 *   - PreferencesKeyboardSection — display and editing of shortcuts
 *
 * When adding a new action, add it here and update both consumers.
 */

export const DEFAULT_SHORTCUTS: Record<string, string> = {
  new_tab: 'Ctrl+Shift+T',
  close_tab: 'Ctrl+Shift+W',
  paste: 'Ctrl+Shift+V',
  search: 'Ctrl+Shift+F',
  preferences: 'Ctrl+,',
  next_tab: 'Ctrl+Tab',
  prev_tab: 'Ctrl+Shift+Tab',
  rename_tab: 'F2',
  toggle_fullscreen: 'F11',
  split_pane_h: 'Ctrl+Shift+D',
  split_pane_v: 'Ctrl+Shift+E',
  close_pane: 'Ctrl+Shift+Q',
  navigate_pane_left: 'Ctrl+Shift+ArrowLeft',
  navigate_pane_right: 'Ctrl+Shift+ArrowRight',
  navigate_pane_up: 'Ctrl+Shift+ArrowUp',
  navigate_pane_down: 'Ctrl+Shift+ArrowDown',
};

/** Type for valid action IDs. */
export type ShortcutActionId = keyof typeof DEFAULT_SHORTCUTS;
