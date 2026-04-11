// SPDX-License-Identifier: MPL-2.0

/**
 * E2E regression test: `ls -al` basic flow.
 *
 * Regression guard for two known bugs:
 *   BUG-1: After `ls -al`, no new shell prompt appears (criteria 4.1 / UX-SC-17).
 *   BUG-2: After `ls -al`, typing produces nothing on screen (criteria 5.1 / UX-SC-20).
 *
 * The test uses `inject_pty_output` (requires --features e2e-testing) to push
 * a deterministic, fixed sequence through the VT pipeline, bypassing the real PTY.
 * Each injection phase is followed by a ≥50 ms settle before assertions
 * (debounce 12 ms in pty_task.rs; 50 ms is a conservative safety margin).
 *
 * Injection sequence (reference: CLAUDE.md §Injection sequence):
 *   Phase 1 — Initial prompt  : "user@tauterm:~$ "
 *   Phase 2 — Command echo    : "ls -al"
 *   Phase 3 — Command output  : "\r\n" + ls-like lines, each ending \r\n
 *   Phase 4 — Return prompt   : "\r\nuser@tauterm:~$ "
 *   Phase 5 — Second char echo: "e"  (regression for BUG-2)
 *
 * Build requirement:
 *   pnpm tauri build --no-bundle -- --features e2e-testing
 *
 * Protocol references:
 *   - Domain-expert success criteria 1.1–6.4
 *   - UX success criteria UX-SC-01–UX-SC-25
 *   - docs/test-protocols/functional-pty-vt-ssh-preferences-ui-ipc.md §4.1
 *
 * Note on browser.execute vs browser.executeAsync:
 *   All Tauri IPC calls use fire-and-forget (browser.execute, not
 *   browser.executeAsync) to avoid the tauri-driver/WebKitGTK done-callback
 *   stall that causes indefinite hangs with async scripts.
 */

import { browser, $ } from '@wdio/globals';
import { Selectors } from './helpers/selectors';

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const PROMPT = 'user@tauterm:~$ ';
const PROMPT_LEN = PROMPT.length; // 16

/**
 * Deterministic, fixed output for `ls -al`.
 * Small enough to fit in any reasonable terminal height (4 lines).
 * Each line ends with \r\n per POSIX PTY conventions (criterion 3.1).
 */
const LS_OUTPUT_LINES = [
  'total 32',
  'drwxr-xr-x  5 user group 4096 Jan 01 12:00 .',
  'drwxr-xr-x 12 user group 4096 Jan 01 12:00 ..',
  '-rw-r--r--  1 user group  123 Jan 01 12:00 Cargo.toml',
];

/** Full output block: \r\n prefix + lines joined by \r\n + trailing \r\n. */
const LS_OUTPUT_BLOCK = '\r\n' + LS_OUTPUT_LINES.map((l) => l + '\r\n').join('');

// ---------------------------------------------------------------------------
// IPC helpers (fire-and-forget pattern — see pty-roundtrip.spec.ts for rationale)
// ---------------------------------------------------------------------------

/**
 * Fire a Tauri IPC command without waiting for its return value.
 * `inject_pty_output` sends bytes to an unbounded mpsc channel and returns
 * immediately; DOM effects are asserted separately via waitUntil.
 */
function tauriFireAndForget(cmd: string, args?: Record<string, unknown>): Promise<void> {
  return browser.execute(
    function (cmdArg: string, argsArg: Record<string, unknown> | undefined) {
      (window as any).__TAURI_INTERNALS__.invoke(cmdArg, argsArg);
    },
    cmd,
    args,
  ) as unknown as Promise<void>;
}

/** Encode a string to a UTF-8 byte array for inject_pty_output. */
function encodeBytes(s: string): number[] {
  return [...new TextEncoder().encode(s)];
}

/**
 * Inject bytes into the given pane's VT pipeline via inject_pty_output.
 * Fire-and-forget; the caller must await DOM effects separately.
 */
async function inject(paneId: string, text: string): Promise<void> {
  await tauriFireAndForget('inject_pty_output', {
    paneId,
    data: encodeBytes(text),
  });
}

// ---------------------------------------------------------------------------
// DOM assertion helpers
// ---------------------------------------------------------------------------

/**
 * Return the full textContent of the terminal grid in a single RPC call.
 * Avoids per-cell RPC overhead (1920 cells per grid × per-call latency).
 */
async function getGridText(): Promise<string> {
  return browser.execute(
    (): string => document.querySelector('.terminal-grid')?.textContent ?? '',
  ) as Promise<string>;
}

/**
 * Wait until the terminal grid contains the given text fragment.
 */
async function waitForTextInGrid(text: string, timeoutMs = 10_000): Promise<void> {
  await browser.waitUntil(
    () =>
      browser.execute((t: string): boolean => {
        const grid = document.querySelector('.terminal-grid');
        return grid !== null && (grid.textContent ?? '').includes(t);
      }, text),
    {
      timeout: timeoutMs,
      timeoutMsg: `"${text}" did not appear in the terminal grid within ${timeoutMs / 1_000} s`,
    },
  );
}

/**
 * Return true if the terminal grid contains at least one non-empty,
 * non-NBSP cell — i.e. the grid is not blank.
 */
function gridHasNonEmptyCell(): Promise<boolean> {
  return browser.execute((): boolean => {
    for (const cell of document.querySelectorAll('.terminal-pane__cell')) {
      const text = cell.textContent ?? '';
      if (text.trim().length > 0 && text !== '\u00a0') return true;
    }
    return false;
  }) as Promise<boolean>;
}

/**
 * Return true if .terminal-pane__cursor is present in the DOM.
 */
function cursorExists(): Promise<boolean> {
  return browser.execute((): boolean => {
    return document.querySelector('.terminal-pane__cursor') !== null;
  }) as Promise<boolean>;
}

/**
 * Return true if .terminal-pane__cursor is present AND does NOT have the
 * --unfocused modifier class (meaning the pane is active and focused).
 */
function cursorIsFocused(): Promise<boolean> {
  return browser.execute((): boolean => {
    const cursor = document.querySelector('.terminal-pane__cursor');
    if (!cursor) return false;
    return !cursor.classList.contains('terminal-pane__cursor--unfocused');
  }) as Promise<boolean>;
}

/**
 * Return the data-char attribute of the cursor element, or null if absent.
 */
function getCursorDataChar(): Promise<string | null> {
  return browser.execute((): string | null => {
    return document.querySelector('.terminal-pane__cursor')?.getAttribute('data-char') ?? null;
  }) as Promise<string | null>;
}

/**
 * Return the CSS `left` inline style of the cursor (e.g. "16ch").
 * Used to detect the cursor column position via the `{col}ch` pattern.
 * Returns null if the cursor is not in the DOM.
 */
function getCursorLeftStyle(): Promise<string | null> {
  return browser.execute((): string | null => {
    const el = document.querySelector<HTMLElement>('.terminal-pane__cursor');
    return el ? el.style.left : null;
  }) as Promise<string | null>;
}

/**
 * Parse the numeric column value from a cursor left style like "16ch".
 * Returns NaN if the style is missing or does not end with "ch".
 */
function parseCursorCol(leftStyle: string | null): number {
  if (!leftStyle || !leftStyle.endsWith('ch')) return NaN;
  return parseInt(leftStyle.slice(0, -2), 10);
}

/**
 * Return the full text content of the terminal grid, row by row.
 * Each entry in the returned array is the concatenated textContent of all
 * cells in that row (a `.terminal-pane__row`).
 */
function getGridRows(): Promise<string[]> {
  return browser.execute((): string[] => {
    return Array.from(document.querySelectorAll('.terminal-pane__row')).map((row) => {
      return Array.from(row.querySelectorAll('.terminal-pane__cell'))
        .map((cell) => {
          const t = cell.textContent ?? '';
          return t === '\u00a0' ? ' ' : t;
        })
        .join('');
    });
  }) as Promise<string[]>;
}

/**
 * Return the index of the last non-blank row in the grid (rows that contain
 * at least one non-space, non-NBSP character), or -1 if all rows are blank.
 */
function getLastOccupiedRowIndex(rows: string[]): number {
  for (let i = rows.length - 1; i >= 0; i--) {
    if (rows[i].trim().length > 0) return i;
  }
  return -1;
}

/**
 * Return true if the grid text contains raw VT escape sequences (criterion 3.5).
 * Looks for ESC character (\x1b) or the literal string "\033[" or bracket sequences
 * that would indicate unrendered ANSI codes.
 */
function gridContainsRawEscapes(): Promise<boolean> {
  return browser.execute((): boolean => {
    const text = document.querySelector('.terminal-grid')?.textContent ?? '';
    // ESC character
    if (text.includes('\x1b')) return true;
    // Literal escape representations sometimes rendered by buggy parsers
    if (text.includes('\\033[') || text.includes('^[')) return true;
    // CSI opener rendered as raw text (e.g. "[1;32m")
    // We check for ESC + '[' only — covered by the \x1b check above.
    return false;
  }) as Promise<boolean>;
}

// ---------------------------------------------------------------------------
// Suite
// ---------------------------------------------------------------------------

describe('ls -al basic flow', () => {
  let paneId: string;

  // -------------------------------------------------------------------------
  // beforeAll: obtain pane ID and establish known-good initial state
  // -------------------------------------------------------------------------

  before(async () => {
    // Dismiss any lingering close-confirmation dialog from a preceding spec.
    await browser.execute((): void => {
      const btn = document.querySelector<HTMLButtonElement>('[data-testid="close-confirm-cancel"]');
      if (btn) btn.dispatchEvent(new MouseEvent('click', { bubbles: true, cancelable: true }));
    });
    await browser.waitUntil(
      () =>
        browser.execute(
          (): boolean => document.querySelector('[data-testid="close-confirm-cancel"]') === null,
        ),
      { timeout: 3_000, timeoutMsg: 'Lingering close-confirmation dialog did not dismiss' },
    );

    // Wait for the active terminal pane to be present and carry a pane ID.
    await browser.waitUntil(
      async () => {
        try {
          return await $(Selectors.activeTerminalPane).isExisting();
        } catch {
          return false;
        }
      },
      { timeout: 15_000, timeoutMsg: 'Active terminal pane did not appear within 15 s' },
    );

    const rawId = await $(Selectors.activeTerminalPane).getAttribute('data-pane-id');
    expect(rawId).toBeTruthy();
    paneId = rawId as string;
  });

  // -------------------------------------------------------------------------
  // Phase 1 — Initial state (criteria 1.1–1.4, UX-SC-01–04)
  // -------------------------------------------------------------------------

  it('Phase 1 — initial state: prompt visible, cursor present and focused', async () => {
    // Inject the initial prompt (no trailing \r\n — cursor sits immediately
    // after the last prompt character, as a real shell produces).
    await inject(paneId, PROMPT);

    // Allow the VT pipeline to process and the DOM to update.
    // 50 ms > debounce (12 ms) + render frame (≈16 ms). (criterion 6.5)
    await browser.pause(50);

    // 1.1 / UX-SC-01: at least one non-empty, non-NBSP cell must be present.
    await browser.waitUntil(gridHasNonEmptyCell, {
      timeout: 10_000,
      timeoutMsg: 'No non-empty cell appeared after injecting initial prompt (criterion 1.1)',
    });

    // UX-SC-03: no raw escape sequences visible in the grid.
    const hasEscapes = await gridContainsRawEscapes();
    expect(hasEscapes).toBe(false); // criterion 3.5 / UX-SC-03

    // 1.2 / UX-SC-02: cursor element must be present.
    await browser.waitUntil(cursorExists, {
      timeout: 5_000,
      timeoutMsg:
        'Cursor (.terminal-pane__cursor) did not appear after prompt injection (criterion 1.2)',
    });

    // 1.2: cursor must not carry the --unfocused modifier.
    const focused = await cursorIsFocused();
    expect(focused).toBe(true); // criterion 1.2 / UX-SC-02

    // 1.3: data-char must be defined (carries the character under the cursor).
    const dataChar = await getCursorDataChar();
    expect(dataChar).not.toBeNull(); // criterion 1.3

    // 1.4 / UX-SC-02: cursor column must equal PROMPT_LEN (immediately after
    // the last prompt character). The style is "{col}ch".
    // Wait for cursor to be in blink-on phase before reading style.left.
    // The earlier waitUntil at the top of this phase may have been in-between
    // blink cycles by the time we reach here. Same race as Phase 4 (see comment there).
    await browser.waitUntil(cursorExists, {
      timeout: 5_000,
      timeoutMsg: 'Cursor did not enter blink-on phase before column read (Phase 1, criterion 1.4)',
    });
    const leftStyle = await getCursorLeftStyle();
    const cursorCol = parseCursorCol(leftStyle);
    expect(cursorCol).toBe(PROMPT_LEN); // criterion 1.4: col = length of prompt

    // UX-SC-04: terminal must not be obscured by a modal or preferences panel.
    // The selector is passed as a parameter so it stays in sync with Selectors.preferencesPanel.
    const panelOpen = (await browser.execute((sel: string): boolean => {
      return document.querySelector(sel) !== null;
    }, Selectors.preferencesPanel)) as boolean;
    expect(panelOpen).toBe(false); // UX-SC-04
  });

  // -------------------------------------------------------------------------
  // Phase 2 — Command echo (criteria 2.1–2.3, UX-SC-05–08)
  // -------------------------------------------------------------------------

  it('Phase 2 — command echo: "ls -al" appears on prompt line after typing', async () => {
    // Inject the 6-character echo as the PTY would echo the user's keystrokes.
    await inject(paneId, 'ls -al');
    await browser.pause(50); // criterion 6.5

    // 2.1 / UX-SC-08: the grid must contain "ls -al" somewhere.
    await waitForTextInGrid('ls -al');

    // 2.1: verify each character is present in the grid (individually).
    const gridText = await getGridText();
    expect(gridText).toContain('l'); // criterion 2.1
    expect(gridText).toContain('s'); // criterion 2.1
    expect(gridText).toContain('-'); // criterion 2.1
    expect(gridText).toContain('a'); // criterion 2.1

    // 2.2 / UX-SC-06: cursor must have advanced by 6 columns (PROMPT_LEN + 6).
    await browser.waitUntil(cursorExists, {
      timeout: 5_000,
      timeoutMsg: 'Cursor did not enter blink-on phase before column read (Phase 2, criterion 2.2)',
    });
    const leftStyle = await getCursorLeftStyle();
    const cursorCol = parseCursorCol(leftStyle);
    expect(cursorCol).toBe(PROMPT_LEN + 6); // criterion 2.2: col = 16 + 6 = 22

    // 2.3 / UX-SC-05: no duplicate characters — "ls -al" appears exactly once.
    // We count occurrences in grid text to guard against ghost rendering.
    const occurrences = (gridText.match(/ls -al/g) ?? []).length;
    expect(occurrences).toBe(1); // criterion 2.3 / UX-SC-07: no duplicates
  });

  // -------------------------------------------------------------------------
  // Phase 3 — Command output (criteria 3.1–3.6, UX-SC-09–16)
  // -------------------------------------------------------------------------

  it('Phase 3 — output: ls -al output rendered correctly without VT artefacts', async () => {
    // Inject the full output block (\r\n prefix + lines + \r\n terminators).
    await inject(paneId, LS_OUTPUT_BLOCK);
    await browser.pause(50); // criterion 6.5

    // UX-SC-09: cursor must have moved past the command line (output started).
    // We wait for "total " which is the first line of ls output.
    await waitForTextInGrid('total ', 10_000); // criterion 3.2 / UX-SC-11

    // 3.2: "total " present on a distinct line.
    const gridText = await getGridText();
    expect(gridText).toContain('total '); // criterion 3.2 / UX-SC-11

    // 3.4: "." and ".." entries present. (criterion 3.4)
    // We use the full path string from our deterministic output.
    expect(gridText).toContain('.'); // criterion 3.4 — "." entry
    expect(gridText).toContain('..'); // criterion 3.4 — ".." entry

    // 3.4: "Cargo.toml" present (third ls entry). (criterion 3.4)
    expect(gridText).toContain('Cargo.toml'); // criterion 3.4 / UX-SC-11

    // 3.5 / UX-SC-13: no raw VT escape sequences in the grid.
    const hasEscapes = await gridContainsRawEscapes();
    expect(hasEscapes).toBe(false); // criterion 3.5 / UX-SC-13

    // 3.3: each output line must begin at column 0 (\r resets to col 0).
    // We verify via the row structure: find rows containing known output text
    // and check they start with the expected content (no leading indent).
    const rows = await getGridRows();
    const totalRow = rows.find((r) => r.trimStart().startsWith('total '));
    expect(totalRow).toBeDefined(); // criterion 3.2
    // The row must begin at col 0 — i.e. the row string starts with "total "
    // (no leading spaces beyond what the command itself produces). (criterion 3.3)
    // We trim only trailing spaces, not leading ones:
    expect(totalRow?.trimEnd()).toMatch(/^total /); // criterion 3.3

    // 3.6 / UX-SC-14: SGR attributes (bold, colors) must not appear as literal
    // text. We cannot assert CSS properties via WebDriver but we can confirm
    // no "\x1b[" sequences leaked into cell textContent.
    // (Already covered by hasEscapes check above — documented explicitly here.)
    // TODO: cannot assert SGR CSS properties (bold, fg, bg) on individual cells
    // via WebDriver — would require reading computed styles for each cell.
    // Requires: a Tauri command exposing per-cell SGR state, or vitest unit test.

    // UX-SC-15: wrapping — no assertion possible without knowing terminal width.
    // TODO: cannot assert wrapping cleanness — requires reading terminal dimensions.
  });

  // -------------------------------------------------------------------------
  // Phase 4 — Return prompt (criteria 4.1–4.5, UX-SC-17–19)
  // REGRESSION: BUG-1 — no new prompt after ls -al
  // -------------------------------------------------------------------------

  it('Phase 4 — return prompt: new prompt appears after output on its own line', async () => {
    // Inject the return prompt: \r\n moves to a fresh line, then the prompt text.
    // Omitting this injection would reproduce BUG-1. (criterion 4.1)
    await inject(paneId, '\r\n' + PROMPT);
    await browser.pause(50); // criterion 6.5

    // REGRESSION: BUG-1 (criterion 4.1 / UX-SC-17): a new prompt instance must
    // appear after the ls output. We look for the prompt text in the grid.
    await waitForTextInGrid(PROMPT.trim(), 10_000);

    // 4.1 / UX-SC-18: the prompt must be on a line distinct from the last output line.
    // Strategy: the prompt row must come after all output rows in the DOM order.
    const rows = await getGridRows();
    const lastOutputRowIdx = rows.reduce((last, row, idx) => {
      if (row.includes('Cargo.toml')) return idx;
      return last;
    }, -1);
    // .terminal-pane__cursor is a sibling div to .terminal-pane__row, not
    // nested inside any cell — so its (empty) textContent does not contaminate
    // getGridRows(). The trimEnd().endsWith() check is therefore safe: the
    // return-prompt row ends with "user@tauterm:~$" (trailing space stripped)
    // with no cursor artifact appended.
    const promptRowIdx = rows.reduce((last, row, idx) => {
      if (row.trimEnd().endsWith(PROMPT.trimEnd())) return idx;
      return last;
    }, -1);

    // REGRESSION: BUG-1 — prompt row index must be found and must be after output.
    expect(promptRowIdx).toBeGreaterThan(-1); // criterion 4.1: prompt row exists
    expect(promptRowIdx).toBeGreaterThan(lastOutputRowIdx); // criterion 4.1: separate line

    // 4.2 / UX-SC-18: the return prompt must be the last occupied line.
    const lastOccupiedIdx = getLastOccupiedRowIndex(rows);
    expect(promptRowIdx).toBe(lastOccupiedIdx); // criterion 4.2

    // 4.3 / UX-SC-19: cursor must be on the return prompt row at col = PROMPT_LEN.
    // Wait for the cursor element to be in the DOM before querying style.left.
    // The blink timer may have left the cursor in its "off" phase at the end of
    // the 50 ms pause, causing the {#if} to hide it and getCursorLeftStyle() to
    // return null (→ NaN). waitUntil(cursorExists) synchronises with the "on"
    // phase before we read style.left, eliminating the race condition.
    await browser.waitUntil(cursorExists, {
      timeout: 5_000,
      timeoutMsg:
        'Cursor (.terminal-pane__cursor) did not appear after return-prompt injection (criterion 4.3)',
    });
    const leftStyle = await getCursorLeftStyle();
    const cursorCol = parseCursorCol(leftStyle);
    expect(cursorCol).toBe(PROMPT_LEN); // criterion 4.3

    // 4.4: cursor must be visible (not blink-hidden — we just injected, no blink gap).
    // cursorExists() already confirmed presence above; this re-checks for explicitness.
    const cursorPresent = await cursorExists();
    expect(cursorPresent).toBe(true); // criterion 4.4

    // 4.4: cursor must not be unfocused.
    const focused = await cursorIsFocused();
    expect(focused).toBe(true); // criterion 4.4

    // 4.5: data-char must not be a character from the ls output.
    const dataChar = await getCursorDataChar();
    // The cursor sits on the first blank cell after the prompt — data-char
    // should be a space or the cell content at that position.
    // We verify it is NOT a character exclusive to ls output like 'd' in "drwxr".
    // A more precise check is not possible without row/col attributes on the cursor.
    // TODO: cannot assert cursor row number precisely — the cursor element only
    // exposes column via "left:{col}ch"; row is via "--cursor-top:{row}lh" CSS var
    // but CSS custom properties are not exposed as DOM attributes in WebDriver.
    // Requires: a `data-row` attribute on the cursor element, or a Tauri command.
    expect(dataChar).not.toBeNull(); // criterion 4.5: at minimum, data-char is defined
  });

  // -------------------------------------------------------------------------
  // Phase 5 — Input responsiveness (criteria 5.1–5.3, UX-SC-20–21)
  // REGRESSION: BUG-2 — typing after ls -al produces nothing
  // -------------------------------------------------------------------------

  it('Phase 5 — input responsiveness: typing after the return prompt renders on screen', async () => {
    // REGRESSION: BUG-2 (criterion 5.1 / UX-SC-20): inject a single character 'e'
    // as the PTY would echo it after the return prompt.
    // If BUG-2 is present, this injection either does not update the DOM or the
    // screen buffer discards it.
    await inject(paneId, 'e');
    await browser.pause(50); // criterion 6.5

    // REGRESSION: BUG-2 — the sequence PROMPT+'e' must appear in the grid.
    // Searching for 'e' alone would be a false positive: the character already
    // appears in Phase 3 output ("drwxr-xr-x", "user", "group", etc.).
    // The discriminating substring is the full prompt (including its trailing
    // space, which is a distinct cell in the DOM) followed by 'e'.
    // Note: PROMPT.trimEnd() would drop the trailing space, producing
    // "user@tauterm:~$e" which never appears — the space cell is always present.
    await waitForTextInGrid(PROMPT + 'e', 5_000); // criterion 5.1 / UX-SC-20

    // 5.2: the cursor must have advanced by one column (col = PROMPT_LEN + 1).
    await browser.waitUntil(cursorExists, {
      timeout: 5_000,
      timeoutMsg: 'Cursor did not enter blink-on phase before column read (Phase 5, criterion 5.2)',
    });
    const leftStyle = await getCursorLeftStyle();
    const cursorCol = parseCursorCol(leftStyle);
    expect(cursorCol).toBe(PROMPT_LEN + 1); // criterion 5.2

    // 5.1 / UX-SC-20: verify 'e' is on the return-prompt line, not somewhere
    // in the ls output (which would be a false positive).
    const rows = await getGridRows();
    const lastOccupiedIdx = getLastOccupiedRowIndex(rows);
    const lastRow = rows[lastOccupiedIdx] ?? '';
    // The last occupied row must contain 'e' immediately after the prompt.
    expect(lastRow).toContain(PROMPT.trimEnd()); // prompt still present on that row
    expect(lastRow).toContain('e'); // 'e' typed after the prompt

    // 5.3: the pane must still be in Running state — not terminated.
    // ProcessTerminatedPane renders as .process-terminated-pane when the PTY
    // process exits. TerminalPane.svelte has no --terminated modifier class on
    // the wrapper div; the banner is a separate child element. (criterion 5.3)
    const terminated = (await browser.execute((sel: string): boolean => {
      return document.querySelector(sel) !== null;
    }, Selectors.processTerminatedPane)) as boolean;
    expect(terminated).toBe(false); // criterion 5.3
  });

  // -------------------------------------------------------------------------
  // Phase 6 — VT invariants (criteria 6.1–6.4, UX-SC-22–25)
  // -------------------------------------------------------------------------

  it('Phase 6 — VT invariants: no artefacts, no alternate screen, scroll_offset = 0', async () => {
    // 6.1: alternate screen not active. The terminal-grid must still be the
    // primary screen content — we verify by confirming the prompt text is still
    // visible (alternate screen would clear it).
    const gridText = await getGridText();
    expect(gridText).toContain(PROMPT.trimEnd()); // criterion 6.1

    // UX-SC-22: no duplicated output lines.
    const totalCount = (gridText.match(/total 32/g) ?? []).length;
    expect(totalCount).toBe(1); // criterion 6.1 / UX-SC-22

    // UX-SC-23: no ghost prompt between output and return prompt.
    // Count occurrences of the prompt string — there should be exactly 2:
    // one from the initial inject and one from the return prompt.
    // NOTE: because cells render individual characters, the grid textContent
    // concatenates all rows without row separators, so prompt occurrences
    // can only be detected by substring matching.
    const promptOccurrences = (() => {
      let count = 0;
      let pos = 0;
      const needle = PROMPT.trimEnd();
      while ((pos = gridText.indexOf(needle, pos)) !== -1) {
        count++;
        pos += needle.length;
      }
      return count;
    })();
    // Exactly 2: initial prompt + return prompt (UX-SC-23: no ghost prompt).
    expect(promptOccurrences).toBe(2); // UX-SC-23

    // 3.5 / UX-SC-13: final check — no raw escape sequences in the full grid.
    const hasEscapes = await gridContainsRawEscapes();
    expect(hasEscapes).toBe(false); // criterion 3.5

    // 6.2: scroll_offset must be 0 — the view is not scrolled back.
    // TODO: cannot assert scroll_offset = 0 directly — it is internal Rust state.
    // Requires: a Tauri command exposing scroll_offset per pane, or a
    // `data-scroll-offset` attribute on .terminal-pane. (criterion 6.2)

    // 6.3: DECCKM mode = false after ls -al.
    // TODO: cannot assert DECCKM state from the DOM — it is internal VT parser state.
    // Requires: a Tauri command exposing the VT parser mode flags. (criterion 6.3)

    // 6.4: scrollbackLines coherence.
    // TODO: cannot assert scrollbackLines from the DOM — it is internal screen buffer state.
    // Requires: a `data-scrollback-lines` attribute on .terminal-pane or a Tauri command.
    // (criterion 6.4)

    // UX-SC-24: output not truncated — all known output lines present.
    for (const line of LS_OUTPUT_LINES) {
      // Each output line must appear in the grid (trimming trailing spaces).
      expect(gridText).toContain(line.trim()); // UX-SC-24
    }

    // UX-SC-25: scroll offset = 0 when typing — cursor is at col PROMPT_LEN + 1
    // (from phase 5 injection), meaning the view is at the bottom.
    // Absence of a scrollbar element confirms scroll_offset = 0 from the DOM side.
    // (The scrollbar only renders when scrollbackLines > 0 per TerminalPane.svelte.)
    const scrollbarVisible = await browser.execute((): boolean => {
      return document.querySelector('.terminal-pane__scrollbar') !== null;
    });
    // The scrollbar renders only when scrollbackLines > 0 (TerminalPane.svelte:322).
    // Its absence confirms no scrollback history was accumulated — which is
    // expected for a short ls -al run that does not overflow the viewport.
    // Note: this asserts scrollbackLines = 0, NOT scroll_offset = 0.
    // Asserting scroll_offset = 0 requires a Tauri command or data attribute
    // not currently present. (criterion 6.2 / UX-SC-25)
    expect(scrollbarVisible).toBe(false); // UX-SC-25: no scrollback history
  });
});

// ---------------------------------------------------------------------------
// Regression: atomic single-burst injection (grid resize + full output)
// ---------------------------------------------------------------------------
//
// The phased injection in the suite above uses 50 ms pauses between each
// phase. This is sufficient to trigger debounce flush but does NOT reproduce
// the real shell behaviour: a shell sends the entire output — command echo,
// ls output, and return prompt — as a single PTY write, which arrives in one
// screen-update event burst. The grid-mismatch bug (grid not resized before
// applyUpdates) is only reliably triggered when the resize and the large
// output arrive in the same burst without a chance for an intermediate render.
//
// This suite injects the complete sequence in a single inject() call to guard
// that specific regression path.

describe('ls -al single-burst injection (grid resize regression)', () => {
  let paneId: string;

  before(async () => {
    // Dismiss any lingering close-confirmation dialog from the preceding suite.
    await browser.execute((): void => {
      const btn = document.querySelector<HTMLButtonElement>('[data-testid="close-confirm-cancel"]');
      if (btn) btn.dispatchEvent(new MouseEvent('click', { bubbles: true, cancelable: true }));
    });
    await browser.waitUntil(
      () =>
        browser.execute(
          (): boolean => document.querySelector('[data-testid="close-confirm-cancel"]') === null,
        ),
      { timeout: 3_000, timeoutMsg: 'Lingering close-confirmation dialog did not dismiss' },
    );

    await browser.waitUntil(
      async () => {
        try {
          return await $(Selectors.activeTerminalPane).isExisting();
        } catch {
          return false;
        }
      },
      { timeout: 15_000, timeoutMsg: 'Active terminal pane did not appear within 15 s' },
    );

    const rawId = await $(Selectors.activeTerminalPane).getAttribute('data-pane-id');
    expect(rawId).toBeTruthy();
    paneId = rawId as string;
  });

  it('renders complete output from atomic single-burst injection', async () => {
    // Build the full sequence as one contiguous string, exactly as a real shell
    // would write it to the PTY in a single burst:
    //   initial prompt → command echo → output lines → return prompt
    //
    // No pauses between phases — the entire string arrives in one inject() call
    // and is processed by the VT pipeline in a single event burst.
    const burst = PROMPT + 'ls -al' + LS_OUTPUT_BLOCK + '\r\n' + PROMPT;

    await inject(paneId, burst);

    // Wait until the burst is fully rendered: the return prompt must be on its
    // own row AND appear after the Cargo.toml output row.  We cannot use
    // waitForTextInGrid(PROMPT.trim()) here because the prompt text already
    // exists in the grid from the preceding suite — that signal fires
    // immediately before the burst is processed, causing a race condition.
    // Instead we poll the rows directly until both structural conditions hold.
    let rows: string[] = [];
    let lastOutputRowIdx = -1;
    let promptRowIdx = -1;
    await browser.waitUntil(
      async () => {
        rows = await getGridRows();
        lastOutputRowIdx = rows.reduce((last, row, idx) => {
          if (row.includes('Cargo.toml')) return idx;
          return last;
        }, -1);
        promptRowIdx = rows.reduce((last, row, idx) => {
          if (row.trimEnd().endsWith(PROMPT.trimEnd())) return idx;
          return last;
        }, -1);
        return promptRowIdx > -1 && promptRowIdx > lastOutputRowIdx;
      },
      {
        timeout: 15_000,
        timeoutMsg:
          'Return prompt did not appear on its own row after Cargo.toml within 15 s',
      },
    );

    // All ls output lines must be present in the grid (no truncation due to
    // grid being too small after the implicit resize that the burst may trigger).
    const gridText = await getGridText();
    for (const line of LS_OUTPUT_LINES) {
      expect(gridText).toContain(line.trim());
    }

    expect(promptRowIdx).toBeGreaterThan(-1); // return prompt found
    expect(promptRowIdx).toBeGreaterThan(lastOutputRowIdx); // prompt after output

    // No raw escape sequences must have leaked through.
    const hasEscapes = await gridContainsRawEscapes();
    expect(hasEscapes).toBe(false);
  });
});
