<!-- SPDX-License-Identifier: MPL-2.0 -->

# Test Protocol — TauTerm: Blocking & Major IPC Items

> **Document status:** Initial revision — 2026-04-05
> **Author:** domain-expert
> **Based on:** FS.md §3.1.7/3.1.10, §3.4–3.8, §3.9, §3.10, §3.11, ARCHITECTURE.md §4–§5
> **Scope:** 12 blocking/major-IPC items identified at bootstrap session j. Covers functional correctness from the terminal emulation, PTY encoding, and SSH protocol perspective. Security scenarios are out of scope here — see `security-pty-ipc-ssh-credentials-csp-osc52.md`.

---

## 1. Domain Constraints

The following constraints from the architecture and VT standards bear directly on the scenarios below.

### 1.1 VT Mode State and IPC Feedback Loop

`keyboard.ts` and the mouse event encoder must track terminal mode state received from the backend via `mode-state-changed` events (ARCHITECTURE.md §4.3). The event carries `{ paneId, decckm, deckpam }`. Without this event:

- Arrow keys encode as `ESC [A` when they should encode as `ESC OA` (DECCKM active — vim, readline)
- Numpad keys encode as digits when they should encode as application sequences (DECKPAM active)
- Mouse events are dispatched to TauTerm UI when they should be forwarded to the PTY

Any scenario that depends on mode-sensitive encoding implicitly requires the `mode-state-changed` event to be wired frontend ↔ backend. Tests marked **[IPC-DEPENDENCY: mode-state-changed]** fail if this event is not listened to.

### 1.2 Mouse Encoding: X10 vs SGR

X10 mouse encoding (default) represents column/row as `byte = 32 + n`. This overflows for terminals wider than 223 columns (32 + 223 = 255, the maximum unsigned byte value before wrapping). SGR encoding (DECSET 1006) uses decimal integers and is required for any coordinate above 223. Tests that verify SGR encoding must use a terminal geometry ≥ 224 columns wide.

Scroll wheel events: in X10/Normal/ButtonEvent/AnyEvent modes, scroll up is button 64 (byte = `32 + 32 + 0` = `96` for SGR, button index `64` in raw form), scroll down is button 65. The exact byte is `32 + 64 + 0 = 96` for scroll-up in X10 byte encoding, or `<64;col;rowM` / `<65;col;rowM` in SGR. Validate against xterm reference.

### 1.3 Bracketed Paste Injection Vector

RFC-level issue: the bracketed paste end sequence `ESC [201~` must be stripped from pasted content before wrapping. Failure to strip allows a crafted clipboard value to inject text after a premature end marker while still within the paste region, which the shell processes as if typed.

### 1.4 SSH PTY Request Terminal Modes

Per RFC 4254 §6.2 and Annex A, terminal mode opcodes are the RFC numbering, NOT the `termios` struct indices from Linux headers. The FS-SSH-013 erratum applies: VEOF = opcode 4 (value 4 = ^D), VKILL = opcode 5 (value 21 = ^U). The implementation in `ssh_cmds.rs` is correct per RFC; `docs/FS.md` has a documentation error in the opcode assignment that must not be corrected by fixing the implementation.

### 1.5 DECKPAM / DECKPNM in the Browser

The Web Platform exposes keypad keys via `KeyboardEvent.code` (e.g., `Numpad5`, `NumpadEnter`, `NumpadAdd`). In application mode, these must map to `ESC O u`, `ESC O M`, `ESC O k` etc. (the SS3-prefixed application keypad sequences). In numeric mode, numpad keys must emit the corresponding digit or operator character. The frontend must distinguish `code: "Numpad5"` from `code: "Digit5"`. NumLock state on the OS affects `key` value but not `code`; use `code` as the canonical identifier.

### 1.6 OSC Title Propagation Path

OSC 0/1/2 sequences are parsed by `VtProcessor` → `OscDispatch`. The parsed title must propagate to the frontend. The architecture does not define a dedicated `title-changed` event; the title is carried in the `session-state-changed` event with `changeType: 'pane-metadata-changed'` (ARCHITECTURE.md §4.3, §4.5.2). The `TabState` returned contains the updated `PaneState.title`. The frontend must listen for this event and update the tab header accordingly.

### 1.7 Focus Events and Pane Focus

`mode-state-changed` carries `deckpam` and `decckm`. Focus events mode 1004 is a separate flag not currently listed in the architecture's `mode-state-changed` payload. Either the payload must be extended to include `focusEvents: bool`, or the frontend must track mode 1004 independently by intercepting the raw `DECSET 1004` / `DECRST 1004` bytes that arrive in `screen-update` events. The preferred approach is to extend `mode-state-changed`.

---

## 2. Blocked Tests

Tests in this protocol that depend on stubs not yet replaced as of 2026-04-05:

| Test ID | Blocked by | Unblocked when |
|---|---|---|
| TEST-SEARCH-* | `search_pane` always returns `[]` | `vt/search.rs` implemented and registered |
| TEST-SSH-AUTH-* | Frontend does not listen to `ssh-state-changed`, `credential-prompt`, `host-key-prompt` | Frontend IPC listeners wired |
| TEST-SSH-RECON-* | No reconnect button in disconnected pane | Reconnect UI + `reconnect_ssh` command wired |
| TEST-MOUSE-* | Mouse events not encoded and written to PTY | `terminal/mouse.ts` encoder + `send_input` path wired |
| TEST-BPASTE-* | `DECSET 2004` not tracked in frontend | `keyboard.ts` / `mouse.ts` bracketed paste state machine |
| TEST-NOTIF-* | `notification-changed` not listened in frontend | Notification IPC listener wired |
| TEST-PANE-FOCUS-* | Click does not call `set_active_pane` | Mouse click handler wired to `invoke('set_active_pane')` |
| TEST-CRED-* | `CredentialManager` not wired in `ssh_cmds.rs` | `platform::CredentialStore` integration |
| TEST-OSC-TITLE-* | OSC 0/1/2 title not propagated to frontend | `VtProcessor::osc_dispatch` → `session-state-changed` pipeline |
| TEST-FOCUS-* | Frontend does not generate `ESC [I` / `ESC [O` | Focus event emission on pane focus/blur |
| TEST-DECKPAM-* | DECKPAM mode ignored in `keyboard.ts` | `mode-state-changed` listener + encoding table |
| TEST-PASTE-SHORTCUT-* | Ctrl+Shift+V not intercepted in `handleGlobalKeydown` | Shortcut handler registration |

---

## 3. Test Scenarios

### 3.1 Scrollback Search (FS-SEARCH-001 to 007)

---

#### TEST-SEARCH-001
**FS requirements:** FS-SEARCH-001, FS-SEARCH-003
**Layer:** Integration (Rust — `vt/search`)
**Priority:** Must

**Preconditions:** A `ScreenBuffer` has been fed 50 lines of output. Lines 10, 25, and 40 contain the strings `error`, `Error`, and `ERROR` respectively. No alternate screen is active.

**Steps:**
1. Call `search_pane` with `query: { text: "error", case_sensitive: false, regex: false }`.
2. Inspect the returned `Vec<SearchMatch>`.

**Expected result:** Three `SearchMatch` entries are returned, one per line (10, 25, 40). Each match carries the correct line index, character range, and the matched text. The matches are in ascending line order.

---

#### TEST-SEARCH-002
**FS requirements:** FS-SEARCH-001, FS-SEARCH-003
**Layer:** Integration (Rust)
**Priority:** Must

**Preconditions:** Same buffer as TEST-SEARCH-001.

**Steps:**
1. Call `search_pane` with `query: { text: "error", case_sensitive: true, regex: false }`.
2. Inspect the returned `Vec<SearchMatch>`.

**Expected result:** Exactly one `SearchMatch` is returned — only the lowercase `error` on line 10. `Error` and `ERROR` are not matched.

---

#### TEST-SEARCH-003
**FS requirements:** FS-SEARCH-002
**Layer:** Integration (Rust)
**Priority:** Must

**Preconditions:** A `ScreenBuffer` with terminal width 20. A 25-character word `superlongidentifiername` is present: characters 1–12 on line 5, characters 13–25 soft-wrapped to line 6 (soft-wrap, not a hard newline).

**Steps:**
1. Call `search_pane` with `query: { text: "superlongidentifiername", case_sensitive: false }`.

**Expected result:** One match is returned. Its character range spans both line 5 and line 6. The `SearchMatch` represents the full word across the soft-wrap boundary, not two partial matches.

**Domain note:** The search implementation must join soft-wrapped lines into logical lines before matching. A hard newline terminates a logical line; a soft wrap does not. This requires the `ScreenBuffer` to track the `soft_wrap` flag per line.

---

#### TEST-SEARCH-004
**FS requirements:** FS-SEARCH-004
**Layer:** Integration (Rust)
**Priority:** Must

**Preconditions:** A `VtProcessor` that has processed `ESC [?1049h` (alternate screen activation). Alternate screen contains the string `pattern`. Normal screen scrollback does not contain `pattern`.

**Steps:**
1. Call `search_pane` while alternate screen is active.

**Expected result:** `Vec<SearchMatch>` is empty. The alternate screen buffer is not searched. Searching does not affect the alternate screen state.

---

#### TEST-SEARCH-005
**FS requirements:** FS-SEARCH-005
**Layer:** Integration (Rust)
**Priority:** Must

**Preconditions:** A `ScreenBuffer` with 100,000 lines, each containing a unique numeric string (e.g., `seq 1 100000` output). The search term `99999` appears exactly once.

**Steps:**
1. Start a timer.
2. Call `search_pane` with `query: { text: "99999", case_sensitive: false }`.
3. Stop the timer.

**Expected result:** Exactly one `SearchMatch` is returned. Elapsed time is under 100 ms on a mid-range x86_64 system.

**Domain note:** 100,000 lines × ~80 chars = ~8 MB of text to scan. A naive line-by-line regex compile-per-call implementation will fail this performance test. The implementation must compile the query once and apply it across all lines in a single pass.

---

#### TEST-SEARCH-006
**FS requirements:** FS-SEARCH-006
**Layer:** E2E
**Priority:** Must

**Preconditions:** TauTerm is open. A pane has scrollback with 5 occurrences of the word `needle` spread across 500 lines of output. Search overlay is open.

**Steps:**
1. Type `needle` in the search field.
2. Observe highlight state.
3. Press Next (4 times total). Press Previous (once).

**Expected result:**
- All 5 occurrences are highlighted with a non-current-match style.
- The first match is highlighted with the current-match style.
- After 4 Next presses: match 5/5 is current, centered in the viewport.
- After 1 Previous: match 4/5 is current, centered in the viewport.
- Match counter shows the ordinal correctly (e.g., `4 / 5`).

---

#### TEST-SEARCH-007
**FS requirements:** FS-SEARCH-007
**Layer:** E2E
**Priority:** Must

**Preconditions:** TauTerm is open with a pane containing scrollback.

**Steps:**
1. Press Ctrl+Shift+F.
2. Observe whether the search overlay opens.
3. Press Escape.
4. Observe whether the search overlay closes and the terminal is interactive.

**Expected result:** Ctrl+Shift+F opens the search overlay. Escape closes it without leaving residual highlights. Focus returns to the terminal pane (keyboard input goes to the PTY).

---

### 3.2 SSH Auth Interactive (FS-SSH-010 to 014)

---

#### TEST-SSH-AUTH-001
**FS requirements:** FS-SSH-010
**Layer:** E2E
**Priority:** Must

**Preconditions:** A saved connection to `127.0.0.1:2222`. The server is reachable. Frontend listens to `ssh-state-changed`.

**Steps:**
1. Invoke `open_ssh_connection` for the saved connection.
2. Observe the pane state during each SSH lifecycle phase.

**Expected result:** Each of the following states is visually represented in the pane in order:
- `Connecting` — a spinner or label indicates TCP connection in progress.
- `Authenticating` — a different label or spinner phase indicates credential exchange.
- `Connected` — the pane transitions to an interactive terminal; a persistent SSH badge is visible.

No state is silently skipped. The pane does not go directly from blank to interactive.

**Domain note:** The `ssh-state-changed` event carries `SshLifecycleState`. The frontend must map all five states (Connecting, Authenticating, Connected, Disconnected, Closed) to distinct visual representations as specified in FS-SSH-010.

---

#### TEST-SSH-AUTH-002
**FS requirements:** FS-SSH-011
**Layer:** E2E
**Priority:** Must

**Preconditions:** `~/.config/tauterm/known_hosts` contains no entry for `127.0.0.1:2222`.

**Steps:**
1. Connect to `127.0.0.1:2222`.
2. Observe whether a `host-key-prompt` event arrives and the frontend displays the prompt.

**Expected result:**
- A prompt dialog or pane overlay is shown containing: (a) a plain-language explanation, (b) the SHA-256 fingerprint, (c) the key type (e.g., `ED25519`), (d) Accept and Reject actions.
- Reject is visually the safe/default action (e.g., the focused or primary button).
- Clicking Reject invokes `reject_host_key`. The connection does not proceed. No entry is written to known-hosts.
- Clicking Accept invokes `accept_host_key`. The connection proceeds to Authenticating. An entry is written to `~/.config/tauterm/known_hosts`.

**Domain note:** Verify the displayed fingerprint matches `ssh-keyscan -t ed25519 127.0.0.1 | ssh-keygen -lf -` output. A mismatch would indicate the `host-key-prompt` event carries the wrong fingerprint.

---

#### TEST-SSH-AUTH-003
**FS requirements:** FS-SSH-011
**Layer:** E2E
**Priority:** Must

**Preconditions:** `~/.config/tauterm/known_hosts` contains a stored key for `127.0.0.1:2222`. The server is now presenting a different key (key has been rotated).

**Steps:**
1. Attempt to connect.
2. Observe the UI response.

**Expected result:**
- Connection is blocked before reaching the Authenticating state.
- A prominent warning dialog is shown with both fingerprints (stored and new) side by side.
- The warning text explicitly names the risk (key change = potential MITM).
- Default action is Reject (the Accept path requires a non-default deliberate step).
- After Reject: state returns to Closed, no `accept_host_key` is invoked.

---

#### TEST-SSH-AUTH-004
**FS requirements:** FS-SSH-012
**Layer:** E2E
**Priority:** Must

**Preconditions:** A saved connection with no identity file. The server requires password authentication. The frontend listens to `credential-prompt`.

**Steps:**
1. Initiate connection.
2. Wait for `Authenticating` state.
3. Observe whether a password prompt appears without manual intervention.
4. Submit the correct password.

**Expected result:**
- A credential prompt dialog appears automatically when the `credential-prompt` event is received.
- The password input field is masked.
- Submitting the correct password invokes `provide_credentials` and the session transitions to Connected.
- Submitting an incorrect password results in an error indicator and either a re-prompt or a Disconnected state with a meaningful error message.

---

#### TEST-SSH-AUTH-005
**FS requirements:** FS-SSH-013
**Layer:** Unit (Rust — `ssh/`)
**Priority:** Must

**Preconditions:** A `build_pty_request_modes()` function (or equivalent) that assembles the `encoded terminal modes` field for the SSH PTY request.

**Steps:**
1. Call the function and parse the returned byte sequence.
2. Verify each opcode/value pair.

**Expected result:** The byte sequence contains, in any order, the following pairs before the `TTY_OP_END` (0) terminator:

| Opcode | Value | Meaning |
|--------|-------|---------|
| 1 (VINTR) | 3 | ^C |
| 2 (VQUIT) | 28 | ^\ |
| 3 (VERASE) | 127 | DEL |
| 4 (VEOF) | 4 | ^D |
| 5 (VKILL) | 21 | ^U |
| 10 (VSUSP) | 26 | ^Z |
| 50 (ISIG) | 1 | enabled |
| 51 (ICANON) | 1 | enabled |
| 53 (ECHO) | 1 | enabled |

The opcodes are RFC 4254 Annex A numbers, not Linux `termios` struct field indices. In particular: VEOF = 4, VKILL = 5. If the implementation swaps these (VKILL=4, VEOF=5), that matches the FS-SSH-013 documentation error but is wrong per RFC — the test should catch it.

**Domain note:** Each opcode/value pair is encoded as 1 byte opcode + 4 bytes value (big-endian uint32) per RFC 4254 §6.2.

---

#### TEST-SSH-AUTH-006
**FS requirements:** FS-SSH-014
**Layer:** E2E
**Priority:** Must

**Preconditions:** An SSH server configured to offer only `ssh-rsa` (SHA-1) is available. First-time connection (or known-hosts pre-populated for this host).

**Steps:**
1. Connect and accept the host key prompt.
2. Complete authentication.
3. Observe the pane after reaching Connected state.

**Expected result:**
- A dismissible warning banner or overlay appears within the pane.
- The warning names `ssh-rsa` (SHA-1) explicitly.
- The terminal is fully interactive — input reaches the PTY.
- Dismissing the warning (via the dismiss action) removes it.
- The warning does not reappear on subsequent keystrokes.
- Invoking `dismiss_ssh_algorithm_warning` clears the backend flag.

---

### 3.3 SSH Reconnection UI (FS-SSH-040 to 042)

---

#### TEST-SSH-RECON-001
**FS requirements:** FS-SSH-040, FS-SSH-041
**Layer:** E2E
**Priority:** Must

**Preconditions:** An established SSH session in Connected state. Network is disrupted (e.g., dropping TCP packets to the SSH port) until three keepalive intervals expire, triggering the Disconnected state.

**Steps:**
1. Wait for `ssh-state-changed` with state `Disconnected`.
2. Observe the pane content.

**Expected result:**
- The pane displays a Disconnected indicator (text, icon, or overlay).
- A "Reconnect" button (or equivalent primary action) is visible directly within the pane without requiring any menu navigation.
- The Reconnect action is keyboard-accessible (focusable via Tab, activatable via Enter/Space).
- The tab header also reflects the Disconnected state (visually distinct from Connected and Closed).

**Domain note:** `SshLifecycleState::Disconnected` carries a `reason` field (network drop, keepalive timeout, write failure). The pane overlay must display this reason per FS-SSH-022.

---

#### TEST-SSH-RECON-002
**FS requirements:** FS-SSH-040
**Layer:** E2E
**Priority:** Must

**Preconditions:** Same as TEST-SSH-RECON-001. Network has been restored.

**Steps:**
1. Click the "Reconnect" button (or activate via keyboard).
2. Observe the reconnection lifecycle.

**Expected result:**
- The pane cycles through: Disconnected → Connecting → Authenticating → Connected.
- The same host/port/username from the original connection are reused.
- No "new connection" configuration dialog is shown.
- If credentials are required and no keychain entry exists, a password prompt is acceptable.
- On success, the pane returns to interactive terminal state.

---

#### TEST-SSH-RECON-003
**FS requirements:** FS-SSH-042
**Layer:** E2E
**Priority:** Must

**Preconditions:** A session that has produced 200 lines of scrollback. Session disconnects and reconnects.

**Steps:**
1. After reconnection reaches Connected state, scroll upward in the scrollback.

**Expected result:**
- All 200 pre-disconnection lines are intact and scrollable.
- A visual separator (e.g., a horizontal rule with text "--- Reconnected ---" or a timestamp label) appears at the reconnection boundary.
- New output from the reconnected session appears below the separator.
- Scrollback navigation (keyboard, mouse wheel, scrollbar) works across the separator.

---

#### TEST-SSH-RECON-004
**FS requirements:** FS-SSH-010 (Closed state contract)
**Layer:** E2E
**Priority:** Must

**Preconditions:** An SSH session whose remote shell exits with code 0 (normal exit — user typed `exit`).

**Steps:**
1. Observe the pane state after the shell exits.

**Expected result:**
- The pane enters the Closed state.
- No "Reconnect" button is shown.
- The available actions are equivalent to a terminated local PTY pane (e.g., "Close pane").
- The Closed state indicator is visually distinct from the Disconnected state indicator.

**Domain note:** Per the state machine (ARCHITECTURE.md §5.2), Closed is reached from Connected via `close_ssh_connection()` or clean remote exit (code 0). Disconnected is reached via network failure or non-zero exit. The Reconnect UI must only appear in the Disconnected state.

---

### 3.4 Mouse Reporting (FS-VT-080 to 086)

---

#### TEST-MOUSE-001
**FS requirements:** FS-VT-080, FS-VT-082
**Layer:** Unit (Frontend — `terminal/mouse.ts`)
**Priority:** Must

**Preconditions:** Mouse reporting mode is `X10` (DECSET 9). A click event occurs at cell (col=10, row=5).

**Steps:**
1. Call the mouse encoder with: `{ mode: X10, button: Left, col: 10, row: 5, type: Press }`.
2. Inspect the returned byte sequence.

**Expected result:** `[0x1B, 0x5B, 0x4D, 0x20, 0x2A, 0x25]` — i.e., `ESC [ M` followed by button byte 32 (left press), col byte 42 (32+10), row byte 37 (32+5).

**Domain note:** X10 mode only reports button press, not release. A release event in X10 mode produces no bytes.

---

#### TEST-MOUSE-002
**FS requirements:** FS-VT-080, FS-VT-082
**Layer:** Unit (Frontend)
**Priority:** Must

**Preconditions:** Mouse reporting mode is `Normal` (DECSET 1000).

**Steps:**
1. Encode a left-button press at (col=5, row=3): expect `ESC [ M` + byte(32+0) + byte(32+5) + byte(32+3).
2. Encode a left-button release at (col=5, row=3): expect `ESC [ M` + byte(32+3) + byte(32+5) + byte(32+3).

**Expected result:** Press produces button byte `0x20` (32). Release produces button byte `0x23` (35 = 32+3, the release sentinel in X10/Normal encoding).

---

#### TEST-MOUSE-003
**FS requirements:** FS-VT-081
**Layer:** Unit (Frontend)
**Priority:** Must

**Preconditions:** Mouse reporting in SGR mode (DECSET 1006 active, Normal mode 1000 also active). Terminal is 240 columns wide. A click at (col=220, row=3).

**Steps:**
1. Encode a left-button press at (col=220, row=3) in SGR mode.
2. Encode the corresponding release.

**Expected result:**
- Press: `ESC [ < 0 ; 220 ; 3 M` (the literal string `\033[<0;220;3M`).
- Release: `ESC [ < 0 ; 220 ; 3 m` (lowercase `m` for release in SGR).

**Domain note:** Column 220 cannot be represented in X10 single-byte encoding (max 223 before overflow). SGR mode is mandatory for terminals wider than 223 columns. This test verifies the encoder selects SGR when enabled, regardless of the coordinate value.

---

#### TEST-MOUSE-004
**FS requirements:** FS-VT-080 (Button-event mode 1002)
**Layer:** Integration (E2E)
**Priority:** Must

**Preconditions:** TauTerm pane running `cat -v`. Enable Button-event mode: `printf "\033[?1000h\033[?1002h"`.

**Steps:**
1. Press and hold the left mouse button. Drag to a new position.
2. Release the button.

**Expected result:** Motion events are reported while the button is held (mode 1002 = ButtonEvent = report motion only while a button is pressed). No motion events after release. Each motion event carries the button byte with the motion flag set (bit 5 = `+32`).

---

#### TEST-MOUSE-005
**FS requirements:** FS-VT-080 (Any-event mode 1003)
**Layer:** Integration (E2E)
**Priority:** Must

**Preconditions:** TauTerm pane running `cat -v`. Enable any-event mode: `printf "\033[?1000h\033[?1003h"`.

**Steps:**
1. Move the mouse over the terminal surface without pressing any button.
2. Count the motion event sequences produced.

**Expected result:** Motion events are produced for each mouse movement regardless of button state. Button byte for no-button motion is `32 + 32 = 64` in X10 encoding. After `printf "\033[?1003l\033[?1000l"`, no further motion events are generated.

---

#### TEST-MOUSE-006
**FS requirements:** FS-VT-083
**Layer:** Integration (E2E)
**Priority:** Must

**Preconditions:** vim is open with `set mouse=a` (mouse reporting active). A pane shows visible text that can be selected.

**Steps:**
1. Hold Shift and click within the terminal.

**Expected result:** TauTerm performs a text selection (a visual selection highlight appears). The click is NOT forwarded to vim as a mouse event. Vim's cursor position does not change. The shift-click event does not produce PTY bytes.

**Domain note:** FS-VT-083 mandates Shift+Click bypass regardless of active mouse reporting mode. The frontend must check the `shiftKey` modifier before deciding whether to encode the event as a PTY mouse report or handle it locally as a selection event.

---

#### TEST-MOUSE-007
**FS requirements:** FS-VT-085
**Layer:** Integration (E2E)
**Priority:** Must

**Preconditions:** Mouse reporting mode 1000 or 1003 active. Pane running `cat -v`.

**Steps:**
1. Scroll the mouse wheel upward over the terminal surface.
2. Hold Shift and scroll the mouse wheel upward.

**Expected result:**
- Step 1: PTY receives button-4 scroll events (one per wheel tick). In SGR mode: `ESC [ < 64 ; col ; row M`. In X10 mode: `ESC [ M` + byte(96) + col-byte + row-byte. No TauTerm scrollback movement.
- Step 2: TauTerm scrollback scrolls upward. No PTY bytes are generated.

---

#### TEST-MOUSE-008
**FS requirements:** FS-VT-086
**Layer:** Integration (E2E)
**Priority:** Must

**Preconditions:** A program sets mode 1003 (any-event) and exits without disabling it. This simulates a crash. A new `cat -v` session is running in the same pane.

**Steps:**
1. Click and move the mouse within the terminal pane.

**Expected result:** No mouse event bytes reach `cat -v`. TauTerm handles mouse events locally (selection, scrollback scroll). All mouse reporting modes are reset on PTY/session close per ARCHITECTURE.md §5.3.

---

### 3.5 Bracketed Paste (FS-CLIP-008, FS-CLIP-009)

---

#### TEST-BPASTE-001
**FS requirements:** FS-CLIP-008
**Layer:** Unit (Frontend)
**Priority:** Must

**Preconditions:** The frontend paste handler has received a `mode-state-changed` event with `bracketedPaste: true`. Clipboard content is `line1\nline2\nline3`.

**Steps:**
1. Invoke the paste action (Ctrl+Shift+V equivalent).
2. Capture the byte sequence passed to `send_input`.

**Expected result:** `send_input` receives: `ESC [200~` + `line1\nline2\nline3` + `ESC [201~` as a single payload (or as consecutive calls that produce the same byte sequence in the PTY). No other prefix or suffix.

---

#### TEST-BPASTE-002
**FS requirements:** FS-CLIP-008 (injection prevention)
**Layer:** Unit (Frontend)
**Priority:** Must

**Preconditions:** Bracketed paste mode active. The clipboard content contains an embedded `ESC [201~` sequence in the middle: `hello\033[201~world`.

**Steps:**
1. Invoke the paste action.
2. Inspect the `send_input` payload.

**Expected result:** The embedded `ESC [201~` is stripped. The payload is: `ESC [200~` + `helloworld` + `ESC [201~`. The content after the would-be premature end-marker (`world`) is preserved.

**Domain note:** If stripping is not performed, the shell receives `ESC [200~ hello ESC [201~` followed by `world ESC [201~`. The shell processes `world` as if typed by the user — a clipboard injection attack.

---

#### TEST-BPASTE-003
**FS requirements:** FS-CLIP-008
**Layer:** Unit (Frontend)
**Priority:** Must

**Preconditions:** Bracketed paste mode NOT active. Clipboard content is `single line without newline`.

**Steps:**
1. Invoke paste.
2. Inspect the `send_input` payload.

**Expected result:** The text is sent directly without bracketed paste markers. No confirmation dialog is shown. `send_input` receives the raw text bytes.

---

#### TEST-BPASTE-004
**FS requirements:** FS-CLIP-009
**Layer:** E2E
**Priority:** Must (confirmation) / Should (dialog configurability)

**Preconditions:** Bracketed paste mode NOT active. Clipboard content is `command1\ncommand2`.

**Steps:**
1. Invoke paste.
2. Observe whether a confirmation dialog appears before any bytes reach the PTY.
3. Click Cancel.
4. Verify no bytes were sent.

**Expected result:** A confirmation dialog is displayed before any text is sent to the PTY. The dialog shows the paste content or a preview. Cancel sends nothing. Confirm sends the raw text (no bracketed markers).

---

#### TEST-BPASTE-005
**FS requirements:** FS-CLIP-008
**Layer:** Unit (Frontend)
**Priority:** Must

**Preconditions:** Bracketed paste mode active. Clipboard content contains actual ESC bytes followed by `[1m` (i.e., the literal SGR bold-on sequence).

**Steps:**
1. Invoke paste.
2. Inspect the `send_input` payload.
3. Observe the terminal display.

**Expected result:** The ESC bytes in the pasted content are delivered verbatim to the PTY inside the bracketed paste markers. The terminal display is NOT affected (no bold mode change, no cursor movement). The application (shell) receives the literal ESC bytes as data, not as escape sequences.

**Domain note:** This is guaranteed by the PTY application's bracketed paste handling — the shell ignores escape sequences within bracketed paste regions. TauTerm's role is to deliver the bytes faithfully without pre-processing them as VT sequences.

---

### 3.6 Ctrl+Shift+V Paste Shortcut (FS-CLIP-005, FS-KBD-003)

---

#### TEST-PASTE-SHORTCUT-001
**FS requirements:** FS-CLIP-005, FS-KBD-001
**Layer:** Unit (Frontend)
**Priority:** Must

**Preconditions:** Ctrl+Shift+V is registered as the Paste shortcut in `handleGlobalKeydown`. The clipboard (CLIPBOARD selection) contains `clipboard_content`. The PRIMARY selection contains `primary_content` (different string).

**Steps:**
1. Simulate a `keydown` event with `ctrlKey: true, shiftKey: true, code: 'KeyV'`.
2. Capture which clipboard source is read and what bytes reach `send_input`.

**Expected result:**
- The CLIPBOARD selection (not PRIMARY) is read.
- `clipboard_content` is passed to the paste handler.
- The key event does not produce a `^V` byte (0x16) in the PTY.

---

#### TEST-PASTE-SHORTCUT-002
**FS requirements:** FS-KBD-001 (consumed shortcut does not reach PTY)
**Layer:** Unit (Frontend)
**Priority:** Must

**Preconditions:** Ctrl+Shift+V is registered as Paste. A mock PTY input recorder is attached to `send_input`.

**Steps:**
1. Simulate the Ctrl+Shift+V keydown event.
2. Check whether any bytes were passed to `send_input` before the paste content.

**Expected result:** No key-encoding bytes (no `^V` = 0x16, no `^[V` for Shift variants) are sent to the PTY. The shortcut is fully consumed. Only the paste payload (if non-empty clipboard) reaches `send_input`.

---

#### TEST-PASTE-SHORTCUT-003
**FS requirements:** FS-KBD-002 (unbound shortcut passes to PTY)
**Layer:** Unit (Frontend)
**Priority:** Must

**Preconditions:** The Paste shortcut has been removed from the shortcut map (empty binding).

**Steps:**
1. Simulate a `keydown` event with `ctrlKey: true, shiftKey: true, code: 'KeyV'`.
2. Capture what `send_input` receives.

**Expected result:** The key combination is encoded as a PTY sequence. `send_input` receives `0x16` (Ctrl+V = ^V) or the appropriate Ctrl+Shift+V CSI sequence per FS-KBD-009. No paste action occurs.

---

### 3.7 Activity Notification Events (FS-NOTIF-001 to 004)

---

#### TEST-NOTIF-001
**FS requirements:** FS-NOTIF-001, FS-TAB-007
**Layer:** Integration (Frontend)
**Priority:** Must

**Preconditions:** Frontend listens to the `notification-changed` event. Two tabs are open; Tab 2 is active.

**Steps:**
1. Simulate a `notification-changed` event with `{ tabId: tab1_id, paneId: pane1_id, notification: { type: "activity" } }`.
2. Observe Tab 1's header.

**Expected result:** Tab 1's header displays a visual activity indicator (dot, underline, colour change, or icon). Tab 2's header is unaffected. The indicator is not shown on the tab bar's currently active tab.

---

#### TEST-NOTIF-002
**FS requirements:** FS-NOTIF-002
**Layer:** Integration (Frontend)
**Priority:** Must

**Preconditions:** Same setup as TEST-NOTIF-001.

**Steps:**
1. Simulate two events on Tab 1: first `{ notification: { type: "activity" } }`, then `{ notification: { type: "process-terminated" } }`.
2. Observe the final indicator on Tab 1's header.

**Expected result:** The final indicator reflects `process-terminated`, which must be visually distinct from the `activity` indicator (different colour, icon, or symbol). The two indicator types must not be visually identical.

---

#### TEST-NOTIF-003
**FS requirements:** FS-NOTIF-003
**Layer:** Integration (Frontend)
**Priority:** Must

**Preconditions:** Tab 1 has an active `activity` notification indicator.

**Steps:**
1. Switch to Tab 1 (simulate active-tab-changed).
2. Observe whether the indicator is cleared.

**Expected result:** The activity indicator is removed from Tab 1's header immediately on tab activation. The `notification-changed` event with `notification: null` is expected from the backend when the tab becomes active; if the frontend clears it optimistically, that is also acceptable.

---

#### TEST-NOTIF-004
**FS requirements:** FS-NOTIF-004, FS-VT-093
**Layer:** Integration (Frontend + E2E)
**Priority:** Must

**Preconditions:** Two tabs open; Tab 2 is active. Tab 1 pane receives a BEL character (`0x07`).

**Steps:**
1. In Tab 1 (background), run `printf "\007"`.
2. Observe Tab 1's header.

**Expected result:** A bell indicator appears on Tab 1's header. If the preference is set to visual bell, no audible sound plays. The bell indicator is distinguishable from the output-activity indicator (per FS-NOTIF-002).

---

### 3.8 Pane Focus via Mouse Click → set_active_pane (FS-PANE-005)

---

#### TEST-PANE-FOCUS-001
**FS requirements:** FS-PANE-005, FS-PANE-006
**Layer:** Integration (Frontend)
**Priority:** Must

**Preconditions:** A tab with two panes (split horizontal). Pane A is active. Frontend is wired to call `invoke('set_active_pane', { pane_id })` on pane click.

**Steps:**
1. Simulate a click event on Pane B's DOM element.
2. Capture the IPC call made.

**Expected result:**
- `invoke('set_active_pane', { pane_id: pane_b_id })` is called.
- Pane B receives the active visual style (e.g., brighter border). Pane A loses it.
- Exactly one pane is active at any time.

---

#### TEST-PANE-FOCUS-002
**FS requirements:** FS-PANE-005
**Layer:** E2E
**Priority:** Must

**Preconditions:** Two panes. Pane A is active and running `cat -v`. Pane B is running a separate shell.

**Steps:**
1. Click on Pane B.
2. Type `hello` on the keyboard.

**Expected result:** "hello" appears in Pane B's `cat` output (or shell output). Pane A receives no input. Verify by observing which pane shows the typed characters.

---

#### TEST-PANE-FOCUS-003
**FS requirements:** FS-PANE-005, FS-KBD-003
**Layer:** E2E
**Priority:** Must

**Preconditions:** Two panes. Pane A is active. Both panes are running `cat -v`.

**Steps:**
1. Press Ctrl+Shift+Right (keyboard pane navigation shortcut).
2. Type `keyboard_input`.

**Expected result:** Pane B becomes active (visual style updates). `invoke('set_active_pane')` was called. `keyboard_input` appears in Pane B's output, not Pane A's.

---

### 3.9 SSH Credential Store Wiring (FS-CRED-001, FS-CRED-005)

---

#### TEST-CRED-001
**FS requirements:** FS-CRED-001
**Layer:** Integration (Rust — `platform/credentials`)
**Priority:** Must

**Preconditions:** The Secret Service D-Bus API is available (keychain daemon running). A password `test_password_abc` is to be stored for connection `conn_id_001`.

**Steps:**
1. Call `CredentialStore::store(conn_id, "test_password_abc")`.
2. Inspect `~/.config/tauterm/preferences.json` for the string `test_password_abc`.
3. Retrieve the credential via `CredentialStore::retrieve(conn_id)`.

**Expected result:**
- `~/.config/tauterm/preferences.json` does not contain `test_password_abc` or any base64/hex encoding of it.
- `CredentialStore::retrieve` returns `test_password_abc` correctly.
- `secret-tool lookup service tauterm connection conn_id_001` returns the stored value.

---

#### TEST-CRED-002
**FS requirements:** FS-CRED-005
**Layer:** Integration (Rust)
**Priority:** Must

**Preconditions:** The Secret Service D-Bus service is unavailable (daemon stopped). `CredentialStore::is_available()` returns `false`.

**Steps:**
1. Attempt to store a credential via `CredentialStore::store()`.
2. Observe the returned error.
3. Attempt to connect to an SSH server that requires password authentication.

**Expected result:**
- `CredentialStore::store()` returns an error (no silent fallback to plaintext file).
- During connection, a `credential-prompt` event is emitted for the user to provide the password interactively.
- The event or the pane overlay includes a notice that credential persistence is unavailable.
- No password is written to any file on disk.

---

#### TEST-CRED-003
**FS requirements:** FS-CRED-001, FS-CRED-005 (integration with ssh_cmds.rs)
**Layer:** Integration (Rust)
**Priority:** Must

**Preconditions:** A saved connection has a stored password in the keychain. `CredentialManager` is wired in `ssh_cmds.rs`.

**Steps:**
1. Initiate `open_ssh_connection` for this saved connection.
2. Observe whether a `credential-prompt` event is emitted.

**Expected result:** No `credential-prompt` event is emitted when the keychain lookup succeeds. The stored password is retrieved transparently and used for authentication. The user is not prompted.

**Domain note:** If the keychain lookup fails (e.g., entry deleted externally), a `credential-prompt` event MUST be emitted to allow the user to provide the password. This fallback must be tested separately.

---

#### TEST-CRED-004
**FS requirements:** FS-CRED-006
**Layer:** Unit (Rust — `platform/`)
**Priority:** Must

**Preconditions:** A `validate_identity_file_path()` function (or equivalent path validation at connection time).

**Steps:**
1. Pass `../../etc/shadow` — expect rejection.
2. Pass `/home/user/.ssh/id_ed25519` (absolute, no traversal) — expect acceptance (if file exists).
3. Pass `../relative/path` — expect rejection.
4. Pass `/home/user/../user/.ssh/id_ed25519` (absolute but contains `..` component) — expect rejection or normalization to canonical path then validation.

**Expected result:** Paths 1, 3, and 4 are rejected with an appropriate error (`INVALID_IDENTITY_PATH` or similar code). Path 2 is accepted. The rejection happens before any file open or network activity.

---

### 3.10 OSC Title Update Propagation (FS-VT-060 to 062, FS-TAB-006)

---

#### TEST-OSC-TITLE-001
**FS requirements:** FS-VT-060
**Layer:** Integration (Rust — `vt/osc`)
**Priority:** Must

**Preconditions:** A `VtProcessor` has processed `ESC ] 0 ; MyCustomTitle BEL` (OSC 0 title set).

**Steps:**
1. Inspect the `VtProcessor`'s post-processing state or the emitted events.

**Expected result:** The processor has recorded the title `MyCustomTitle`. A `session-state-changed` event with `changeType: 'pane-metadata-changed'` is emitted. The `TabState` in the event contains `PaneState.title = "MyCustomTitle"`.

---

#### TEST-OSC-TITLE-002
**FS requirements:** FS-VT-060
**Layer:** Integration (Rust)
**Priority:** Must

**Preconditions:** Same as TEST-OSC-TITLE-001 but with OSC 2 (`ESC ] 2 ; AnotherTitle BEL`).

**Steps:**
1. Feed the OSC 2 sequence to `VtProcessor`.
2. Inspect the emitted event.

**Expected result:** Same pipeline as OSC 0 — title updates to `AnotherTitle`. OSC 1 (icon title) may be silently discarded; it must not cause a panic, error, or corrupt the title state.

---

#### TEST-OSC-TITLE-003
**FS requirements:** FS-VT-062
**Layer:** Unit (Rust — `vt/osc`)
**Priority:** Must

**Preconditions:** None.

**Steps:**
1. Feed `ESC ] 0 ; Title\x01\x1b[31mRed BEL` (title with C0 control and embedded SGR bytes).
2. Feed `ESC ] 0 ; ` followed by 300 `A` characters followed by ` BEL` (title exceeding 256 chars).

**Expected result:**
- Step 1: sanitized title contains neither `\x01` nor `\x1b`. The visible text `TitleRed` or `Title[31mRed` is present as plain characters — the SGR bytes are stripped, not interpreted.
- Step 2: title is capped at exactly 256 characters.

---

#### TEST-OSC-TITLE-004
**FS requirements:** FS-TAB-006, FS-VT-060
**Layer:** Integration (Frontend)
**Priority:** Must

**Preconditions:** A tab has a user-set label `UserLabel` (via `invoke('rename_tab', { label: 'UserLabel' })`). An OSC 0 sequence `ESC ] 0 ; ProcessTitle BEL` is processed.

**Steps:**
1. Simulate the `session-state-changed` event carrying `PaneState.title = "ProcessTitle"`.
2. Observe the tab header.

**Expected result:** The tab header continues to display `UserLabel`. OSC-driven title updates do not override a user-set label. Clearing the label (via `invoke('rename_tab', { label: null })`) reverts to displaying the OSC-driven title `ProcessTitle`.

---

#### TEST-OSC-TITLE-005
**FS requirements:** FS-VT-063
**Layer:** Unit (Rust — `vt/osc`)
**Priority:** Must

**Preconditions:** A `VtProcessor` has a mock PTY write channel.

**Steps:**
1. Feed `ESC [ 2 1 t` (CSI 21t — report window title read-back sequence).
2. Inspect the PTY write channel.

**Expected result:** Zero bytes are written to the PTY. The sequence is silently discarded. No title string is injected into the PTY input stream. No error is logged.

**Domain note:** This is a terminal injection prevention requirement (FS-VT-063). A response to CSI 21t would allow a program to inject the current tab title back into the shell's stdin — enabling injection of arbitrary commands if the title contains shell metacharacters.

---

### 3.11 Focus Events Mode 1004 (FS-VT-084)

---

#### TEST-FOCUS-001
**FS requirements:** FS-VT-084
**Layer:** Unit (Frontend)
**Priority:** Must

**Preconditions:** The frontend has received a `mode-state-changed` event with `focusEvents: true` for a given `paneId` (mode 1004 active). A mock `send_input` is attached.

**Steps:**
1. Simulate the pane losing focus (e.g., another pane is clicked, triggering `blur` on the pane element).
2. Simulate the pane regaining focus.

**Expected result:**
- On defocus: `send_input` is called with `{ pane_id, data: [0x1B, 0x5B, 0x4F] }` — i.e., `ESC [ O`.
- On refocus: `send_input` is called with `{ pane_id, data: [0x1B, 0x5B, 0x49] }` — i.e., `ESC [ I`.

**Domain note:** ESC [ I = focus-in, ESC [ O = focus-out. These are the xterm standard sequences for focus events mode 1004. Verify against `xterm` with `printf "\033[?1004h"` and observing raw input.

---

#### TEST-FOCUS-002
**FS requirements:** FS-VT-084
**Layer:** Unit (Frontend)
**Priority:** Must

**Preconditions:** Mode 1004 is NOT active (`focusEvents: false` in current mode state).

**Steps:**
1. Simulate pane defocus and refocus.
2. Capture all `send_input` calls.

**Expected result:** No focus event bytes are sent. `send_input` is not called for focus/blur events.

---

#### TEST-FOCUS-003
**FS requirements:** FS-VT-084, FS-VT-086
**Layer:** Integration (Rust — VT mode reset)
**Priority:** Must

**Preconditions:** `VtProcessor` has processed `ESC [ ? 1004 h` (mode 1004 enabled). The PTY session closes (process exits).

**Steps:**
1. Simulate session close: invoke the session cleanup path.
2. Inspect the mode state.

**Expected result:** Mode 1004 is reset to `false` in the post-session mode state. The frontend receives a `mode-state-changed` event (or a final `screen-update`) reflecting the reset state. Subsequent focus/blur events on the reused pane (e.g., after restart) do not generate focus bytes.

---

#### TEST-FOCUS-004
**FS requirements:** FS-VT-084
**Layer:** E2E (application compatibility)
**Priority:** Must

**Preconditions:** vim is running in a TauTerm pane. vim sets mode 1004 automatically.

**Steps:**
1. Click on another application window (defocusing TauTerm).
2. Click back on the TauTerm pane.

**Expected result:** vim behaves correctly across focus transitions — it does not display spurious characters or enter an error state. If vim uses focus events to update buffer content (e.g., `:set autoread`), external file modifications are detected on refocus.

---

### 3.12 DECKPAM / DECKPNM Keypad Mode (FS-KBD-010)

---

#### TEST-DECKPAM-001
**FS requirements:** FS-KBD-010
**Layer:** Unit (Frontend — `terminal/keyboard.ts`)
**Priority:** Must

**Preconditions:** The frontend has received `mode-state-changed` with `deckpam: true` (application mode). A mock `send_input` is attached.

**Steps:**
1. Simulate a `keydown` event for `code: "Numpad0"`, `key: "0"` (KP_0 in application mode).
2. Simulate `code: "Numpad5"`, `key: "5"` (KP_5).
3. Simulate `code: "NumpadEnter"`, `key: "Enter"` (KP_Enter).
4. Simulate `code: "NumpadAdd"`, `key: "+"` (KP_Plus).

**Expected result (application mode — SS3 sequences):**
- KP_0 → `ESC O p` (`\033Op`)
- KP_5 → `ESC O u` (`\033Ou`)
- KP_Enter → `ESC O M` (`\033OM`)
- KP_Plus → `ESC O k` (`\033Ok`)

**Domain note:** SS3-prefixed application keypad sequences are defined in the xterm terminfo `kp*` entries. Verify the full mapping table against xterm's `ctlseqs.ms` or the VT220 programmer reference manual.

---

#### TEST-DECKPAM-002
**FS requirements:** FS-KBD-010
**Layer:** Unit (Frontend)
**Priority:** Must

**Preconditions:** `mode-state-changed` with `deckpam: false` (numeric mode — default).

**Steps:**
1. Simulate `code: "Numpad5"`, `key: "5"`.
2. Simulate `code: "NumpadEnter"`, `key: "Enter"`.

**Expected result (numeric mode):**
- KP_5 → `0x35` (ASCII `5`)
- KP_Enter → `0x0D` (CR, same as regular Enter)

**Domain note:** The frontend must use `code` (e.g., `"Numpad5"`) to distinguish numpad keys from regular digit keys (`"Digit5"`), since the `key` value is the same (`"5"`) in both cases when NumLock is on.

---

#### TEST-DECKPAM-003
**FS requirements:** FS-KBD-010
**Layer:** Integration (Frontend)
**Priority:** Must

**Preconditions:** `deckpam` starts as `false`. A `mode-state-changed` event arrives with `deckpam: true`.

**Steps:**
1. Before the event: encode KP_5 → expect `0x35`.
2. Apply the mode-state update.
3. After the event: encode KP_5 → expect `ESC O u`.

**Expected result:** The encoding switches immediately on mode-state update. No restart or pane reset required. The transition is atomic — no intermediate state where some keys use old encoding and others use new encoding.

---

#### TEST-DECKPAM-004
**FS requirements:** FS-KBD-010
**Layer:** E2E (application compatibility)
**Priority:** Must

**Preconditions:** vim is running in a TauTerm pane (vim sets DECKPAM automatically on start, DECKPNM on exit).

**Steps:**
1. In vim normal mode, navigate with arrow keys.
2. In insert mode, use the numpad Enter key.
3. Exit vim. Test numpad behavior after exit.

**Expected result:**
- Arrow keys work correctly throughout (unaffected by DECKPAM — arrow key encoding uses DECCKM, not DECKPAM).
- KP_Enter in insert mode inserts a newline correctly.
- After vim exits, DECKPNM is restored. KP_5 produces the digit `5` in a subsequent shell session.

---

## 4. Coverage Summary

| Item | FS-* references | Test IDs | Layer |
|---|---|---|---|
| Scrollback search | FS-SEARCH-001–007 | TEST-SEARCH-001–007 | Unit, Integration, E2E |
| SSH auth interactive | FS-SSH-010–014 | TEST-SSH-AUTH-001–006 | Unit, E2E |
| SSH reconnection UI | FS-SSH-040–042, FS-SSH-010 | TEST-SSH-RECON-001–004 | E2E |
| Mouse reporting | FS-VT-080–086 | TEST-MOUSE-001–008 | Unit, Integration, E2E |
| Bracketed paste | FS-CLIP-008–009 | TEST-BPASTE-001–005 | Unit, E2E |
| Ctrl+Shift+V paste | FS-CLIP-005, FS-KBD-001–003 | TEST-PASTE-SHORTCUT-001–003 | Unit |
| Activity notifications | FS-NOTIF-001–004, FS-TAB-007 | TEST-NOTIF-001–004 | Integration, E2E |
| Pane focus IPC | FS-PANE-005–006, FS-KBD-003 | TEST-PANE-FOCUS-001–003 | Integration, E2E |
| Credential store SSH | FS-CRED-001, FS-CRED-005–006 | TEST-CRED-001–004 | Unit, Integration |
| OSC title update | FS-VT-060–063, FS-TAB-006 | TEST-OSC-TITLE-001–005 | Unit, Integration |
| Focus events mode 1004 | FS-VT-084, FS-VT-086 | TEST-FOCUS-001–004 | Unit, Integration, E2E |
| DECKPAM | FS-KBD-010 | TEST-DECKPAM-001–004 | Unit, Integration, E2E |

**Total scenarios:** 47
