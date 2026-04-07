// SPDX-License-Identifier: MPL-2.0

/**
 * Canonical CSS selectors for TauTerm E2E tests.
 *
 * Centralised here to prevent selector drift across spec files.
 * Update this file when the DOM structure changes — do not inline
 * selectors in individual specs.
 */
export const Selectors = {
  terminalGrid: ".terminal-grid",
  terminalCell: ".terminal-pane__cell",
  terminalPane: ".terminal-pane",
  activeTerminalPane: ".terminal-pane[data-active='true']",
  tabBar: ".tab-bar",
  tab: ".tab-bar__tab",
  activeTab: ".tab-bar__tab[aria-selected='true']",
  scrollArrowLeft: ".tab-bar__scroll-arrow--left",
  scrollArrowRight: ".tab-bar__scroll-arrow--right",
  newTabButton: ".tab-bar__new-tab",
  // Fullscreen chrome
  tabRow: ".terminal-view__tab-row",
  tabRowHidden: ".terminal-view__tab-row--hidden",
  fullscreenHoverTop: ".terminal-view__fullscreen-hover-top",
  fullscreenExitBadge: '[data-testid="fullscreen-exit-badge"]',
  fullscreenToggleBtn: '[data-testid="fullscreen-toggle-btn"]',
  // Terminal state overlays
  preferencesPanel: ".preferences-panel",
  processTerminatedPane: ".process-terminated-pane",
  // SSH connection manager
  sshButton: ".terminal-view__ssh-btn",
  connectionManager: ".connection-manager",
  connectionOpenInNewTabBtn: ".connection-manager__action-btn",
  connectionErrorBanner: ".terminal-view__connection-error",
  connectionErrorCloseBtn: ".terminal-view__connection-error-close",
} as const;
