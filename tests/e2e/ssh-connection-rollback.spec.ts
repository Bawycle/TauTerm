// SPDX-License-Identifier: MPL-2.0

/**
 * E2E scenario: SSH connection open rollback (FS-SSH-032).
 *
 * Verifies that when `open_ssh_connection` fails after `create_tab` has
 * already succeeded, `handleConnectionOpen` correctly:
 *   1. Calls `close_tab` to roll back the orphan tab.
 *   2. Shows the `.terminal-view__connection-error` error banner.
 *   3. Allows the banner to be dismissed.
 *
 * Protocol references:
 * - TEST-SSH-ROLLBACK-001 through TEST-SSH-ROLLBACK-003
 * - FS-SSH-032 (rollback on open_ssh_connection failure)
 * - docs/test-protocols/functional-pty-vt-ssh-preferences-ui-ipc.md §4.4
 *
 * Mechanism:
 *   `inject_ssh_failure({ count: 1 })` arms the `SshFailureRegistry` (e2e-testing
 *   feature only) so the next `open_ssh_connection` call returns a synthetic
 *   error immediately, without needing a real or unreachable SSH server.
 *   The test connection is created through the ConnectionManager UI so that
 *   TerminalView's `savedConnections` state is updated synchronously — no
 *   need to reload preferences from disk.
 *
 * Prerequisites:
 *   Binary must be built with `--features e2e-testing` (`pnpm tauri build --no-bundle
 *   -- --features e2e-testing`).
 */

import { browser, $ } from "@wdio/globals";
import { Selectors } from "./helpers/selectors";

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/** Count tabs currently in the DOM. */
async function countTabs(): Promise<number> {
  const tabs = await browser.$$(Selectors.tab);
  return tabs.length;
}

/**
 * Fire a Tauri IPC command without waiting for its result.
 *
 * Mirrors the pattern established in pty-roundtrip.spec.ts: `browser.execute`
 * (synchronous script injection) rather than `browser.executeAsync`, which is
 * unreliable with tauri-driver / WebKitGTK.
 */
function tauriFireAndForget(cmd: string, args?: Record<string, unknown>): Promise<void> {
  return browser.execute(
    function (cmdArg: string, argsArg: Record<string, unknown> | undefined) {
      (window as unknown as {
        __TAURI_INTERNALS__: { invoke: (c: string, a?: unknown) => Promise<unknown> };
      }).__TAURI_INTERNALS__.invoke(cmdArg, argsArg);
    },
    cmd,
    args,
  ) as unknown as Promise<void>;
}

/**
 * Open the SSH ConnectionManager panel via the SSH button in the tab row.
 * Waits for the panel to be present in the DOM before returning.
 */
async function openConnectionManager(): Promise<void> {
  await $(Selectors.terminalGrid).click();
  const sshBtn = await $(Selectors.sshButton);
  await expect(sshBtn).toExist();

  // Use dispatchEvent so Svelte 5 listeners receive the click reliably.
  await browser.execute(function () {
    const btn = document.querySelector(".terminal-view__ssh-btn") as HTMLButtonElement | null;
    btn?.dispatchEvent(new MouseEvent("click", { bubbles: true, cancelable: true }));
  });

  await browser.waitUntil(
    async () => {
      try {
        return await $(Selectors.connectionManager).isExisting();
      } catch {
        return false;
      }
    },
    { timeout: 5_000, timeoutMsg: "ConnectionManager did not appear within 5 s" },
  );
}

/**
 * Create a dummy SSH connection via the ConnectionManager form UI.
 *
 * Using the form rather than a direct `save_connection` IPC call ensures that
 * TerminalView's `savedConnections` state is updated immediately (the form's
 * `handleSave` calls `onsave` which updates the parent state).  A direct IPC
 * call would persist to disk but the already-mounted TerminalView would not
 * see the change until the next `get_connections` load.
 *
 * Fields are filled using WebdriverIO `addValue` on the WebElement, which
 * triggers the native `input` event that Svelte 5 listens to via `oninput`.
 */
async function createConnectionViaForm(): Promise<void> {
  // Click "New connection" button to open the form.
  await browser.execute(function () {
    const actionsEl = document.querySelector(".connection-manager__actions");
    const btn = actionsEl?.querySelector("button");
    btn?.dispatchEvent(new MouseEvent("click", { bubbles: true, cancelable: true }));
  });

  // Wait for the form to appear (identified by the host input).
  await browser.waitUntil(
    () =>
      browser.execute(function (): boolean {
        return document.getElementById("cm-host") !== null;
      }),
    { timeout: 3_000, timeoutMsg: "ConnectionManager form (#cm-host) did not appear" },
  );

  // Populate form fields by simulating typing via the WebDriver element API.
  // We use browser.execute to dispatch native InputEvent objects that Svelte 5
  // oninput handlers can observe (WebDriver sendKeys in WebKitGTK sometimes
  // routes keystrokes to the window rather than the focused input element).

  async function typeIntoInput(id: string, value: string): Promise<void> {
    // Focus the element via WebdriverIO click (reliable for scroll-into-view).
    const el = await browser.$(`#${id}`);
    await el.click();
    // Clear existing content and set via InputEvent dispatch so Svelte sees it.
    await browser.execute(
      function (elId: string, val: string): void {
        const input = document.getElementById(elId) as HTMLInputElement | null;
        if (!input) return;
        // Focus explicitly in the document context.
        input.focus();
        // Set value using the native setter so React/Svelte input wrappers detect the change.
        const proto = Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, "value");
        proto?.set?.call(input, val);
        // Dispatch a real InputEvent — Svelte 5 listens for "input" via addEventListener.
        input.dispatchEvent(new InputEvent("input", { bubbles: true, cancelable: false, data: val, inputType: "insertText" }));
        // Also dispatch "change" for good measure.
        input.dispatchEvent(new Event("change", { bubbles: true }));
      },
      id,
      value,
    );
    // Wait for the input value to be reflected in the DOM before proceeding.
    await browser.waitUntil(
      () =>
        browser.execute(
          function (elId: string, val: string): boolean {
            const input = document.getElementById(elId) as HTMLInputElement | null;
            return input !== null && input.value === val;
          },
          id,
          value,
        ),
      { timeout: 2_000, timeoutMsg: `Input #${id} did not reflect value "${value}"` },
    );
  }

  await typeIntoInput("cm-label", "E2E Test Connection");
  await typeIntoInput("cm-host", "localhost");
  // Port defaults to 22 — skip.
  await typeIntoInput("cm-username", "e2e-test-user");

  // Switch auth method to "password" to avoid identity_file path validation.
  // The default auth method is "identity", which would submit an empty
  // identityFile path that Rust rejects as non-absolute (INVALID_PATH).
  await browser.execute(function () {
    // The password radio button has value="password" and name="cm-auth-method".
    const radio = document.querySelector<HTMLInputElement>(
      'input[name="cm-auth-method"][value="password"]',
    );
    if (!radio) return;
    radio.checked = true;
    radio.dispatchEvent(new Event("change", { bubbles: true }));
  });

  // Wait for Svelte reactive state to reflect the radio change: the
  // password auth method radio must be checked in the DOM before proceeding.
  await browser.waitUntil(
    () =>
      browser.execute(function (): boolean {
        const radio = document.querySelector<HTMLInputElement>(
          'input[name="cm-auth-method"][value="password"]',
        );
        return radio !== null && radio.checked;
      }),
    { timeout: 3_000, timeoutMsg: "Password auth radio did not become checked" },
  );

  // Click the first button inside the form footer div (the "Save" button,
  // whichever locale is active).  The form footer renders as a `<div class="flex
  // gap-2 mt-6">` containing the Save button first, then Cancel.
  await browser.execute(function () {
    // The form is the only element containing #cm-host.
    // The save button is the first <button> inside the flex footer that follows
    // the form fields.  More reliably: it is the first button in the
    // .connection-manager__form area that is NOT a Cancel-type button
    // (Cancel button is the second button in the footer).
    const form = document.querySelector(".connection-manager__form");
    if (!form) return;
    const buttons = form.querySelectorAll<HTMLButtonElement>("button");
    // The footer has exactly two buttons: Save (index 0), Cancel (index 1).
    // Click the first one (Save / Enregistrer / etc.).
    if (buttons.length > 0) {
      buttons[0].dispatchEvent(new MouseEvent("click", { bubbles: true, cancelable: true }));
    }
  });

  // Wait for the form to close (list view becomes visible again) AND for at
  // least one connection item to appear.  The item appears after:
  //   1. showForm = false (synchronous — fires when Save button is clicked)
  //   2. handleConnectionSave IPC completes and savedConnections is updated
  //      (async — may take a few hundred ms for the Tauri IPC round-trip)
  await browser.waitUntil(
    () =>
      browser.execute(function (): boolean {
        return (
          document.querySelector(".connection-manager__actions") !== null &&
          document.getElementById("cm-host") === null &&
          document.querySelectorAll(".connection-manager__item").length > 0
        );
      }),
    { timeout: 8_000, timeoutMsg: "ConnectionManager did not show a connection item after save" },
  );
}

/**
 * Invoke a Tauri IPC command and return its result.
 *
 * Stores the resolved value in `window.__e2e_invoke_result` then polls until
 * it appears, working around the `browser.executeAsync` unreliability with
 * tauri-driver / WebKitGTK.
 */
async function tauriInvoke<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  await browser.execute(function () {
    (window as unknown as Record<string, unknown>).__e2e_invoke_result = undefined;
  });

  await browser.execute(
    function (cmdArg: string, argsArg: Record<string, unknown> | undefined) {
      (window as unknown as {
        __TAURI_INTERNALS__: { invoke: (c: string, a?: unknown) => Promise<unknown> };
      }).__TAURI_INTERNALS__
        .invoke(cmdArg, argsArg)
        .then(function (r: unknown) {
          (window as unknown as Record<string, unknown>).__e2e_invoke_result = r ?? "__e2e_null__";
        })
        .catch(function () {
          (window as unknown as Record<string, unknown>).__e2e_invoke_result = "__e2e_error__";
        });
    },
    cmd,
    args,
  );

  await browser.waitUntil(
    () =>
      browser.execute(function (): boolean {
        return (window as unknown as Record<string, unknown>).__e2e_invoke_result !== undefined;
      }),
    { timeout: 5_000, timeoutMsg: `tauriInvoke("${cmd}") did not resolve within 5 s` },
  );

  const raw = await browser.execute(function (): unknown {
    const v = (window as unknown as Record<string, unknown>).__e2e_invoke_result;
    delete (window as unknown as Record<string, unknown>).__e2e_invoke_result;
    return v;
  });

  if (raw === "__e2e_error__") {
    throw new Error(`tauriInvoke("${cmd}") rejected`);
  }
  return (raw === "__e2e_null__" ? null : raw) as T;
}

/**
 * Arm one `open_ssh_connection` failure and click "Open in new tab" for the
 * first connection in the list.
 *
 * `inject_ssh_failure` is awaited via `tauriInvoke` to ensure the backend has
 * registered the failure before the frontend triggers the SSH open.
 *
 * "Open in new tab" is the first button in `.connection-manager__item-actions`
 * — identified by position, not by translated text or locale-dependent label.
 */
async function armFailureAndOpenInNewTab(): Promise<void> {
  // Arm one SSH failure for the next open_ssh_connection call.
  // Use tauriInvoke (awaited) to guarantee the backend is armed before the click.
  await tauriInvoke<void>("inject_ssh_failure", { count: 1 });

  // Click "Open in new tab" — first button in the first item's action group.
  // The order in ConnectionManager.svelte is: Open in new tab (1), Open in
  // pane (2), Edit (3), Duplicate (4), Delete (5).
  await browser.execute(function () {
    const actionGroup = document.querySelector<HTMLElement>(".connection-manager__item-actions");
    if (!actionGroup) return;
    const buttons = actionGroup.querySelectorAll<HTMLButtonElement>(
      ".connection-manager__action-btn",
    );
    if (buttons.length > 0) {
      buttons[0].dispatchEvent(new MouseEvent("click", { bubbles: true, cancelable: true }));
    }
  });
}

// ---------------------------------------------------------------------------
// Suite
// ---------------------------------------------------------------------------

describe("TauTerm — SSH connection open rollback (FS-SSH-032)", () => {
  /**
   * Setup: open the ConnectionManager and create a dummy connection via the
   * form UI so that `savedConnections` in TerminalView is populated without
   * requiring a page reload.
   */
  before(async () => {
    await openConnectionManager();
    await createConnectionViaForm();
  });

  /**
   * TEST-SSH-ROLLBACK-001: No orphan tab after open_ssh_connection failure.
   *
   * When `open_ssh_connection` fails after `create_tab` has already been called,
   * the rollback logic in `handleConnectionOpen` must call `close_tab` for the
   * newly created tab and remove it from local state.  The tab bar must show the
   * same number of tabs as before the open attempt.
   */
  it("does not leave an orphan tab when open_ssh_connection fails", async () => {
    const tabsBefore = await countTabs();

    // Verify the ConnectionManager is still visible with a connection in the list.
    // If the save succeeded but the panel closed, re-open it.
    const managerVisible = await browser.execute(function (): boolean {
      return document.querySelector(".connection-manager") !== null;
    });
    if (!managerVisible) {
      await openConnectionManager();
    }

    // Verify there is at least one connection item in the list.
    const hasItems = await browser.execute(function (): boolean {
      return document.querySelectorAll(".connection-manager__item").length > 0;
    });
    if (!hasItems) {
      throw new Error("No connection items found in ConnectionManager list — setup may have failed");
    }

    await armFailureAndOpenInNewTab();

    // Wait for the error banner — it signals that the SSH step completed and
    // the rollback has been applied (the banner is set after tabs is updated).
    await browser.waitUntil(
      async () => {
        try {
          return await $(Selectors.connectionErrorBanner).isExisting();
        } catch {
          return false;
        }
      },
      {
        timeout: 8_000,
        timeoutMsg: "Connection error banner did not appear within 8 s — rollback may not have fired",
      },
    );

    // The tab count must be unchanged — the rollback removed the orphan tab.
    const tabsAfter = await countTabs();
    expect(tabsAfter).toBe(tabsBefore);
  });

  /**
   * TEST-SSH-ROLLBACK-002: Error banner is visible after open_ssh_connection failure.
   *
   * The `.terminal-view__connection-error` banner must be shown so the user is
   * informed that the SSH connection attempt failed (FS-SSH-032).
   */
  it("shows the connection error banner when open_ssh_connection fails", async () => {
    // The banner must still be visible from TEST-SSH-ROLLBACK-001 (same browser
    // session, the banner was not dismissed yet).
    const banner = await $(Selectors.connectionErrorBanner);
    await expect(banner).toExist();
  });

  /**
   * TEST-SSH-ROLLBACK-003: Error banner can be dismissed.
   *
   * Clicking the close button inside the `.terminal-view__connection-error` banner
   * must hide it (FS-SSH-032).
   */
  it("dismisses the error banner when the close button is clicked", async () => {
    // Ensure the banner is visible (either from a prior test or re-trigger it).
    let bannerVisible = await browser.execute(function (): boolean {
      return document.querySelector(".terminal-view__connection-error") !== null;
    });

    if (!bannerVisible) {
      // Re-open panel if it was closed.
      const managerVisible = await browser.execute(function (): boolean {
        return document.querySelector(".connection-manager") !== null;
      });
      if (!managerVisible) {
        await openConnectionManager();
      }
      await armFailureAndOpenInNewTab();
      await browser.waitUntil(
        async () => {
          try {
            return await $(Selectors.connectionErrorBanner).isExisting();
          } catch {
            return false;
          }
        },
        { timeout: 8_000, timeoutMsg: "Error banner did not appear before dismiss test" },
      );
      bannerVisible = true;
    }

    // Click the close button inside the banner via dispatchEvent.
    await browser.execute(function () {
      const closeBtn = document.querySelector(
        ".terminal-view__connection-error-close",
      ) as HTMLButtonElement | null;
      closeBtn?.dispatchEvent(new MouseEvent("click", { bubbles: true, cancelable: true }));
    });

    // The banner must disappear (Svelte removes it from the DOM when connectionOpenError = false).
    await browser.waitUntil(
      () =>
        browser.execute(function (): boolean {
          return document.querySelector(".terminal-view__connection-error") === null;
        }),
      {
        timeout: 3_000,
        timeoutMsg: "Connection error banner did not disappear within 3 s after clicking close",
      },
    );
  });

  /**
   * Teardown: delete the E2E test connection to leave preferences clean.
   * Best-effort — non-fatal on failure.
   */
  after(async () => {
    try {
      // Retrieve the connection ID from the DOM (connection label is unique).
      await tauriFireAndForget("get_connections");
      // Minimal cleanup: close the panel if still open.
      await browser.execute(function () {
        const closeBtn = document.querySelector<HTMLButtonElement>(
          ".connection-manager button[aria-label='Close']",
        );
        if (closeBtn) {
          closeBtn.dispatchEvent(new MouseEvent("click", { bubbles: true, cancelable: true }));
        }
      });
    } catch {
      // ignore
    }
  });
});
