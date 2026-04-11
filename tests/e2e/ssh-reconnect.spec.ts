// SPDX-License-Identifier: MPL-2.0

/**
 * E2E scenario: SSH disconnected banner and reconnect flow.
 *
 * Covers FS-SSH-040, FS-SSH-041, FS-SSH-042 for the observable UI behaviour
 * when an SSH session drops and the user triggers a reconnect.
 *
 * Strategy:
 *   1. Open an SSH connection via the ConnectionManager (pointing at 127.0.0.1:1
 *      — nothing listening there, guaranteed ECONNREFUSED after a synthetic
 *      delay so the Connecting overlay is visible first).
 *   2. Once the Connecting overlay appears, inject a synthetic Disconnected
 *      event via `inject_ssh_disconnect` — this exercises the banner code path
 *      without depending on real TCP failure timing.
 *   3. Assert the `.terminal-pane__ssh-disconnected` banner is visible.
 *   4. Click the Reconnect button — this invokes `reconnect_ssh` IPC.
 *   5. Arm a new `inject_ssh_delay` so the reconnect stays in Connecting state
 *      long enough to observe it.
 *   6. Assert the `.ssh-connecting-overlay` appears (Connecting state).
 *
 * No real SSH server is needed. The connection target (127.0.0.1:1) will fail
 * with ECONNREFUSED after the injected delay, but by that point the test has
 * already observed all required states.
 *
 * Build requirement:
 *   Binary MUST be built with --features e2e-testing.
 *
 * Protocol references:
 *   - FS-SSH-040 (disconnected banner), FS-SSH-041 (reason text), FS-SSH-042 (reconnect)
 *   - docs/test-protocols/functional-pty-vt-ssh-preferences-ui-ipc.md §4.5
 */

import { browser, $ } from '@wdio/globals';
import { Selectors } from './helpers/selectors';

// ---------------------------------------------------------------------------
// IPC helpers
// ---------------------------------------------------------------------------

/**
 * Invoke a Tauri IPC command and return its result.
 *
 * Uses the window-variable polling pattern (same as ssh-overlay-states.spec.ts)
 * to work around `browser.executeAsync` unreliability with tauri-driver /
 * WebKitGTK.
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
            r ?? '__e2e_null__';
        })
        .catch(function () {
          (window as unknown as Record<string, unknown>).__e2e_invoke_result = '__e2e_error__';
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

  if (raw === '__e2e_error__') {
    throw new Error(`tauriInvoke("${cmd}") rejected`);
  }
  return (raw === '__e2e_null__' ? null : raw) as T;
}

// ---------------------------------------------------------------------------
// ConnectionManager helpers (mirrors ssh-overlay-states.spec.ts)
// ---------------------------------------------------------------------------

async function openConnectionManager(): Promise<void> {
  await $(Selectors.terminalGrid).click();
  const sshBtn = await $(Selectors.sshButton);
  await expect(sshBtn).toExist();

  await browser.execute(function () {
    const btn = document.querySelector('.terminal-view__ssh-btn') as HTMLButtonElement | null;
    btn?.dispatchEvent(new MouseEvent('click', { bubbles: true, cancelable: true }));
  });

  await browser.waitUntil(
    async () => {
      try {
        return await $(Selectors.connectionManager).isExisting();
      } catch {
        return false;
      }
    },
    { timeout: 5_000, timeoutMsg: 'ConnectionManager did not appear within 5 s' },
  );
}

async function typeIntoInput(id: string, value: string): Promise<void> {
  const el = await browser.$(`#${id}`);
  await el.click();
  await browser.execute(
    function (elId: string, val: string): void {
      const input = document.getElementById(elId) as HTMLInputElement | null;
      if (!input) return;
      input.focus();
      const proto = Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, 'value');
      proto?.set?.call(input, val);
      input.dispatchEvent(
        new InputEvent('input', {
          bubbles: true,
          cancelable: false,
          data: val,
          inputType: 'insertText',
        }),
      );
      input.dispatchEvent(new Event('change', { bubbles: true }));
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
 * Create a connection entry in the ConnectionManager pointing at 127.0.0.1:1.
 * Uses password auth to avoid identity_file path validation.
 */
async function createReconnectTestConnection(): Promise<void> {
  await browser.execute(function () {
    const actionsEl = document.querySelector('.connection-manager__actions');
    const btn = actionsEl?.querySelector('button');
    btn?.dispatchEvent(new MouseEvent('click', { bubbles: true, cancelable: true }));
  });

  await browser.waitUntil(
    () =>
      browser.execute(function (): boolean {
        return document.getElementById('cm-host') !== null;
      }),
    { timeout: 3_000, timeoutMsg: 'ConnectionManager form (#cm-host) did not appear' },
  );

  await typeIntoInput('cm-label', 'E2E Reconnect Test');
  await typeIntoInput('cm-host', '127.0.0.1');
  await typeIntoInput('cm-port', '1');
  await typeIntoInput('cm-username', 'e2e-reconnect-user');

  await browser.execute(function () {
    const radio = document.querySelector<HTMLInputElement>(
      'input[name="cm-auth-method"][value="password"]',
    );
    if (!radio) return;
    radio.checked = true;
    radio.dispatchEvent(new Event('change', { bubbles: true }));
  });

  await browser.waitUntil(
    () =>
      browser.execute(function (): boolean {
        const radio = document.querySelector<HTMLInputElement>(
          'input[name="cm-auth-method"][value="password"]',
        );
        return radio !== null && radio.checked;
      }),
    { timeout: 3_000, timeoutMsg: 'Password auth radio did not become checked' },
  );

  await browser.execute(function () {
    const form = document.querySelector('.connection-manager__form');
    if (!form) return;
    const buttons = form.querySelectorAll<HTMLButtonElement>('button');
    if (buttons.length > 0) {
      buttons[0].dispatchEvent(new MouseEvent('click', { bubbles: true, cancelable: true }));
    }
  });

  await browser.waitUntil(
    () =>
      browser.execute(function (): boolean {
        return (
          document.querySelector('.connection-manager__actions') !== null &&
          document.getElementById('cm-host') === null &&
          document.querySelectorAll('.connection-manager__item').length > 0
        );
      }),
    { timeout: 8_000, timeoutMsg: 'ConnectionManager did not show a connection item after save' },
  );
}

/** Click "Open in new tab" for the first connection in the list. */
async function openFirstConnectionInNewTab(): Promise<void> {
  await browser.execute(function () {
    const actionGroup = document.querySelector<HTMLElement>('.connection-manager__item-actions');
    if (!actionGroup) return;
    const buttons = actionGroup.querySelectorAll<HTMLButtonElement>(
      '.connection-manager__action-btn',
    );
    if (buttons.length > 0) {
      buttons[0].dispatchEvent(new MouseEvent('click', { bubbles: true, cancelable: true }));
    }
  });
}

// ---------------------------------------------------------------------------
// Suite
// ---------------------------------------------------------------------------

describe('TauTerm — SSH disconnected banner and reconnect (FS-SSH-040/041/042)', () => {
  before(async () => {
    await openConnectionManager();
    await createReconnectTestConnection();
  });

  /**
   * TEST-SSH-RECONNECT-001: disconnected banner is visible when SSH state is Disconnected.
   *
   * GIVEN an SSH pane that has just been connected
   * WHEN inject_ssh_disconnect is called for that pane
   * THEN .terminal-pane__ssh-disconnected is visible in the DOM
   * AND a Reconnect button (.terminal-pane__ssh-reconnect-btn) is present in the banner
   */
  it('TEST-SSH-RECONNECT-001: disconnected banner appears after inject_ssh_disconnect', async () => {
    // Ensure ConnectionManager is open.
    const managerVisible = await browser.execute(function (): boolean {
      return document.querySelector('.connection-manager') !== null;
    });
    if (!managerVisible) {
      await openConnectionManager();
    }

    // Arm a 2-second delay so connect_task stays in Connecting state long enough
    // for us to grab the paneId from the overlay before the task times out.
    await tauriInvoke<void>('inject_ssh_delay', { delayMs: 2000 });

    // Open the connection in a new tab.
    await openFirstConnectionInNewTab();

    // Wait for the connecting overlay to appear — this gives us the pane ID.
    await browser.waitUntil(
      () =>
        browser.execute(function (): boolean {
          return document.querySelector('.ssh-connecting-overlay') !== null;
        }),
      {
        timeout: 3_000,
        timeoutMsg: '.ssh-connecting-overlay did not appear within 3 s',
      },
    );

    // Extract paneId from the overlay's ancestor .terminal-pane.
    const paneId = await browser.execute(function (): string | null {
      const overlay = document.querySelector('.ssh-connecting-overlay');
      if (!overlay) return null;
      const pane = overlay.closest<HTMLElement>('.terminal-pane');
      return pane?.dataset.paneId ?? null;
    });
    expect(paneId).toBeTruthy();

    // Inject a synthetic Disconnected event — immediately moves pane to disconnected state.
    await tauriInvoke<void>('inject_ssh_disconnect', { paneId });

    // Connecting overlay must disappear.
    await browser.waitUntil(
      () =>
        browser.execute(function (): boolean {
          return document.querySelector('.ssh-connecting-overlay') === null;
        }),
      { timeout: 2_000, timeoutMsg: '.ssh-connecting-overlay did not disappear after disconnect' },
    );

    // Disconnected banner must appear.
    await browser.waitUntil(
      () =>
        browser.execute(function (): boolean {
          return document.querySelector('.terminal-pane__ssh-disconnected') !== null;
        }),
      {
        timeout: 2_000,
        timeoutMsg: '.terminal-pane__ssh-disconnected banner did not appear within 2 s',
      },
    );

    // Reconnect button must be present inside the banner.
    const reconnectBtnPresent = await browser.execute(function (): boolean {
      return document.querySelector('.terminal-pane__ssh-reconnect-btn') !== null;
    });
    expect(reconnectBtnPresent).toBe(true);
  });

  /**
   * TEST-SSH-RECONNECT-002: clicking Reconnect transitions pane to Connecting state.
   *
   * GIVEN a pane in Disconnected state (banner visible from TEST-SSH-RECONNECT-001)
   * WHEN the user clicks the Reconnect button
   * THEN the disconnected banner disappears
   * AND the .ssh-connecting-overlay appears (pane enters Connecting state)
   *
   * The reconnect_ssh IPC command spawns a new connect_task. We arm another
   * inject_ssh_delay so the task stays in Connecting state long enough for
   * WebdriverIO to assert the overlay.
   */
  it('TEST-SSH-RECONNECT-002: clicking Reconnect shows Connecting overlay', async () => {
    // The disconnected banner must still be visible (continuation from -001).
    const bannerPresent = await browser.execute(function (): boolean {
      return document.querySelector('.terminal-pane__ssh-disconnected') !== null;
    });
    if (!bannerPresent) {
      // Guard: if -001 left the app in a different state, the test cannot continue.
      throw new Error(
        'TEST-SSH-RECONNECT-002 requires the disconnected banner to be present. ' +
          'Ensure TEST-SSH-RECONNECT-001 ran first and left the pane in Disconnected state.',
      );
    }

    // Arm a 2-second delay for the reconnect connect_task so the overlay is
    // observable before the TCP failure resolves.
    await tauriInvoke<void>('inject_ssh_delay', { delayMs: 2000 });

    // Click the Reconnect button.
    await browser.execute(function (): void {
      const btn = document.querySelector<HTMLButtonElement>(
        '.terminal-pane__ssh-reconnect-btn',
      );
      btn?.dispatchEvent(new MouseEvent('click', { bubbles: true, cancelable: true }));
    });

    // The disconnected banner must disappear (pane left Disconnected state).
    await browser.waitUntil(
      () =>
        browser.execute(function (): boolean {
          return document.querySelector('.terminal-pane__ssh-disconnected') === null;
        }),
      {
        timeout: 3_000,
        timeoutMsg: '.terminal-pane__ssh-disconnected banner did not disappear after Reconnect click',
      },
    );

    // The connecting overlay must appear (pane entered Connecting state).
    await browser.waitUntil(
      () =>
        browser.execute(function (): boolean {
          return document.querySelector('.ssh-connecting-overlay') !== null;
        }),
      {
        timeout: 2_000,
        timeoutMsg:
          '.ssh-connecting-overlay did not appear after Reconnect click — ' +
          'pane may not have entered Connecting state',
      },
    );
  });

  after(async () => {
    // Best-effort cleanup: close the SSH tab opened by this suite.
    try {
      const tabCount = await browser.execute(
        (): number => document.querySelectorAll('.tab-bar__tab').length,
      );
      if (tabCount > 1) {
        await browser.execute((): void => {
          (document.querySelector('.terminal-grid') as HTMLElement | null)?.dispatchEvent(
            new KeyboardEvent('keydown', {
              key: 'W',
              code: 'KeyW',
              ctrlKey: true,
              shiftKey: true,
              bubbles: true,
              cancelable: true,
            }),
          );
        });
        await browser.waitUntil(
          async () =>
            (await browser.execute(
              (): number => document.querySelectorAll('.tab-bar__tab').length,
            )) < tabCount,
          { timeout: 3_000, timeoutMsg: 'SSH tab did not close' },
        );
      }
    } catch {
      // ignore
    }

    // Close the ConnectionManager if still open.
    try {
      await browser.execute(function () {
        const closeBtn = document.querySelector<HTMLButtonElement>(
          ".connection-manager button[aria-label='Close']",
        );
        if (closeBtn) {
          closeBtn.dispatchEvent(new MouseEvent('click', { bubbles: true, cancelable: true }));
        }
      });
    } catch {
      // ignore
    }
  });
});
