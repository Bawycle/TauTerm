// SPDX-License-Identifier: MPL-2.0

/**
 * E2E scenario: SSH connecting overlay visibility (UXD §7.5.2, FS-SSH-015).
 *
 * Covered:
 *   TEST-SSH-OVERLAY-001 — .ssh-connecting-overlay is present in the DOM
 *                          while the connection is in `connecting` state.
 *   TEST-SSH-OVERLAY-002 — .ssh-connecting-overlay is removed once the
 *                          connection attempt ends (failed TCP → Disconnected).
 *
 * Mechanism:
 *   `inject_ssh_delay({ delay_ms: 2000 })` arms a synthetic delay at the
 *   start of `connect_task` (e2e-testing feature only).  The delay fires
 *   *after* `open_connection_inner` has already emitted the `Connecting`
 *   state event and the overlay is rendered, holding it visible for 2 s so
 *   WebdriverIO has time to assert it.
 *
 *   The test connection points to 127.0.0.1:9999 (nothing listening there),
 *   so after the delay `connect_task` gets ECONNREFUSED immediately and the
 *   backend emits `Disconnected` — the overlay disappears.
 *
 *   No real SSH server is needed.
 *
 * Prerequisites:
 *   Binary must be built with `--features e2e-testing`.
 */

import { browser, $ } from "@wdio/globals";
import { Selectors } from "./helpers/selectors";

// ---------------------------------------------------------------------------
// IPC helpers (mirrors ssh-connection-rollback.spec.ts)
// ---------------------------------------------------------------------------

/**
 * Invoke a Tauri IPC command and return its result.
 *
 * Stores the resolved value in `window.__e2e_invoke_result` then polls until
 * it appears — workaround for `browser.executeAsync` unreliability with
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
          (window as unknown as Record<string, unknown>).__e2e_invoke_result =
            r ?? "__e2e_null__";
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

// ---------------------------------------------------------------------------
// ConnectionManager helpers
// ---------------------------------------------------------------------------

async function openConnectionManager(): Promise<void> {
  await $(Selectors.terminalGrid).click();
  const sshBtn = await $(Selectors.sshButton);
  await expect(sshBtn).toExist();

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
 * Fill a form input via native InputEvent dispatch so Svelte 5 sees the change.
 */
async function typeIntoInput(id: string, value: string): Promise<void> {
  const el = await browser.$(`#${id}`);
  await el.click();
  await browser.execute(
    function (elId: string, val: string): void {
      const input = document.getElementById(elId) as HTMLInputElement | null;
      if (!input) return;
      input.focus();
      const proto = Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, "value");
      proto?.set?.call(input, val);
      input.dispatchEvent(
        new InputEvent("input", {
          bubbles: true,
          cancelable: false,
          data: val,
          inputType: "insertText",
        }),
      );
      input.dispatchEvent(new Event("change", { bubbles: true }));
    },
    id,
    value,
  );
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

/**
 * Create a connection to 127.0.0.1:9999 via the ConnectionManager form.
 *
 * Port 9999 has nothing listening in the E2E environment, so the TCP
 * connection will fail immediately with ECONNREFUSED after the injected delay
 * expires — without needing a real SSH server.
 */
async function createOverlayTestConnection(): Promise<void> {
  await browser.execute(function () {
    const actionsEl = document.querySelector(".connection-manager__actions");
    const btn = actionsEl?.querySelector("button");
    btn?.dispatchEvent(new MouseEvent("click", { bubbles: true, cancelable: true }));
  });

  await browser.waitUntil(
    () =>
      browser.execute(function (): boolean {
        return document.getElementById("cm-host") !== null;
      }),
    { timeout: 3_000, timeoutMsg: "ConnectionManager form (#cm-host) did not appear" },
  );

  await typeIntoInput("cm-label", "E2E Overlay Test");
  await typeIntoInput("cm-host", "127.0.0.1");
  await typeIntoInput("cm-port", "9999");
  await typeIntoInput("cm-username", "e2e-overlay-user");

  // Switch to password auth to avoid identity_file path validation.
  await browser.execute(function () {
    const radio = document.querySelector<HTMLInputElement>(
      'input[name="cm-auth-method"][value="password"]',
    );
    if (!radio) return;
    radio.checked = true;
    radio.dispatchEvent(new Event("change", { bubbles: true }));
  });

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

  // Click Save (first button in the form footer).
  await browser.execute(function () {
    const form = document.querySelector(".connection-manager__form");
    if (!form) return;
    const buttons = form.querySelectorAll<HTMLButtonElement>("button");
    if (buttons.length > 0) {
      buttons[0].dispatchEvent(new MouseEvent("click", { bubbles: true, cancelable: true }));
    }
  });

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

/** Click "Open in new tab" for the first connection in the list. */
async function openFirstConnectionInNewTab(): Promise<void> {
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

describe("TauTerm — SSH connecting overlay (UXD §7.5.2, FS-SSH-015)", () => {
  before(async () => {
    await openConnectionManager();
    await createOverlayTestConnection();
  });

  /**
   * TEST-SSH-OVERLAY-001: .ssh-connecting-overlay is visible while connecting.
   *
   * Arms a 2-second delay in `connect_task` so the connection stays in
   * `connecting` state long enough for the assertion.  The overlay must be
   * present in the DOM within 1 s of clicking "Open in new tab".
   */
  it("renders .ssh-connecting-overlay while SSH state is connecting", async () => {
    // Ensure the ConnectionManager is open with items.
    const managerVisible = await browser.execute(function (): boolean {
      return document.querySelector(".connection-manager") !== null;
    });
    if (!managerVisible) {
      await openConnectionManager();
    }

    // Arm 2-second delay so connect_task pauses in Connecting state.
    // Tauri 2 serialises Rust snake_case params as camelCase on the JS side.
    await tauriInvoke<void>("inject_ssh_delay", { delayMs: 2000 });

    // Trigger the connection — command returns Ok() and the tab is created.
    await openFirstConnectionInNewTab();

    // The overlay must appear within 1 s (the Connecting event fires before
    // the command even returns, so rendering should be near-instant).
    await browser.waitUntil(
      () =>
        browser.execute(function (): boolean {
          return document.querySelector(".ssh-connecting-overlay") !== null;
        }),
      {
        timeout: 1_000,
        timeoutMsg: ".ssh-connecting-overlay did not appear within 1 s of opening SSH connection",
      },
    );
  });

  /**
   * TEST-SSH-OVERLAY-002: .ssh-connecting-overlay disappears after connection ends.
   *
   * Continues from TEST-SSH-OVERLAY-001: the 2-second delay expires, then
   * `connect_task` tries TCP connect to 127.0.0.1:9999 (ECONNREFUSED) and
   * the backend emits `Disconnected`.  The overlay must be removed from the DOM.
   */
  it("removes .ssh-connecting-overlay once the connection attempt ends", async () => {
    // Wait for overlay to disappear: delay (2 s) + TCP failure + event round-trip.
    // Generous 8-second timeout covers slow CI environments.
    await browser.waitUntil(
      () =>
        browser.execute(function (): boolean {
          return document.querySelector(".ssh-connecting-overlay") === null;
        }),
      {
        timeout: 8_000,
        timeoutMsg:
          ".ssh-connecting-overlay was still present 8 s after opening SSH connection — " +
          "overlay may not react to Disconnected state",
      },
    );
  });

  after(async () => {
    // Best-effort: close the ConnectionManager and any SSH tabs opened by this suite.
    try {
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
