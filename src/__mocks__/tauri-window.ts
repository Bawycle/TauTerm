// SPDX-License-Identifier: MPL-2.0

/**
 * Test stub for @tauri-apps/api/window — getCurrentWindow().
 *
 * Mirrors the real Tauri 2 onCloseRequested contract (from window.js source):
 *
 *   onCloseRequested(handler) wraps the user handler:
 *     await handler(evt);
 *     if (!evt.isPreventDefault()) { await this.destroy(); }
 *
 *   So simulateCloseRequest() runs all registered handlers, then calls
 *   destroy() automatically if no handler called event.preventDefault().
 *   This exactly mirrors what Tauri 2's onCloseRequested wrapper does.
 *
 *   destroy() forces the window closed without firing CloseRequested.
 *   Production code uses destroy() for all programmatic closes.
 *
 *   close() re-emits CloseRequested (can be intercepted again). Production
 *   code does NOT use close() for programmatic closes — only destroy().
 *
 * Test helpers:
 *   mockAppWindow.simulateCloseRequest() — trigger the WM close button
 *   mockAppWindow.closed                 — true once destroy() was called
 *   mockAppWindow.destroyCallCount       — number of destroy() calls
 *   resetMockWindow()                    — call in afterEach to clear state
 */

export type UnlistenFn = () => void;

type CloseHandler = (event: { preventDefault: () => void }) => void | Promise<void>;

class MockAppWindow {
  private handlers: CloseHandler[] = [];

  /** True once destroy() has been called. */
  public closed = false;

  /** Counts every destroy() call. */
  public destroyCallCount = 0;

  /** Last title set via setTitle(). */
  public title: string | null = null;

  /** Counts every setTitle() call. */
  public setTitleCallCount = 0;

  async isFullscreen(): Promise<boolean> {
    return false;
  }

  async setTitle(title: string): Promise<void> {
    this.title = title;
    this.setTitleCallCount++;
  }

  async onCloseRequested(handler: CloseHandler): Promise<UnlistenFn> {
    this.handlers.push(handler);
    return () => {
      this.handlers = this.handlers.filter((h) => h !== handler);
    };
  }

  /**
   * Forces the window closed without emitting CloseRequested.
   * Mirrors appWindow.destroy() in Tauri 2 — always succeeds immediately.
   */
  async destroy(): Promise<void> {
    this.destroyCallCount++;
    this.closed = true;
  }

  /**
   * Emits CloseRequested (can be intercepted). Mirrors appWindow.close().
   * Production code does not call this for programmatic closes — it uses
   * destroy(). Provided for completeness; tests should not need it.
   */
  async close(): Promise<void> {
    await this.simulateCloseRequest();
  }

  /**
   * Simulates the user pressing the WM close button.
   * Mirrors Tauri 2's onCloseRequested wrapper:
   *   1. Runs all registered handlers (awaiting each).
   *   2. If no handler called event.preventDefault(), calls destroy().
   */
  async simulateCloseRequest(): Promise<void> {
    let prevented = false;
    const event = {
      preventDefault: () => {
        prevented = true;
      },
    };

    for (const handler of [...this.handlers]) {
      await handler(event);
    }

    if (!prevented) {
      await this.destroy();
    }
  }

  /** Reset all state — call in afterEach. */
  reset(): void {
    this.handlers = [];
    this.closed = false;
    this.destroyCallCount = 0;
    this.title = null;
    this.setTitleCallCount = 0;
  }
}

export const mockAppWindow = new MockAppWindow();

export function getCurrentWindow(): MockAppWindow {
  return mockAppWindow;
}

/** Convenience reset — import and call in afterEach. */
export function resetMockWindow(): void {
  mockAppWindow.reset();
}
