// SPDX-License-Identifier: MPL-2.0

/**
 * E2E scenario: SSH credential dialog visibility.
 *
 * Covered:
 *   TEST-SSH-CRED-001 — .ssh-credential-dialog renders when a
 *                       `credential-prompt` event is emitted.
 *   TEST-SSH-CRED-002 — .ssh-credential-dialog disappears after the
 *                       user submits the dialog.
 *
 * Mechanism:
 *   `inject_credential_prompt` directly emits a `credential-prompt` event
 *   without requiring a live SSH auth flow (e2e-testing feature only).
 *   This exercises the full frontend rendering path:
 *     Tauri event → onCredentialPrompt → setCredentialPrompt →
 *     _ssh.credentialPrompt reactive state → SshCredentialDialog open=true.
 *
 *   On submit, `handleProvideCredentials` clears the state (dialog closes)
 *   then calls `provide_credentials` IPC which returns NoPendingCredentials
 *   (silently swallowed by catch {}).
 *
 * Prerequisites:
 *   Binary must be built with `--features e2e-testing`.
 */

import { browser, $ } from "@wdio/globals";

// ---------------------------------------------------------------------------
// IPC helpers (mirrors ssh-overlay-states.spec.ts)
// ---------------------------------------------------------------------------

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
        .catch(function (err: unknown) {
          (window as unknown as Record<string, unknown>).__e2e_invoke_result =
            "__e2e_error__:" + String(err);
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

  if (typeof raw === "string" && raw.startsWith("__e2e_error__:")) {
    throw new Error(`tauriInvoke("${cmd}") rejected: ${raw.slice("__e2e_error__:".length)}`);
  }
  return (raw === "__e2e_null__" ? null : raw) as T;
}

type PaneNode =
  | { type: "leaf"; paneId: string }
  | { type: "split"; first: PaneNode; second: PaneNode };

function firstPaneIdFromNode(node: PaneNode): string | null {
  if (node.type === "leaf") return node.paneId;
  return firstPaneIdFromNode(node.first) ?? firstPaneIdFromNode(node.second);
}

/** Retrieve the first available pane ID from session state. */
async function getFirstPaneId(): Promise<string> {
  type SessionState = {
    tabs: Array<{ layout: PaneNode }>;
  };
  const state = await tauriInvoke<SessionState>("get_session_state");
  const layout = state?.tabs?.[0]?.layout;
  const paneId = layout ? firstPaneIdFromNode(layout) : null;
  if (!paneId) throw new Error("No pane found in session state");
  return paneId;
}

// ---------------------------------------------------------------------------
// Suite
// ---------------------------------------------------------------------------

describe("TauTerm — SSH credential dialog", () => {
  /**
   * TEST-SSH-CRED-001: .ssh-credential-dialog is visible after a
   * credential-prompt event is injected.
   */
  it("renders .ssh-credential-dialog when credential-prompt event fires", async () => {
    const paneId = await getFirstPaneId();

    await tauriInvoke<void>("inject_credential_prompt", {
      paneId,
      host: "test.example.com",
      username: "e2e-user",
    });

    await browser.waitUntil(
      () =>
        browser.execute(function (): boolean {
          return document.querySelector(".ssh-credential-dialog") !== null;
        }),
      {
        timeout: 3_000,
        timeoutMsg:
          ".ssh-credential-dialog did not appear within 3 s of inject_credential_prompt",
      },
    );
  });

  /**
   * TEST-SSH-CRED-002: .ssh-credential-dialog disappears after submit.
   *
   * Continues from TEST-SSH-CRED-001: dialog is open; fill in a password
   * and click "Connect".  The dialog must disappear.
   */
  it("removes .ssh-credential-dialog after user submits", async () => {
    // Type a password into the password field.
    await browser.execute(function () {
      const input = document.getElementById("ssh-credential-password") as HTMLInputElement | null;
      if (!input) return;
      input.focus();
      const proto = Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, "value");
      proto?.set?.call(input, "test-password");
      input.dispatchEvent(new InputEvent("input", { bubbles: true, data: "test-password" }));
    });

    // Wait for the input value to be reflected.
    await browser.waitUntil(
      () =>
        browser.execute(function (): boolean {
          const input = document.getElementById("ssh-credential-password") as HTMLInputElement | null;
          return input !== null && input.value === "test-password";
        }),
      { timeout: 2_000, timeoutMsg: "Password input did not reflect value" },
    );

    // Click the "Connect" button (primary button in dialog footer).
    await browser.execute(function () {
      // Find the primary button inside the dialog content area.
      const dialog = document.querySelector("[role='dialog']");
      if (!dialog) return;
      const btns = dialog.querySelectorAll<HTMLButtonElement>("button");
      // The "Connect" button is the last button (after "Cancel").
      const connectBtn = Array.from(btns).find((b) => !b.disabled && b.textContent?.trim() !== "");
      // Click the submit button — the one that is NOT disabled (has password value).
      const primaryBtn = Array.from(btns).filter((b) => !b.disabled).pop();
      primaryBtn?.dispatchEvent(new MouseEvent("click", { bubbles: true, cancelable: true }));
    });

    // Dialog must close within 3 s.
    await browser.waitUntil(
      () =>
        browser.execute(function (): boolean {
          return document.querySelector(".ssh-credential-dialog") === null;
        }),
      {
        timeout: 3_000,
        timeoutMsg: ".ssh-credential-dialog did not close after submit",
      },
    );
  });
});
