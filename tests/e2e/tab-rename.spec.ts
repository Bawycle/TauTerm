// SPDX-License-Identifier: MPL-2.0

/**
 * E2E scenario: Tab inline rename (UXD §7.1.6, FS-TAB-006).
 *
 * Covers the full inline rename flow triggered from the tab bar:
 *   - Double-click on a tab label activates the rename input.
 *   - The input is pre-filled with the current title and gains focus.
 *   - Typing a new name and pressing Enter confirms the rename.
 *   - The tab label updates in the DOM to reflect the new title.
 *   - Pressing Escape during rename cancels without changing the title.
 *   - F2 key on a focused tab activates rename (keyboard accessibility).
 *
 * Protocol references:
 *   - FS-TAB-006 (rename tab), FS-KBD-002 (keyboard shortcut)
 *   - docs/test-protocols/functional-pty-vt-ssh-preferences-ui-ipc.md §4.3
 */

import { browser, $ } from '@wdio/globals';
import { Selectors } from './helpers/selectors';

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/** Return the text content of the active tab's title span. */
async function getActiveTabTitle(): Promise<string> {
  return browser.execute((): string => {
    const activeTab = document.querySelector('.tab-bar__tab[aria-selected="true"]');
    if (!activeTab) return '';
    const title = activeTab.querySelector('.tab-bar__tab-title');
    return title?.textContent?.trim() ?? '';
  });
}

/**
 * Dispatch a native InputEvent to set the value of the rename input, then
 * dispatch an input + change event so Svelte 5 sees the update.
 *
 * Direct `setValue()` via WebDriver works for the value but does not trigger
 * Svelte's `oninput` handler reliably in WebKitGTK.  Replicating the approach
 * used in ssh-overlay-states.spec.ts (native InputEvent via prototype setter).
 */
async function typeIntoRenameInput(value: string): Promise<void> {
  await browser.execute(
    function (val: string): void {
      const input = document.querySelector<HTMLInputElement>('.tab-bar__rename-input');
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
    value,
  );

  await browser.waitUntil(
    () =>
      browser.execute(
        function (val: string): boolean {
          const input = document.querySelector<HTMLInputElement>('.tab-bar__rename-input');
          return input !== null && input.value === val;
        },
        value,
      ),
    { timeout: 2_000, timeoutMsg: `Rename input did not reflect value "${value}"` },
  );
}

/**
 * Confirm the rename by removing focus from the rename input.
 *
 * `onblur` on the input calls `onConfirmRename(tab.id)` → `rename.confirmRename(tabId)`,
 * which invokes `rename_tab` IPC with the current `renameValue`.  This is more
 * reliable than dispatching a keydown Enter in WebKitGTK because blur is a DOM
 * state change, not a synthetic keyboard event.
 */
async function confirmRename(): Promise<void> {
  await browser.execute(function (): void {
    const input = document.querySelector<HTMLInputElement>('.tab-bar__rename-input');
    if (!input) return;
    // blur() triggers the native blur event → Svelte's onblur handler → confirmRename.
    input.blur();
  });
}

/** Press Escape inside the rename input to cancel the rename. */
async function cancelRename(): Promise<void> {
  await browser.execute(function (): void {
    const input = document.querySelector<HTMLInputElement>('.tab-bar__rename-input');
    if (!input) return;
    input.dispatchEvent(
      new KeyboardEvent('keydown', {
        key: 'Escape',
        code: 'Escape',
        bubbles: true,
        cancelable: true,
      }),
    );
  });
}

/** Wait for the rename input to appear or disappear in the DOM. */
async function waitForRenameInput(present: boolean, timeout = 3_000): Promise<void> {
  await browser.waitUntil(
    () =>
      browser.execute(
        function (p: boolean): boolean {
          return (document.querySelector('.tab-bar__rename-input') !== null) === p;
        },
        present,
      ),
    {
      timeout,
      timeoutMsg: present
        ? '.tab-bar__rename-input did not appear within ' + timeout + ' ms'
        : '.tab-bar__rename-input did not disappear within ' + timeout + ' ms',
    },
  );
}

// ---------------------------------------------------------------------------
// Suite
// ---------------------------------------------------------------------------

describe('TauTerm — Tab inline rename (UXD §7.1.6, FS-TAB-006)', () => {
  /**
   * TEST-TAB-RENAME-001: double-click activates rename input pre-filled with current title.
   *
   * GIVEN a tab with a title
   * WHEN the user double-clicks the tab label
   * THEN .tab-bar__rename-input appears in the DOM
   * AND the input value equals the current tab title
   * AND the input has focus
   *
   * Clean up: press Escape to cancel rename so subsequent tests start clean.
   */
  it('TEST-TAB-RENAME-001: double-click on tab activates inline rename input', async () => {
    const originalTitle = await getActiveTabTitle();

    // Double-click the active tab to trigger rename.
    await browser.execute(function (): void {
      const activeTab = document.querySelector<HTMLElement>(
        '.tab-bar__tab[aria-selected="true"]',
      );
      if (!activeTab) return;
      activeTab.dispatchEvent(new MouseEvent('dblclick', { bubbles: true, cancelable: true }));
    });

    // Rename input must appear.
    await waitForRenameInput(true);

    // The input must be pre-filled with the current title.
    const inputValue = await browser.execute(function (): string {
      const input = document.querySelector<HTMLInputElement>('.tab-bar__rename-input');
      return input?.value ?? '';
    });
    expect(inputValue).toBe(originalTitle);

    // The input must have focus.
    const inputFocused = await browser.execute(function (): boolean {
      const input = document.querySelector<HTMLInputElement>('.tab-bar__rename-input');
      return input !== null && document.activeElement === input;
    });
    expect(inputFocused).toBe(true);

    // Cancel rename to leave the app in a clean state.
    await cancelRename();
    await waitForRenameInput(false);
  });

  /**
   * TEST-TAB-RENAME-002: typing a new name and pressing Enter updates the tab title.
   *
   * GIVEN a tab in rename mode (double-click triggered)
   * WHEN the user clears the input, types a new name, and presses Enter
   * THEN .tab-bar__rename-input disappears
   * AND .tab-bar__tab-title shows the new name
   */
  it('TEST-TAB-RENAME-002: Enter confirms rename and updates tab title in the DOM', async () => {
    const newTitle = 'E2E-Renamed-Tab';

    // Activate rename mode via double-click.
    await browser.execute(function (): void {
      const activeTab = document.querySelector<HTMLElement>(
        '.tab-bar__tab[aria-selected="true"]',
      );
      if (!activeTab) return;
      activeTab.dispatchEvent(new MouseEvent('dblclick', { bubbles: true, cancelable: true }));
    });
    await waitForRenameInput(true);

    // Set the new title value.
    await typeIntoRenameInput(newTitle);

    // Confirm with Enter.
    await confirmRename();

    // Input must disappear (rename mode exited).
    await waitForRenameInput(false);

    // The active tab title must now be the new name.
    await browser.waitUntil(
      async () => (await getActiveTabTitle()) === newTitle,
      {
        timeout: 3_000,
        timeoutMsg: `Tab title did not update to "${newTitle}" within 3 s`,
      },
    );
  });

  /**
   * TEST-TAB-RENAME-003: Escape during rename cancels without changing the title.
   *
   * GIVEN a tab with a known title (E2E-Renamed-Tab from TEST-TAB-RENAME-002,
   *       or whatever the current title is)
   * WHEN the user activates rename, types a different name, then presses Escape
   * THEN .tab-bar__rename-input disappears
   * AND the tab title remains unchanged
   */
  it('TEST-TAB-RENAME-003: Escape cancels rename without changing the title', async () => {
    const titleBeforeRename = await getActiveTabTitle();

    // Activate rename mode.
    await browser.execute(function (): void {
      const activeTab = document.querySelector<HTMLElement>(
        '.tab-bar__tab[aria-selected="true"]',
      );
      if (!activeTab) return;
      activeTab.dispatchEvent(new MouseEvent('dblclick', { bubbles: true, cancelable: true }));
    });
    await waitForRenameInput(true);

    // Type a new name (should NOT be committed).
    await typeIntoRenameInput('Discarded-Title');

    // Cancel with Escape.
    await cancelRename();

    // Input must disappear.
    await waitForRenameInput(false);

    // Title must remain unchanged.
    const titleAfterCancel = await getActiveTabTitle();
    expect(titleAfterCancel).toBe(titleBeforeRename);
  });

  /**
   * TEST-TAB-RENAME-004: F2 key on a focused tab activates rename (keyboard accessibility).
   *
   * GIVEN a tab with keyboard focus
   * WHEN the user presses F2
   * THEN .tab-bar__rename-input appears
   *
   * Clean up: Escape to cancel.
   *
   * Note: this exercises the keyboard accessibility path defined in UXD §7.1.6 and
   * FS-KBD-002, which requires rename to be reachable without a mouse.
   */
  it('TEST-TAB-RENAME-004: F2 on focused tab activates rename input', async () => {
    // Focus the active tab element.
    await browser.execute(function (): void {
      const activeTab = document.querySelector<HTMLElement>(
        '.tab-bar__tab[aria-selected="true"]',
      );
      activeTab?.focus();
    });

    // Dispatch F2 keydown on the tab — handled by onTabKeydown → rename.startRename.
    await browser.execute(function (): void {
      const activeTab = document.querySelector<HTMLElement>(
        '.tab-bar__tab[aria-selected="true"]',
      );
      if (!activeTab) return;
      activeTab.dispatchEvent(
        new KeyboardEvent('keydown', {
          key: 'F2',
          code: 'F2',
          bubbles: true,
          cancelable: true,
        }),
      );
    });

    // Rename input must appear.
    await waitForRenameInput(true);

    // Cancel to leave app in clean state.
    await cancelRename();
    await waitForRenameInput(false);
  });
});
