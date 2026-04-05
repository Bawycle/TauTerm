# Test Protocol — TauTerm Minor Sprint

> **Document status:** Initial revision — 2026-04-05
> **Author:** test-engineer
> **Based on:** FS.md §3.7 (FS-SB-007), §3.8 (FS-SEARCH-002), §3.14 (FS-CLIP-009), §3.15 (FS-UX-002), §3.17 (FS-I18N-001), §3.1.8 (FS-VT-073); UXD.md §7.3.2 (TUITC-UX-060); test-protocols/ui-terminal-components.md §3.11
> **Related report:** docs/test-protocols/report-major-sprint-2026-04-05.md (pre-existing failures flagged)

---

## 1. Scope

This protocol covers seven scenarios identified during the major sprint review as gaps or regressions requiring focused testing:

| Scenario ID | Feature reference | Short description |
|---|---|---|
| TP-MIN-001 | FS-SB-007 | Scrollbar click interaction |
| TP-MIN-002 | FS-SB-007 | Scrollbar drag interaction |
| TP-MIN-003 | FS-SB-007 | Scrollbar mouse wheel |
| TP-MIN-004 | FS-UX-002 | First-launch hint appears |
| TP-MIN-005 | FS-UX-002 | Hint disappears after right-click |
| TP-MIN-006 | FS-I18N-001 | No hardcoded strings in TabBar (EN + FR) |
| TP-MIN-007 | FS-I18N-001 | No hardcoded strings in TerminalPane (EN + FR) |
| TP-MIN-008 | FS-I18N-001 | No hardcoded strings in TerminalView (EN + FR) |
| TP-MIN-009 | FS-VT-073 | `file://` accepted for local PTY session |
| TP-MIN-010 | FS-VT-073 | `file://` rejected for SSH session |
| TP-MIN-011 | FS-VT-073 | `javascript:` always rejected |
| TP-MIN-012 | FS-VT-073 | `data:` always rejected |
| TP-MIN-013 | FS-VT-073 | URI > 2048 chars rejected |
| TP-MIN-014 | FS-VT-073 | URI with C0 control characters rejected |
| TP-MIN-015 | FS-CLIP-009 | Multiline paste confirmation dialog — shown when needed |
| TP-MIN-016 | FS-CLIP-009 | Confirmation dialog not shown when bracketed paste active |
| TP-MIN-017 | FS-CLIP-009 | Confirmation dialog not shown for single-line paste |
| TP-MIN-018 | TUITC-UX-060 | Inactive tab title contrast ≥ 4.5:1 |
| TP-MIN-019 | SEARCH-SOFT-001 / FS-SEARCH-002 | Soft-wrapped word found by search |

---

## 2. Test Layer Assignment

| Scenario ID | Layer | Runner |
|---|---|---|
| TP-MIN-001 – TP-MIN-003 | E2E | WebdriverIO + tauri-driver |
| TP-MIN-004 – TP-MIN-005 | E2E | WebdriverIO + tauri-driver |
| TP-MIN-006 – TP-MIN-008 | Unit (Frontend) | vitest (static analysis + runtime) |
| TP-MIN-009 – TP-MIN-014 | Unit (Rust) | cargo nextest |
| TP-MIN-015 – TP-MIN-017 | Unit (Frontend) | vitest |
| TP-MIN-018 | Unit (Frontend) | vitest (contrast calculation) |
| TP-MIN-019 | Unit (Rust) | cargo nextest |

---

## 3. Scenarios

---

### TP-MIN-001

**Scenario ID:** TP-MIN-001
**Feature reference:** FS-SB-007
**Description:** Scrollbar click — clicking in the scrollbar track above the thumb scrolls up; clicking below the thumb scrolls down.
**Layer:** E2E

**Preconditions:**
- TauTerm is open with one local PTY pane.
- The pane has been fed at least 200 lines of output so the scrollback is non-empty.
- The viewport is at the bottom (scroll_offset = 0).
- The scrollbar is visible.

**Steps:**
1. Locate the scrollbar element (`data-testid="scrollbar-track"` or equivalent).
2. Click in the track area above the scrollbar thumb.
3. Record the new `scroll_offset` (or inspect viewport position via `data-scroll-offset`).
4. Click in the track area below the current thumb position.
5. Record the resulting scroll_offset.

**Expected result:**
- After step 2: the viewport has scrolled up (scroll_offset > 0). The thumb has moved upward within the track.
- After step 4: the viewport has scrolled back down toward the bottom.

**Pass criteria:** Scroll position changes in the expected direction on each click. The terminal content updates within 500ms of each click. No layout shift occurs (pane width unchanged).

**Fail criteria:** Click has no effect on scroll position; or the viewport moves in the wrong direction; or a layout shift is observed.

---

### TP-MIN-002

**Scenario ID:** TP-MIN-002
**Feature reference:** FS-SB-007
**Description:** Scrollbar drag — dragging the thumb changes the scroll position proportionally.
**Layer:** E2E

**Preconditions:**
- Same as TP-MIN-001.
- Viewport is at the bottom.

**Steps:**
1. Locate the scrollbar thumb element (`data-testid="scrollbar-thumb"` or equivalent).
2. Record its current bounding box position (Y coordinate).
3. Drag the thumb upward by 50px using `browser.action("pointer")` drag simulation.
4. Wait for the scroll position to update (`waitUntil` scroll_offset > 0).
5. Record the new scroll_offset.
6. Drag the thumb back to the bottom of the track.
7. Record the resulting scroll_offset.

**Expected result:**
- After step 4: scroll_offset is proportionally greater than 0, consistent with the thumb position within the track (within ±5% tolerance).
- After step 6: scroll_offset is 0 or the minimum possible value (at-bottom).

**Pass criteria:** Thumb movement and scroll_offset are proportionally consistent. The relationship between thumb position and buffer offset is linear (±5%). No content corruption or display artifacts.

**Fail criteria:** Dragging thumb produces no scroll movement; proportionality is not respected; or display artifacts appear.

---

### TP-MIN-003

**Scenario ID:** TP-MIN-003
**Feature reference:** FS-SB-007
**Description:** Mouse wheel over the terminal area scrolls the scrollback.
**Layer:** E2E

**Preconditions:**
- Same as TP-MIN-001.
- Viewport is at the bottom.

**Steps:**
1. Position the mouse cursor over the terminal pane content area.
2. Scroll the mouse wheel upward by 3 clicks (`browser.action("wheel")` with deltaY = -300).
3. Record the scroll_offset.
4. Scroll the mouse wheel downward by 3 clicks (deltaY = +300).
5. Record the scroll_offset.

**Expected result:**
- After step 2: scroll_offset > 0. The viewport shows content above the current bottom.
- After step 4: scroll_offset decreases (returns toward 0).

**Pass criteria:** Mouse wheel controls scroll position. Scroll direction matches wheel direction (upward wheel = scroll up = shows older content). Smooth update with no visual flicker.

**Fail criteria:** Wheel has no effect; scroll direction is inverted; content flickers or renders incorrectly.

---

### TP-MIN-004

**Scenario ID:** TP-MIN-004
**Feature reference:** FS-UX-002
**Description:** First-launch hint is visible on a clean install / fresh state.
**Layer:** E2E

**Preconditions:**
- The TauTerm preferences file (`preferences.json`) does not exist, or `hints.right_click_seen` is `false` (or absent).
- TauTerm is started fresh.

**Steps:**
1. Launch TauTerm.
2. Wait for the main window to render.
3. Inspect the terminal area for a visible hint element (e.g., `data-testid="right-click-hint"`).

**Expected result:** A hint element is visible in the terminal area. The hint text references right-clicking (retrieved via locale catalogue — not hardcoded). The hint does not block keyboard input to the terminal.

**Pass criteria:** The hint element is present in the DOM and `isDisplayed()` returns true. The terminal pane is interactable (typing a character is not blocked by the hint overlay). The hint text matches the active locale string for the right-click hint key.

**Fail criteria:** No hint visible; hint blocks terminal input; hint text is a raw key string (untranslated).

---

### TP-MIN-005

**Scenario ID:** TP-MIN-005
**Feature reference:** FS-UX-002
**Description:** First-launch hint disappears after the user right-clicks, and does not reappear on next launch.
**Layer:** E2E

**Preconditions:**
- Same initial state as TP-MIN-004 (hint is visible after fresh start).

**Steps:**
1. Launch TauTerm (hint is visible per TP-MIN-004).
2. Right-click anywhere within the terminal pane content area.
3. Wait 200ms for UI update.
4. Check whether the hint element is still visible.
5. Dismiss the context menu (press Escape).
6. Close and relaunch TauTerm.
7. Check whether the hint element is visible on second launch.

**Expected result:**
- After step 4: the hint element is no longer visible (`isDisplayed()` returns false or element is removed from DOM).
- After step 7: the hint element remains absent on subsequent launch. The preference `hints.right_click_seen = true` persisted.

**Pass criteria:** Hint disappears immediately after first right-click. Does not reappear on subsequent launches.

**Fail criteria:** Hint remains after right-click; hint reappears on next launch; context menu does not open on right-click.

---

### TP-MIN-006

**Scenario ID:** TP-MIN-006
**Feature reference:** FS-I18N-001
**Description:** No user-visible string is hardcoded in `TabBar.svelte` — all strings are sourced from Paraglide message accessors.
**Layer:** Unit (Frontend) — static analysis

**Preconditions:**
- Source file: `src/lib/components/TabBar.svelte` (or equivalent path).
- Paraglide catalogue: `src/lib/i18n/messages/en.json` and `fr.json`.

**Steps:**
1. Parse `TabBar.svelte` source.
2. For each text node and attribute value (`aria-label`, `title`, `placeholder`, `alt`), check whether it is:
   a. A Paraglide accessor call: `{m.some_key()}` or `m.some_key()`, or
   b. A non-translatable proper noun (e.g., "TauTerm").
3. Identify any literal string that is neither of the above.
4. For each accessor key found in step 2a, verify the key exists in `en.json` with a non-empty value.
5. Repeat step 4 for `fr.json`.

**Expected result:** Zero hardcoded user-visible strings. Every Paraglide key used in the component exists in both `en.json` and `fr.json` with non-empty values.

**Pass criteria:** No violations found in steps 3 and 4/5.

**Fail criteria:** Any literal string that is not a Paraglide accessor or an acceptable proper noun; any key missing from either catalogue; any key mapping to an empty string.

---

### TP-MIN-007

**Scenario ID:** TP-MIN-007
**Feature reference:** FS-I18N-001
**Description:** No user-visible string is hardcoded in `TerminalPane.svelte` — all strings are sourced from Paraglide.
**Layer:** Unit (Frontend) — static analysis

**Preconditions:**
- Source file: `src/lib/components/TerminalPane.svelte` (or equivalent path).
- Same Paraglide catalogues as TP-MIN-006.

**Steps:**
1–5. Same procedure as TP-MIN-006, applied to `TerminalPane.svelte`.

**Expected result:** Zero hardcoded user-visible strings. All keys exist in both catalogues with non-empty values.

**Pass criteria / Fail criteria:** Same as TP-MIN-006.

---

### TP-MIN-008

**Scenario ID:** TP-MIN-008
**Feature reference:** FS-I18N-001
**Description:** No user-visible string is hardcoded in `TerminalView.svelte` — all strings are sourced from Paraglide.
**Layer:** Unit (Frontend) — static analysis

**Preconditions:**
- Source file: `src/routes/+page.svelte` or `src/lib/components/TerminalView.svelte` (whichever component implements the top-level terminal view).
- Same Paraglide catalogues as TP-MIN-006.

**Steps:**
1–5. Same procedure as TP-MIN-006.

**Expected result and Pass/Fail criteria:** Same as TP-MIN-006.

---

### TP-MIN-009

**Scenario ID:** TP-MIN-009
**Feature reference:** FS-VT-073
**Description:** A `file://` URI in an OSC 8 hyperlink is accepted when the session is a local PTY session.
**Layer:** Unit (Rust)

**Preconditions:**
- Test module in `src-tauri/src/vt/` (hyperlink or URI validation module).
- A `SessionContext` value indicating `SessionType::LocalPty`.

**Steps:**
1. Construct a URI validator / `HyperlinkUri::validate("file:///home/user/doc.txt", SessionType::LocalPty)` (or equivalent function signature).
2. Assert the result.

**Expected result:** The call returns `Ok(())` or the equivalent accepted/valid result.

**Pass criteria:** `file://` URIs are accepted for local PTY sessions. No panic.

**Fail criteria:** The call returns an error or rejection for a valid local session context.

---

### TP-MIN-010

**Scenario ID:** TP-MIN-010
**Feature reference:** FS-VT-073
**Description:** A `file://` URI in an OSC 8 hyperlink is rejected when the session is SSH.
**Layer:** Unit (Rust)

**Preconditions:**
- Same module as TP-MIN-009.
- A `SessionContext` value indicating `SessionType::Ssh`.

**Steps:**
1. Call the URI validator with `"file:///etc/passwd"` and `SessionType::Ssh`.
2. Assert the result.

**Expected result:** The call returns `Err(UriValidationError::FileSchemeNotAllowedInSsh)` or equivalent rejection.

**Pass criteria:** `file://` URIs are rejected for SSH sessions.

**Fail criteria:** The call returns Ok; file URI is accepted for SSH.

---

### TP-MIN-011

**Scenario ID:** TP-MIN-011
**Feature reference:** FS-VT-073
**Description:** A `javascript:` URI is rejected for both local PTY and SSH sessions.
**Layer:** Unit (Rust)

**Preconditions:**
- Same module as TP-MIN-009.

**Steps:**
1. Call the URI validator with `"javascript:alert(1)"` and `SessionType::LocalPty`. Assert rejection.
2. Call the URI validator with `"javascript:alert(1)"` and `SessionType::Ssh`. Assert rejection.

**Expected result:** Both calls return an error value. The error variant identifies the scheme as disallowed.

**Pass criteria:** Both calls reject the URI regardless of session type.

**Fail criteria:** Either call accepts the URI.

---

### TP-MIN-012

**Scenario ID:** TP-MIN-012
**Feature reference:** FS-VT-073
**Description:** A `data:` URI is rejected for both local PTY and SSH sessions.
**Layer:** Unit (Rust)

**Preconditions:**
- Same module as TP-MIN-009.

**Steps:**
1. Call the URI validator with `"data:text/html,<h1>test</h1>"` and `SessionType::LocalPty`. Assert rejection.
2. Call the URI validator with the same URI and `SessionType::Ssh`. Assert rejection.

**Expected result:** Both calls return an error identifying `data:` as a disallowed scheme.

**Pass criteria:** Both calls reject the URI.

**Fail criteria:** Either call accepts the URI.

---

### TP-MIN-013

**Scenario ID:** TP-MIN-013
**Feature reference:** FS-VT-073
**Description:** A URI exceeding 2048 characters is rejected.
**Layer:** Unit (Rust)

**Preconditions:**
- Same module as TP-MIN-009.
- A URI string of exactly 2049 characters with a valid `https://` scheme.

**Steps:**
1. Construct a URI: `"https://example.com/" + "a".repeat(2029)` (total length = 2049 characters).
2. Call the URI validator with this URI and `SessionType::LocalPty`. Assert rejection.
3. Construct a URI of exactly 2048 characters. Call the validator. Assert acceptance.

**Expected result:**
- Step 2: rejected with a length-exceeded error.
- Step 3: accepted.

**Pass criteria:** The boundary at 2048 characters is enforced correctly (2048 = accept, 2049 = reject).

**Fail criteria:** 2049-char URI is accepted; 2048-char URI is rejected; boundary is off by one.

---

### TP-MIN-014

**Scenario ID:** TP-MIN-014
**Feature reference:** FS-VT-073
**Description:** A URI containing C0 control characters or a null byte is rejected.
**Layer:** Unit (Rust)

**Preconditions:**
- Same module as TP-MIN-009.

**Steps:**
1. Call the URI validator with `"https://example.com/\x00"` (null byte). Assert rejection.
2. Call the URI validator with `"https://example.com/\x07"` (BEL, C0). Assert rejection.
3. Call the URI validator with `"https://example.com/\x1b[A"` (ESC, C0). Assert rejection.
4. Call the URI validator with `"https://example.com/"` (clean URI). Assert acceptance.

**Expected result:** Steps 1–3 return errors. Step 4 returns Ok.

**Pass criteria:** All control-character variants are rejected. Clean URI is accepted.

**Fail criteria:** Any control-character URI is accepted; clean URI is rejected.

---

### TP-MIN-015

**Scenario ID:** TP-MIN-015
**Feature reference:** FS-CLIP-009
**Description:** A confirmation dialog is shown when the user pastes multiline text and bracketed paste mode is inactive.
**Layer:** Unit (Frontend) — vitest

**Preconditions:**
- Component under test: the paste-confirmation dialog (or the paste-handling logic in `src/lib/terminal/` or `src/lib/state/`).
- Bracketed paste mode is `false` (inactive).
- Clipboard content contains at least one newline character.

**Steps:**
1. Mount (or invoke) the paste handler with `bracketedPasteActive = false` and `text = "first line\nsecond line"`.
2. Check whether the confirmation dialog is triggered/visible.

**Expected result:** The confirmation dialog is shown (component rendered, or a signal/callback indicating the dialog should open is triggered).

**Pass criteria:** Dialog appears when `bracketedPasteActive = false` and text contains `\n`.

**Fail criteria:** Dialog does not appear; paste proceeds without confirmation.

---

### TP-MIN-016

**Scenario ID:** TP-MIN-016
**Feature reference:** FS-CLIP-009
**Description:** No confirmation dialog is shown when bracketed paste mode is active, even with multiline text.
**Layer:** Unit (Frontend) — vitest

**Preconditions:**
- Same setup as TP-MIN-015.
- Bracketed paste mode is `true` (active).

**Steps:**
1. Invoke the paste handler with `bracketedPasteActive = true` and `text = "first line\nsecond line"`.
2. Check whether the confirmation dialog is triggered.

**Expected result:** The dialog is not shown. The text is sent directly (wrapped in bracketed-paste escape sequences per FS-CLIP-008).

**Pass criteria:** No dialog; text is forwarded wrapped in `ESC[200~` / `ESC[201~`.

**Fail criteria:** Dialog appears despite bracketed paste being active.

---

### TP-MIN-017

**Scenario ID:** TP-MIN-017
**Feature reference:** FS-CLIP-009
**Description:** No confirmation dialog is shown when the pasted text contains no newlines, even with bracketed paste inactive.
**Layer:** Unit (Frontend) — vitest

**Preconditions:**
- Same setup as TP-MIN-015.
- Bracketed paste mode is `false`.
- Clipboard text contains no newline characters.

**Steps:**
1. Invoke the paste handler with `bracketedPasteActive = false` and `text = "singlelinecommand"`.
2. Check whether the confirmation dialog is triggered.

**Expected result:** No dialog. Text is pasted directly (legacy behavior, no wrapping).

**Pass criteria:** No dialog for single-line paste with bracketed paste inactive.

**Fail criteria:** Dialog appears for single-line paste.

---

### TP-MIN-018

**Scenario ID:** TP-MIN-018
**Feature reference:** TUITC-UX-060 / FS-A11Y-001
**Description:** Inactive tab title text achieves a contrast ratio of at least 4.5:1 against the tab bar background (WCAG 2.1 AA, SC 1.4.3).
**Layer:** Unit (Frontend) — vitest (contrast calculation)

**Background:** Prior sprint report (report-major-sprint-2026-04-05.md) flagged the inactive tab title contrast at approximately 2.5:1. The UXD.md §7 (TUITC-UX-060 correction block) raised the token `--color-tab-inactive-fg` to `#9c9890` (neutral-400) to achieve ~6.0:1. This test verifies the corrected values.

**Preconditions:**
- Design tokens: `--color-tab-inactive-fg` and `--color-tab-bg` are defined in the theme.
- A WCAG contrast ratio calculation function is available (e.g., `src/lib/preferences/contrast.ts`).

**Steps:**
1. Read the token values:
   - Foreground: `--color-tab-inactive-fg` (expected: `#9c9890` per UXD correction).
   - Background: `--color-tab-bg` (expected: `#242118`).
2. Compute the relative luminance for each color.
3. Compute the contrast ratio: `(L_lighter + 0.05) / (L_darker + 0.05)`.
4. Assert ratio ≥ 4.5.

**Expected result:** Contrast ratio ≥ 4.5:1.

**Pass criteria:** Computed ratio is at or above 4.5. Specific expectation: ~6.0:1 with corrected tokens.

**Fail criteria:** Ratio < 4.5:1. Any value below this threshold is a WCAG AA failure and a regression of the TUITC-UX-060 fix.

---

### TP-MIN-019

**Scenario ID:** TP-MIN-019
**Feature reference:** SEARCH-SOFT-001 / FS-SEARCH-002
**Description:** A word that spans two soft-wrapped lines (not a hard newline) is found when searching for that word.
**Layer:** Unit (Rust)

**Background:** Prior sprint report (report-major-sprint-2026-04-05.md) identified that `search_scrollback` searched line-by-line and did not join soft-wrapped lines, preventing words spanning a wrap boundary from being found. This test verifies the fix is in place.

**Preconditions:**
- A screen buffer (80 columns) is constructed with a soft-wrapped line.
- The `search_scrollback` function or equivalent is callable from a test.

**Steps:**
1. Construct a screen buffer with width = 10 columns.
2. Append a line with content `"helloworld!"` — at 10 columns wide, "helloworld" fills the first row and "!" wraps to the second row, or alternatively use a word that straddles the boundary: set row 0 to `"aaaBBBBBBB"` (soft-wrap flag set) and row 1 to `"BBBccc"` so the word `"BBBBBBBBBBB"` spans both rows.
3. Call the search function with query `"BBBBBBBBBBB"` (or the equivalent spanning word).
4. Collect results.

**Expected result:** At least one match is found at the position spanning row 0 / row 1. The match covers cells from the end of row 0 through the start of row 1.

**Pass criteria:** Non-empty result set. The match position correctly identifies the cross-row span.

**Fail criteria:** Zero results returned; the soft-wrap boundary prevents the match; or a panic occurs.

**Note:** If test `search_soft_wrap_word_spanning_two_rows_is_found` already exists in `src-tauri/src/vt/search.rs` without `#[ignore]`, this scenario is satisfied by that test. Additional coverage may be added for a word spanning three soft-wrapped rows.

---

## 4. Blocked Scenarios

| Scenario ID | Reason | Unblocked when |
|---|---|---|
| TP-MIN-001 – TP-MIN-003 | E2E requires `pnpm tauri build` + running display + tauri-driver | Production build functional, CI E2E job available |
| TP-MIN-004 – TP-MIN-005 | Same as above; also requires first-launch state reset mechanism | Same as above |

All other scenarios (TP-MIN-006 through TP-MIN-019) are runnable in the current environment (unit tests, nextest, vitest).

---

## 5. No-Regression Policy

All scenarios TP-MIN-006 through TP-MIN-019 must pass before any feature touching the relevant modules is considered complete. E2E scenarios (TP-MIN-001 through TP-MIN-005) are deferred but must be added to the E2E suite as soon as the build gate is green.

Any failure in TP-MIN-018 (contrast ratio) is a WCAG AA regression and is treated as a blocker.
