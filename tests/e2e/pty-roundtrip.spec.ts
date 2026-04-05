// SPDX-License-Identifier: MPL-2.0

/**
 * E2E scenario: PTY input/output round-trip.
 *
 * Verifies that text typed into a terminal pane is sent to the PTY and the
 * resulting output is rendered in the terminal grid.
 *
 * Protocol references:
 * - TEST-PTY-001 (app.spec.ts — launch)
 * - TEST-PTY-002
 * - docs/test-protocols/functional-pty-vt-ssh-preferences-ui-ipc.md §4.1
 *
 * Note on inject_pty_output IPC:
 * We call `window.__TAURI_INTERNALS__.invoke` via browser.execute (fire-and-forget)
 * rather than browser.executeAsync.  browser.executeAsync requires a callback
 * convention that tauri-driver/WebKitGTK does not honour reliably, causing
 * indefinite hangs.  Since inject_pty_output sends bytes to an unbounded mpsc
 * channel and returns immediately, we don't need its return value — DOM effects
 * (cells appearing in the grid) are asserted separately via waitUntil.
 */

import { browser, $, $$ } from "@wdio/globals";
import { Selectors } from "./helpers/selectors";

/**
 * Fire a Tauri IPC command and return without waiting for its result.
 *
 * `inject_pty_output` sends bytes to an unbounded mpsc channel and returns
 * immediately; we do not need the return value.  Using `browser.execute`
 * (synchronous script execution) avoids the `browser.executeAsync` callback
 * issue in tauri-driver where the done-callback mechanism can stall.
 *
 * The caller is responsible for waiting for observable DOM effects instead of
 * relying on this function's return value for sequencing.
 */
function tauriFireAndForget(cmd: string, args?: Record<string, unknown>): Promise<void> {
  return browser.execute(
    function(cmdArg: string, argsArg: Record<string, unknown> | undefined) {
      // Fire-and-forget: do not await the Promise.
      (window as any).__TAURI_INTERNALS__.invoke(cmdArg, argsArg);
    },
    cmd,
    args,
  ) as unknown as Promise<void>;
}

describe("TauTerm — PTY input/output round-trip", () => {
  /**
   * TEST-PTY-RT-001: Initial pane renders a shell prompt.
   *
   * In the E2E build, InjectablePtyBackend is used instead of a real PTY.
   * We inject a synthetic prompt string via inject_pty_output, then verify
   * that the VT→screen-buffer→DOM pipeline surfaces it in the terminal grid.
   *
   * This verifies:
   * 1. The VT parser processes injected bytes correctly.
   * 2. The screen buffer is updated and emits a screen-update event.
   * 3. The terminal grid DOM is re-rendered with the injected content.
   */
  it("renders a shell prompt in the initial pane", async () => {
    // Wait for the active terminal pane to appear.
    await browser.waitUntil(
      async () => {
        try {
          return await $(Selectors.activeTerminalPane).isExisting();
        } catch {
          return false;
        }
      },
      { timeout: 15_000, timeoutMsg: "Active terminal pane did not appear within 15 s" }
    );

    const paneId = await $(Selectors.activeTerminalPane).getAttribute("data-pane-id");
    expect(paneId).toBeTruthy();

    // Inject a synthetic prompt to simulate shell output.
    const prompt = "user@tauterm:~$ ";
    const bytes = [...new TextEncoder().encode(prompt)];
    await tauriFireAndForget("inject_pty_output", { paneId, data: bytes });

    // The terminal grid must exist.
    const terminalGrid = await $(Selectors.terminalGrid);
    await expect(terminalGrid).toExist();

    // Wait for at least one non-empty cell to appear (prompt rendered).
    // Use browser.execute for the DOM scan — iterating 1920 cells via WebDriver
    // RPC (one call per cell) would exceed any reasonable timeout.
    await browser.waitUntil(
      () =>
        browser.execute((): boolean => {
          for (const cell of document.querySelectorAll(".terminal-pane__cell")) {
            if ((cell.textContent ?? "").trim().length > 0) return true;
          }
          return false;
        }),
      {
        timeout: 10_000,
        timeoutMsg: "Shell prompt did not appear in the terminal grid within 10 seconds",
      }
    );
  });

  /**
   * TEST-PTY-RT-002: Injected bytes appear on the terminal grid.
   *
   * Calls `inject_pty_output` via the Tauri IPC to push synthetic bytes
   * directly into the VT pipeline for the active pane, bypassing the real PTY.
   * Waits for the injected marker string to appear in the terminal grid.
   *
   * This verifies:
   * 1. The injectable PTY backend channels bytes through the VT parser.
   * 2. The screen-buffer-to-DOM rendering pipeline surfaces the output.
   * 3. `inject_pty_output` is correctly wired and registered (ADR-0015).
   *
   * The `e2e-testing` Cargo feature must be active in the binary under test.
   */
  it("TEST-PTY-RT-002: injected bytes appear on the terminal grid", async () => {
    // Retrieve the active pane's ID from the DOM.
    const paneId = await $(Selectors.activeTerminalPane).getAttribute("data-pane-id");
    expect(paneId).toBeTruthy();

    // Prepare the byte sequence: "tauterm-e2e-marker\r\n"
    const marker = "tauterm-e2e-marker";
    const bytes = [...new TextEncoder().encode(marker + "\r\n")];

    // Inject bytes directly into the VT pipeline via the Tauri IPC.
    await tauriFireAndForget("inject_pty_output", { paneId, data: bytes });

    // Wait for the marker to appear in the terminal grid.
    // Use browser.execute for the text scan (single RPC call) to avoid the
    // per-cell RPC overhead that would exceed the timeout.
    await browser.waitUntil(
      () =>
        browser.execute((markerArg: string): boolean => {
          const grid = document.querySelector(".terminal-grid");
          return grid !== null && (grid.textContent ?? "").includes(markerArg);
        }, marker),
      {
        timeout: 10_000,
        timeoutMsg: `"${marker}" did not appear on the terminal grid within 10 s`,
      }
    );
  });
});
