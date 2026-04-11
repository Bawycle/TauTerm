// SPDX-License-Identifier: MPL-2.0

/**
 * Global Vitest setup — polyfills for jsdom gaps.
 *
 * Loaded via `test.setupFiles` in vitest.config.ts.
 */

// ResizeObserver: jsdom does not implement it, but several components
// (TabBar, SplitPane, etc.) use it for layout-driven updates.
// This no-op stub prevents ReferenceError in unit tests.
// Actual resize behaviour is validated in E2E tests where a real browser is used.
if (typeof globalThis.ResizeObserver === 'undefined') {
  globalThis.ResizeObserver = class ResizeObserver {
    observe() {}
    unobserve() {}
    disconnect() {}
  };
}

// scrollIntoView: jsdom does not implement it; components (e.g. TabBar active
// tab scrolling) call it in requestAnimationFrame callbacks. A no-op stub
// prevents TypeError when those callbacks fire in unit tests.
// Smooth-scroll behaviour is validated in E2E tests only.
if (typeof Element.prototype.scrollIntoView === 'undefined') {
  Element.prototype.scrollIntoView = function (): void {};
}
