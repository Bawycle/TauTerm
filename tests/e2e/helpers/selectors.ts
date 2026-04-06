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
} as const;
