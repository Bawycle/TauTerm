// SPDX-License-Identifier: MPL-2.0

/**
 * E2E scenario: PTY input/output round-trip.
 *
 * Verifies that text typed into a terminal pane is sent to the PTY and the
 * resulting output is rendered in the terminal grid.
 *
 * DEFERRED: build required — these tests require `pnpm tauri build` to produce
 * a working binary with a real PTY backend. Until PTY I/O is implemented
 * (LinuxPtySession::write is currently a stub), these scenarios will fail at
 * the PTY write step. They are written here to establish the test contract and
 * unblock E2E CI setup.
 *
 * Protocol references:
 * - TEST-PTY-001 (app.spec.ts — launch)
 * - TEST-PTY-002 (blocked: PTY write stub)
 * - docs/test-protocols/functional-pty-vt-ssh-preferences-ui-ipc.md §4.1
 */

import { browser, $, $$ } from "@wdio/globals";

describe("TauTerm — PTY input/output round-trip", () => {
  /**
   * TEST-PTY-RT-001: Initial pane renders a shell prompt.
   *
   * After launch, the first pane should display a shell prompt within 5
   * seconds. This verifies PTY spawn and the VT→screen-buffer→frontend
   * rendering pipeline.
   *
   * [DEFERRED: PTY write stub] — passes only once LinuxPtySession::write
   * is implemented and the screen-buffer event pipeline emits screen updates.
   */
  it("renders a shell prompt in the initial pane", async () => {
    // The terminal grid is rendered inside .terminal-pane > .terminal-grid.
    // We wait up to 10 seconds for at least one row to contain non-empty content.
    const terminalGrid = await $(".terminal-grid");
    await expect(terminalGrid).toExist();

    // Wait for shell prompt to appear — indicated by at least one cell with content.
    await browser.waitUntil(
      async () => {
        const cells = await $$(".terminal-grid .cell");
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
          "Shell prompt did not appear in the terminal grid within 10 seconds " +
          "[DEFERRED: requires PTY write implementation]",
      }
    );
  });

  /**
   * TEST-PTY-RT-002: Type a command and observe its echo in the terminal.
   *
   * Types `echo tauterm-e2e-marker` followed by Enter. Waits for the marker
   * string to appear in the terminal output. This verifies:
   * 1. IPC `send_input` is wired to PTY write.
   * 2. PTY output is read and rendered via the screen-buffer pipeline.
   *
   * [DEFERRED: PTY write stub]
   */
  it("echoes typed text back in the terminal output", async () => {
    const marker = "tauterm-e2e-marker";
    const terminalGrid = await $(".terminal-grid");
    await expect(terminalGrid).toExist();

    // Focus the terminal pane (click on it).
    await terminalGrid.click();

    // Type the command using WebdriverIO keyboard input.
    await browser.keys(["echo ", marker, "Return"]);

    // Wait for the marker to appear in the rendered terminal output.
    await browser.waitUntil(
      async () => {
        const gridText = await terminalGrid.getText();
        return gridText.includes(marker);
      },
      {
        timeout: 10_000,
        timeoutMsg:
          `Marker string "${marker}" did not appear in terminal output within 10 seconds ` +
          "[DEFERRED: requires PTY I/O implementation]",
      }
    );
  });
});
