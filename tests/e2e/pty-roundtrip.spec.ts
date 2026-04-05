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
 */

import { browser, $, $$ } from "@wdio/globals";
import { Selectors } from "./helpers/selectors";

describe("TauTerm — PTY input/output round-trip", () => {
  /**
   * TEST-PTY-RT-001: Initial pane renders a shell prompt.
   *
   * After launch, the first pane should display a shell prompt within 5
   * seconds. This verifies PTY spawn and the VT→screen-buffer→frontend
   * rendering pipeline.
   */
  it("renders a shell prompt in the initial pane", async () => {
    // The terminal grid is rendered inside .terminal-pane > .terminal-grid.
    // We wait up to 10 seconds for at least one row to contain non-empty content.
    const terminalGrid = await $(Selectors.terminalGrid);
    await expect(terminalGrid).toExist();

    // Wait for shell prompt to appear — indicated by at least one cell with content.
    await browser.waitUntil(
      async () => {
        const cells = await $$(Selectors.terminalCell);
        // At least one cell should be non-empty once the shell starts.
        for (const cell of cells) {
          const text = await cell.getText();
          if (text.trim().length > 0) return true;
        }
        return false;
      },
      {
        timeout: 10_000,
        timeoutMsg:
          "Shell prompt did not appear in the terminal grid within 10 seconds",
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
    // Retrieve the active pane's ID from the DOM.  The frontend renders it as
    // a `data-pane-id` attribute on the pane root element so that E2E tests
    // can pass the correct ID to `inject_pty_output`.
    const paneId = await $(Selectors.activeTerminalPane).getAttribute(
      "data-pane-id"
    );
    expect(paneId).toBeTruthy();

    // Prepare the byte sequence: "tauterm-e2e-marker\r\n"
    const marker = "tauterm-e2e-marker";
    const bytes = [...new TextEncoder().encode(marker + "\r\n")];

    // Inject bytes directly into the VT pipeline via the Tauri IPC.
    // `browser.execute` runs the callback in the WebView context, where
    // `@tauri-apps/api/core` is available.  Arguments must be JSON-serialisable:
    // pass `number[]` rather than `Uint8Array` (Uint8Array does not round-trip
    // through JSON cleanly; the Rust side expects Vec<u8> from a JSON array of
    // integers).
    await browser.execute(
      async (paneIdArg: string, dataArg: number[]) => {
        const { invoke } = await import("@tauri-apps/api/core");
        await invoke("inject_pty_output", { paneId: paneIdArg, data: dataArg });
      },
      paneId,
      bytes
    );

    // Wait for the marker to appear in the terminal grid.  The VT pipeline is
    // synchronous from the read-task perspective, but the screen-update event
    // propagation to the DOM is async, so polling via waitUntil is correct.
    await browser.waitUntil(
      async () => {
        const text = await $(Selectors.terminalGrid).getText();
        return text.includes(marker);
      },
      {
        timeout: 3000,
        timeoutMsg: `"${marker}" did not appear on the terminal grid within 3 s`,
      }
    );
  });
});
