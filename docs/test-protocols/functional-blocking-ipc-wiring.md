<!-- SPDX-License-Identifier: MPL-2.0 -->

# TauTerm — Functional & Security Test Protocol: Blocking Items & IPC Wiring

**Version:** 1.0
**Date:** 2026-04-05
**Scope:** 12 blocking items identified during bootstrap session j. Covers functional correctness and security validation for features that are stubbed, unwired, or partially implemented. Complements the existing protocols in `docs/test-protocols/` without duplicating their scenarios.

**Prerequisites for all scenarios:**
- TauTerm built in release mode (`pnpm tauri build`) or dev mode (`pnpm tauri dev`)
- A local SSH server reachable at `127.0.0.1:2222` (e.g., `sshd` in a container) for SSH scenarios
- The `secret-tool` CLI available for credential storage verification
- `xdotool` or equivalent for injecting X11 mouse events programmatically

**Legend:**
- PASS — observed result matches expected result exactly
- FAIL — any deviation from expected result
- N/A — precondition not satisfiable in current environment (document reason)

---

## Item 1: Search in Scrollback (FS-SEARCH-001 to 007)

**Context:** `search_pane` command on the Rust side is a stub that always returns `[]`. These scenarios validate both the IPC contract and the UI behaviour once wired.

---

### SEARCH-001 — Basic case-insensitive search finds all matches

**Preconditions:** A pane contains scrollback with at least three occurrences of the word "error" in varying case (`error`, `Error`, `ERROR`). Alternate screen is not active.

**Action:** Open the search overlay (Ctrl+Shift+F). Type `error`. Confirm that case-insensitive mode is active (default).

**Expected result:** All three occurrences are highlighted in the scrollback. A match counter shows `3 matches`. The first match is scrolled into view and visually distinct (current-match style) from the other two.

**Criterion PASS/FAIL:** Exactly three highlighted regions appear, each on the correct line. Current match has a different highlight colour than the others.

---

### SEARCH-002 — Search result count is zero for non-existent term

**Preconditions:** Pane contains scrollback with no occurrence of the string `xyzzy_notfound_42`.

**Action:** Open search overlay. Type `xyzzy_notfound_42`.

**Expected result:** Overlay shows `0 matches` or an equivalent empty-state indicator. No highlights appear. Next/previous navigation controls are disabled or produce no effect.

**Criterion PASS/FAIL:** Zero highlighted regions. No crash or spinner stuck in loading state. `search_pane` IPC call returns `[]` and the UI handles it gracefully.

---

### SEARCH-003 — Search across soft-wrapped lines (FS-SEARCH-002)

**Preconditions:** Terminal width is narrow enough that a long word (e.g., `superlongidentifiername`) wraps across two visual lines. The word exists in the scrollback.

**Action:** Search for `superlongidentifiername`.

**Expected result:** The match is found and highlighted across both visual lines as a single contiguous match. The match counter shows `1 match`.

**Criterion PASS/FAIL:** One highlight spanning the soft-wrap boundary. The term is not split into two separate partial matches, and is not missed.

---

### SEARCH-004 — Search does not operate on alternate screen content (FS-SEARCH-004)

**Preconditions:** vim is open in a pane displaying the word "pattern" on screen. The scrollback (before vim launch) does not contain "pattern".

**Action:** Open search overlay. Type `pattern`.

**Expected result:** `0 matches`. No content from the alternate screen (vim's buffer) is matched.

**Criterion PASS/FAIL:** Zero matches returned. After closing vim and searching again, `0 matches` still, because "pattern" was only on the alternate screen.

---

### SEARCH-005 — Search performance: 100,000-line scrollback under 100 ms (FS-SEARCH-005)

**Preconditions:** Run `seq 1 100000` in a pane to fill the scrollback to 100,000 lines. The search term `99999` appears exactly once.

**Action:** Open search overlay. Type `99999`. Measure time from keypress to first result displayed.

**Expected result:** The result appears in under 100 ms on a mid-range system. Match counter shows `1 match` pointing to the correct line.

**Criterion PASS/FAIL:** Results visible in < 100 ms. Verified by browser performance timeline or manual stopwatch. If search blocks the UI thread during this window, FAIL.

---

### SEARCH-006 — Next/previous navigation centers match in viewport (FS-SEARCH-006)

**Preconditions:** Scrollback contains 5 occurrences of `needle` spread across 500 lines. Search is open, term `needle` entered, all 5 are highlighted.

**Action:** Press Next three times. Then press Previous once.

**Expected result:** Each press of Next/Previous scrolls the viewport so the target match is vertically centered. After pressing Previous once, the third match (not the second) is the current match — i.e., navigation wraps correctly.

**Criterion PASS/FAIL:** Current match is always centered in viewport. Match index counter reflects the correct ordinal (e.g., `3 / 5`, then `2 / 5`).

---

### SEARCH-007 — Case-sensitive toggle distinguishes case (FS-SEARCH-003)

**Preconditions:** Scrollback contains `error`, `Error`, and `ERROR`.

**Action:** Open search overlay. Type `error`. Toggle case-sensitive mode on. Observe match count.

**Expected result:** Only the lowercase `error` occurrence is matched. Counter shows `1 match`. Toggling back to case-insensitive mode returns to `3 matches`.

**Criterion PASS/FAIL:** Match count changes correctly on toggle. No stale highlights remain after toggle.

---

## Item 2: SSH Auth Interactive UI (FS-SSH-010 to 014)

**Context:** The frontend does not listen to `ssh-state-changed`, `credential-prompt`, or `host-key-prompt` events. These scenarios test the full IPC event → UI flow.

---

### SSH-AUTH-001 — Lifecycle states are reflected in the UI (FS-SSH-010)

**Preconditions:** A saved SSH connection to `127.0.0.1:2222` is configured. The server is reachable.

**Action:** Initiate connection. Observe the pane/status bar state during each phase.

**Expected result:**
1. **Connecting** state: pane shows a spinner or "Connecting…" label while TCP handshake is in progress.
2. **Authenticating** state: pane briefly shows "Authenticating…" during credential exchange.
3. **Connected** state: pane transitions to the interactive terminal; status bar shows the SSH indicator.

**Criterion PASS/FAIL:** Each of the three states is visually represented during a normal connection flow. No state is skipped without being displayed. The pane does not go directly from blank to interactive.

---

### SSH-AUTH-002 — Host key prompt displayed on first connection (FS-SSH-011)

**Preconditions:** The TauTerm known-hosts file (`~/.config/tauterm/known_hosts`) does not contain an entry for `127.0.0.1:2222`.

**Action:** Connect to `127.0.0.1:2222`.

**Expected result:** A dialog (or pane overlay) is displayed containing:
- A plain-language explanation (e.g., "Connecting to `127.0.0.1` for the first time…")
- The SHA-256 host key fingerprint
- The key type (e.g., `ED25519`)
- Accept and Reject buttons (Reject is the safe/default-style action)

**Criterion PASS/FAIL:** All four elements are present. Connection does not proceed until the user acts. Clicking Reject aborts the connection without writing to known-hosts.

**Security:** The fingerprint displayed must match `ssh-keyscan -t ed25519 127.0.0.1` output. Any mismatch between displayed and actual fingerprint is a critical defect.

---

### SSH-AUTH-003 — Host key change triggers blocking MITM warning (FS-SSH-011)

**Preconditions:** `~/.config/tauterm/known_hosts` contains a stored key for `127.0.0.1:2222`. The server's host key has been rotated (different key now presented).

**Action:** Attempt to connect to `127.0.0.1:2222`.

**Expected result:** Connection is blocked immediately. A prominent warning dialog is shown with:
- The stored fingerprint (old key)
- The new fingerprint (server's current key)
- An explicit MITM warning in plain language
- Instructions to contact the server administrator
- Default action is **Reject** (visually primary/default button)
- Accept requires a deliberate non-default interaction

**Criterion PASS/FAIL:** Connection does not proceed past this dialog. The old and new fingerprints are shown side-by-side. The Accept path requires at least one additional deliberate step (e.g., a secondary confirmation or a non-primary button press).

**Security:** This scenario is a mandatory security gate. If the dialog does not block the connection or if Accept is the default action, FAIL immediately — this is a MITM vulnerability.

---

### SSH-AUTH-004 — Password prompt appears when publickey auth fails (FS-SSH-012)

**Preconditions:** The saved connection specifies no identity file. The server requires password authentication. `ssh-state-changed` event carries `{ state: "Authenticating" }` followed by `credential-prompt`.

**Action:** Initiate connection. Observe whether a password input dialog is displayed.

**Expected result:** A credential prompt dialog appears asking for the password. The input field is masked. Submitting a correct password completes authentication and transitions to Connected.

**Criterion PASS/FAIL:** Dialog appears without manual intervention. Correct password leads to Connected state. Incorrect password either re-prompts or shows an error and transitions to Disconnected.

**Security:** The password field MUST be masked (no plaintext display). The submitted value MUST NOT appear in browser DevTools console, Tauri logs at any level, or IPC payloads visible in the WebView inspector.

---

### SSH-AUTH-005 — Deprecated algorithm warning is shown but non-blocking (FS-SSH-014)

**Preconditions:** An SSH server configured to offer only `ssh-rsa` (SHA-1) as its host key algorithm is available.

**Action:** Connect to that server. Accept the host key if prompted (first-time TOFU).

**Expected result:** After reaching the Connected state, a dismissible warning banner or overlay is displayed within the pane naming `ssh-rsa` (SHA-1) and stating that the server should be updated. The terminal is fully interactive; the warning does not block input.

**Criterion PASS/FAIL:** Warning is visible. Terminal accepts input. Dismissing the warning removes it. The warning does not reappear on the next keystroke.

---

### SSH-AUTH-006 — TauTerm does not read or write ~/.ssh/known_hosts (FS-SSH-011)

**Preconditions:** `~/.ssh/known_hosts` contains an entry for `127.0.0.1`. TauTerm's own known-hosts file is empty.

**Action:** Connect to `127.0.0.1:2222`. Accept the host key prompt.

**Expected result:** The host key prompt is shown (because TauTerm's own file is empty — it does not consult `~/.ssh/known_hosts`). After acceptance, the key is written to `~/.config/tauterm/known_hosts`, not to `~/.ssh/known_hosts`.

**Criterion PASS/FAIL:** `~/.ssh/known_hosts` is unmodified after the session. `~/.config/tauterm/known_hosts` gains the new entry. Verify both with `stat --format="%Y"` before and after to confirm mtime.

---

## Item 3: SSH Reconnection UI (FS-SSH-040 to 042)

**Context:** No reconnect button exists in the disconnected pane/tab UI.

---

### SSH-RECON-001 — Reconnect button visible in Disconnected state (FS-SSH-040, FS-SSH-041)

**Preconditions:** An established SSH session exists. Network is then disrupted (e.g., `iptables -I OUTPUT -d 127.0.0.1 -p tcp --dport 2222 -j DROP`) until keepalive timeout triggers Disconnected state.

**Action:** Wait for the pane to enter Disconnected state. Observe the pane content.

**Expected result:** The pane displays a clear Disconnected indicator (text, icon, or overlay). A "Reconnect" button (or equivalent action) is visible directly within the pane. The tab header also reflects the disconnected state.

**Criterion PASS/FAIL:** Reconnect action is discoverable without opening any menu. The button is reachable via keyboard alone (focusable, activatable with Enter/Space).

---

### SSH-RECON-002 — Reconnect re-establishes session without reconfiguration (FS-SSH-040)

**Preconditions:** Same as RECON-001. Network is restored before clicking Reconnect.

**Action:** Click the "Reconnect" button.

**Expected result:** The pane cycles through Connecting → Authenticating → Connected states. The same host/port/user credentials are used. No dialog asking for connection parameters appears.

**Criterion PASS/FAIL:** Session is re-established. If credentials are needed (no saved keychain entry), a password prompt is acceptable; a full "new connection" dialog is FAIL.

---

### SSH-RECON-003 — Scrollback preserved with visual separator after reconnection (FS-SSH-042)

**Preconditions:** Session has 200 lines of scrollback. Session drops and reconnects.

**Action:** After reconnect, scroll upward into the scrollback.

**Expected result:** All 200 pre-disconnection lines are intact. A visual separator line (e.g., a horizontal rule with a timestamp or label "--- Reconnected ---") appears at the boundary between the previous session content and new output.

**Criterion PASS/FAIL:** No scrollback content is lost. Separator is visible. New output appears below the separator.

---

### SSH-RECON-004 — Reconnect not available in Closed state (FS-SSH-010)

**Preconditions:** An SSH session whose remote shell exited cleanly (exit code 0).

**Action:** Observe the pane after the remote shell exits normally.

**Expected result:** The pane enters the Closed state. No "Reconnect" button is shown. Only "Close pane" or "New connection" options are available (matching local PTY terminated pane behavior).

**Criterion PASS/FAIL:** Reconnect button absent. Closed state is visually distinct from Disconnected state.

---

## Item 4: Mouse Reporting (FS-VT-080 to 086)

**Context:** Mouse events are not encoded and sent to the PTY. These scenarios verify the full encode-and-write path.

---

### MOUSE-001 — X10 mode (DECSET 9): click coordinates sent to PTY

**Preconditions:** In a pane, run: `printf "\033[?9h"` to enable X10 mouse reporting. Then run `cat -v` to display raw bytes.

**Action:** Click at cell coordinates (col=10, row=5) — i.e., the 10th character column, 5th row.

**Expected result:** `cat -v` displays a mouse report sequence in X10 format: `^[[M` followed by three bytes: button byte (32 for left-click), col byte (32+10=42), row byte (32+5=37).

**Criterion PASS/FAIL:** Bytes `\033[M` + `\x20\x2a\x25` appear in cat output (or equivalent for the coordinates chosen). No extra bytes. Reporting stops after `printf "\033[?9l"`.

---

### MOUSE-002 — Normal mode (DECSET 1000): press and release reported

**Preconditions:** Enable mode 1000: `printf "\033[?1000h"`. Run `cat -v`.

**Action:** Click at (col=5, row=3) and release.

**Expected result:** Two mouse report sequences: one for button-press (button=0), one for button-release (button=3). Both carry the same coordinates.

**Criterion PASS/FAIL:** Both press and release sequences arrive at the PTY. Button byte for release is `32+3=35` (button 3 in X10 encoding = release event).

---

### MOUSE-003 — SGR mode (DECSET 1006): large coordinates encoded correctly

**Preconditions:** Enable SGR mouse mode: `printf "\033[?1000h\033[?1006h"`. Run `cat -v`. Use a terminal at least 220 columns wide.

**Action:** Click at column 220, row 1.

**Expected result:** SGR format report: `\033[<0;220;1M` for press and `\033[<0;220;1m` for release. The coordinate 220 is transmitted as a decimal number, not as a single byte — which is the key advantage of SGR mode over X10 for large terminals.

**Criterion PASS/FAIL:** The sequence `\033[<0;220;1M` appears in cat output verbatim. If a single-byte column encoding is used instead, FAIL (coordinate would overflow 223).

---

### MOUSE-004 — Any-event mode (DECSET 1003): motion events sent

**Preconditions:** Enable mode 1003: `printf "\033[?1000h\033[?1003h"`. Run `cat -v`.

**Action:** Move the mouse over the terminal surface without clicking.

**Expected result:** Motion events are continuously reported to the PTY. Each motion produces a sequence with the button byte indicating motion (32+32=64 for no-button motion in X10 encoding).

**Criterion PASS/FAIL:** Multiple motion sequences appear in cat output during mouse movement. Motion stops reporting after `printf "\033[?1003l\033[?1000l"`.

---

### MOUSE-005 — Shift+Click bypasses mouse reporting, performs selection (FS-VT-083)

**Preconditions:** vim is open with `set mouse=a` (captures all mouse events). Mouse reporting is active.

**Action:** Hold Shift and click within the terminal viewport.

**Expected result:** TauTerm performs a text selection (visible selection highlight). The click is NOT forwarded to vim as a mouse event. vim does not reposition its cursor.

**Criterion PASS/FAIL:** Selection is visible. vim cursor position is unchanged. The shift-click sequence does not appear in vim's event log.

---

### MOUSE-006 — Mouse wheel events forwarded to PTY as button 4/5 (FS-VT-085)

**Preconditions:** Mouse reporting mode 1000 active. Run `cat -v`.

**Action:** Scroll the mouse wheel up over the terminal pane.

**Expected result:** Button 4 events (scroll up) appear in cat output. Scrolling down produces button 5. Shift+Wheel does NOT produce PTY events but instead scrolls TauTerm's scrollback.

**Criterion PASS/FAIL:** Scroll-up → button byte encodes 64+0+64=64 (button 4 in X10: 32 base + 64 motion flag — actually 64 = 32+32, button 4 = 32+64=96 in some encodings; validate against actual xterm behaviour). Shift+Wheel scrolls scrollback, no bytes to PTY.

**Note to implementer:** Confirm the exact byte value by testing against xterm with the same mode enabled. Button 4/5 encoding is `32 + 64 + button_index` in SGR mode or `32+64+n` byte in X10.

---

### MOUSE-007 — Mouse reporting modes reset on application exit (FS-VT-086)

**Preconditions:** Run a program that sets mode 1003 (any-event) and then exits without explicitly disabling it (simulating a crash).

**Action:** After the program exits, click within the terminal pane.

**Expected result:** TauTerm performs a normal selection (no mouse bytes sent to the PTY). Mouse reporting is reset to off state. The terminal is fully usable for selection and scrolling.

**Criterion PASS/FAIL:** After program exit, mouse events are handled locally (selection, scrolling). No spurious bytes appear in subsequent cat sessions. Pane reset clears all active mouse modes.

---

### MOUSE-008 — Focus events (DECSET 1004): ESC [I and ESC [O generated (FS-VT-084)

**Preconditions:** Enable focus event mode: `printf "\033[?1004h"`. Run `cat -v`.

**Action:** Click on another application window (defocusing the TauTerm pane), then click back into the TauTerm pane.

**Expected result:** On defocus: `^[[O` (ESC [ O) appears in cat output. On refocus: `^[[I` (ESC [ I) appears.

**Criterion PASS/FAIL:** Both sequences appear in the correct order. After `printf "\033[?1004l"`, no focus events are sent. This scenario also directly covers Item 11 (focus events mode 1004).

---

## Item 5: Bracketed Paste (FS-CLIP-008)

**Context:** DECSET 2004 is not handled on the frontend side. These scenarios test the full paste-wrapping path.

---

### BPASTE-001 — Pasted text is wrapped with bracketed paste markers

**Preconditions:** A pane is running zsh or bash with bracketed paste enabled (set by the shell automatically; verify with `printf "\033[?2004h"`). The clipboard contains a multi-line snippet: `line1\nline2\nline3`.

**Action:** Paste with Ctrl+Shift+V.

**Expected result:** The pane receives: `ESC [200~` + `line1\nline2\nline3` + `ESC [201~`. The shell does not auto-execute the pasted command. The text appears in the shell's readline buffer, not executed.

**Criterion PASS/FAIL:** The shell prompt shows the pasted text pending. No command execution occurs until the user presses Enter explicitly.

---

### BPASTE-002 — Bracketed paste end sequence stripped from pasted content (FS-CLIP-008)

**Preconditions:** Bracketed paste mode active. Clipboard contains a string that includes the literal bytes `ESC [201~` embedded in the middle of the text (a crafted adversarial payload).

**Action:** Paste the content.

**Expected result:** The embedded `ESC [201~` is stripped from the pasted content before wrapping. The shell receives: `ESC [200~` + (content with the embedded sequence removed) + `ESC [201~`. The shell does not interpret an early end-of-paste.

**Criterion PASS/FAIL:** The pasted content in the shell buffer does not contain the early terminator. The text after the stripped sequence is still present in the buffer.

**Security:** This is an injection-prevention requirement. If the embedded `ESC [201~` is not stripped, a malicious clipboard payload (e.g., from a web page) could terminate the bracketed paste early and inject arbitrary text as if typed by the user — bypassing the user's confirmation of the pasted content.

---

### BPASTE-003 — Legacy paste (no bracketed paste mode): multiline confirmation dialog (FS-CLIP-009)

**Preconditions:** Bracketed paste mode is NOT active in the pane (disable with `printf "\033[?2004l"`). Clipboard contains `command1\ncommand2`.

**Action:** Paste with Ctrl+Shift+V.

**Expected result:** A confirmation dialog appears warning that the clipboard contains multiple lines, listing the content (or a truncated preview). The user must confirm before the text is sent to the PTY.

**Criterion PASS/FAIL:** Dialog is shown before any bytes reach the PTY. Cancelling the dialog sends nothing. Confirming sends the raw text without bracketed paste wrapping.

---

### BPASTE-004 — Single-line paste does not trigger confirmation dialog

**Preconditions:** Bracketed paste mode NOT active. Clipboard contains a single-line string `echo hello`.

**Action:** Paste with Ctrl+Shift+V.

**Expected result:** Text is sent directly to the PTY without any confirmation dialog. No delay from an unnecessary dialog.

**Criterion PASS/FAIL:** No dialog appears. Text reaches the PTY immediately.

---

### BPASTE-005 — Pasted text is not interpreted as escape sequences (FS-CLIP-008)

**Preconditions:** Bracketed paste mode active. Clipboard contains the string `\033[1m` (the literal characters backslash, 0, 3, 3, etc. — NOT a real escape sequence; but also test with actual ESC byte followed by `[1m`).

**Action:** Paste the content containing actual ESC bytes.

**Expected result:** The ESC bytes within the pasted content are NOT processed as terminal control sequences. They are delivered verbatim to the application within the bracketed paste markers. The terminal display is not affected (no bold mode change, no cursor movement, etc.).

**Criterion PASS/FAIL:** After pasting, no SGR attribute change is visible. The pasted bytes appear in the shell buffer as literal characters.

---

## Item 6: Ctrl+Shift+V Paste Shortcut (FS-CLIP-005, FS-KBD-003)

**Context:** Ctrl+Shift+V is not intercepted in `handleGlobalKeydown`.

---

### PASTE-001 — Ctrl+Shift+V pastes from CLIPBOARD (not PRIMARY)

**Preconditions:** Clipboard (CLIPBOARD selection on X11) contains "clipboard_content". PRIMARY selection contains a different string "primary_content" (achieved by selecting "primary_content" text in another application).

**Action:** In a TauTerm pane running `cat`, press Ctrl+Shift+V.

**Expected result:** "clipboard_content" is pasted into the terminal. "primary_content" is NOT pasted.

**Criterion PASS/FAIL:** The correct selection source is used. Middle-click in the same pane should paste "primary_content" (verifying the two selections are distinct).

---

### PASTE-002 — Ctrl+Shift+V does not forward the key to the PTY

**Preconditions:** A pane running `cat -v` (to display all raw bytes).

**Action:** Press Ctrl+Shift+V.

**Expected result:** The clipboard content is pasted. No key encoding of Ctrl+V (`^V` = 0x16) or Ctrl+Shift+V appears in the cat output — the shortcut is fully consumed by TauTerm.

**Criterion PASS/FAIL:** `^V` does not appear in cat output. Only the pasted text bytes appear.

---

### PASTE-003 — Ctrl+Shift+V is inactive when no shortcut is bound (FS-KBD-002)

**Preconditions:** The Paste shortcut has been removed in preferences (FS-KBD-002 allows removing shortcuts).

**Action:** Press Ctrl+Shift+V in a pane running `cat -v`.

**Expected result:** The key combination is passed to the PTY. `^V` (0x16) appears in cat output. No paste occurs.

**Criterion PASS/FAIL:** `^V` byte appears in cat output when the shortcut is unbound. The PTY receives the key.

---

## Item 7: Activity Notification Events (FS-NOTIF-001 to 004)

**Context:** `notification-changed` event is not listened to in the frontend.

---

### NOTIF-001 — Background tab output triggers visual activity indicator (FS-NOTIF-001)

**Preconditions:** Two tabs are open. Tab 2 is active.

**Action:** In Tab 1 (background), run a command that produces output: `sleep 1 && echo done`. Wait for it to emit output.

**Expected result:** Tab 1's header displays a visual activity indicator (e.g., a dot, a coloured underline, or a highlight) within the tab label area. The indicator is visible without switching to Tab 1.

**Criterion PASS/FAIL:** Indicator appears on Tab 1 header. Switching to Tab 1 clears the indicator (FS-NOTIF-003). The indicator does not appear on the currently active Tab 2.

---

### NOTIF-002 — Process termination in background tab shows distinct indicator (FS-NOTIF-002)

**Preconditions:** Two tabs. Tab 2 is active. Tab 1 has a running shell.

**Action:** In Tab 1 (background), run `exit`. The shell terminates.

**Expected result:** Tab 1 shows a distinct "process ended" indicator that is visually different from the output activity indicator (e.g., different colour, icon, or symbol).

**Criterion PASS/FAIL:** Two different indicators are used — one for output activity, one for process termination. They are visually distinguishable.

---

### NOTIF-003 — Switching to notified tab clears the indicator (FS-NOTIF-003)

**Preconditions:** Tab 1 has an activity indicator showing.

**Action:** Click on Tab 1 to make it active.

**Expected result:** The activity indicator is removed from Tab 1's header immediately upon activation.

**Criterion PASS/FAIL:** Indicator disappears on tab switch. It does not persist after the tab is focused.

---

### NOTIF-004 — Bell in background tab produces visual indicator (FS-NOTIF-004)

**Preconditions:** Two tabs. Tab 2 is active. Tab 1 is running a shell.

**Action:** In Tab 1 (background), run `printf "\007"` to emit a BEL character.

**Expected result:** Tab 1 shows a bell indicator (same as or related to the activity indicator). If the notification type is set to "visual bell", a visual indicator appears on the tab header.

**Criterion PASS/FAIL:** An indicator appears on Tab 1 when BEL is received. No audible bell plays unless the user has configured audible bell.

---

## Item 8: Pane Focus via Mouse Click → set_active_pane (FS-PANE-005)

**Context:** Clicking a pane does not call `set_active_pane` on the backend.

---

### PANE-FOCUS-001 — Clicking an inactive pane makes it active in the backend

**Preconditions:** A tab with two panes (split). Pane A is active, Pane B is inactive.

**Action:** Click anywhere within Pane B.

**Expected result:** Pane B becomes visually active (distinct border or highlight, per FS-PANE-006). The backend's `set_active_pane` is invoked with Pane B's ID. Subsequent keyboard input goes to Pane B's PTY.

**Criterion PASS/FAIL:** After clicking Pane B, typing a command executes in Pane B's shell (not Pane A's). Verify by running `echo PANEB` and observing which pane displays the output.

---

### PANE-FOCUS-002 — Keyboard input after focus switch targets the correct PTY

**Preconditions:** Two panes, each running `cat -v`. Pane A is active.

**Action:** Click Pane B to focus it. Type `hello`.

**Expected result:** "hello" appears in Pane B's `cat` output. Pane A receives no input.

**Criterion PASS/FAIL:** Input is routed to Pane B exclusively. No bytes appear in Pane A.

---

### PANE-FOCUS-003 — Active pane is visually distinguished from inactive panes (FS-PANE-006)

**Preconditions:** Three panes open.

**Action:** Click each pane in succession.

**Expected result:** At any time, exactly one pane has the "active" visual style (e.g., brighter border). The previously active pane reverts to the inactive style. The visual update is immediate (no animation delay that could leave both panes appearing active).

**Criterion PASS/FAIL:** One and only one pane shows the active style at all times. Verified for all three pane transitions.

---

### PANE-FOCUS-004 — Keyboard shortcut focus navigation also updates backend state

**Preconditions:** Two panes. Pane A is active. Both run `cat -v`.

**Action:** Use Ctrl+Shift+Right to navigate focus to Pane B (keyboard shortcut per FS-KBD-003).

**Expected result:** Pane B becomes active in the UI and in the backend. Typing sends input to Pane B's PTY.

**Criterion PASS/FAIL:** Input routing matches the visually focused pane after keyboard navigation.

---

## Item 9: SSH Credential Store Wiring (FS-CRED-001, FS-CRED-005)

**Context:** `CredentialManager` is not wired in `ssh_cmds.rs`.

---

### CRED-001 — Saved password is stored in OS keychain, not in preferences file (FS-CRED-001)

**Preconditions:** Configure a saved SSH connection with password authentication. Enter and save the password.

**Action:** Inspect `~/.config/tauterm/preferences.json` (or equivalent config path) for any plaintext password.

**Expected result:** The preferences file contains no password string. The credential is retrievable via `secret-tool lookup service tauterm host 127.0.0.1` (or equivalent schema).

**Criterion PASS/FAIL:** `grep -i password ~/.config/tauterm/preferences.json` returns nothing. `secret-tool lookup` returns the stored password.

**Security:** Presence of any plaintext credential in the preferences file is an immediate FAIL and a P0 security defect.

---

### CRED-002 — Credentials are not logged at any log level (FS-CRED-004)

**Preconditions:** Enable maximum log verbosity (set `RUST_LOG=trace`). Configure a saved SSH connection with a known password.

**Action:** Connect to the SSH server. Observe the log output.

**Expected result:** The password string does not appear anywhere in the log output. The `Debug` representation of the credential struct is redacted (e.g., `Credentials { password: "[REDACTED]" }`).

**Criterion PASS/FAIL:** `grep -i <password_string> <log_file>` returns zero matches. All log levels checked including trace.

---

### CRED-003 — Credential not cached beyond authentication handshake (FS-CRED-003)

**Preconditions:** An SSH connection is established with a password. Connection is now in Connected state.

**Action:** Use `gcore` or `/proc/<pid>/mem` inspection to search for the password string in the process memory after authentication completes.

**Expected result:** The password string is not found in process memory after successful authentication. Memory has been zeroed or the string has gone out of scope.

**Criterion PASS/FAIL:** Password bytes not locatable in process memory dump after authentication phase. This test may require a debug build with known memory layout.

**Note:** This is best-effort given Rust's memory safety model. If the password exists in a `String` that has been dropped and the page not yet reused, the bytes may still be readable. The test passes if no live reference to the credential exists in the application's reachable memory graph.

---

### CRED-004 — No Secret Service provider: credential prompt on each connection, no fallback storage (FS-CRED-005)

**Preconditions:** The Secret Service D-Bus service is not running (stop `gnome-keyring-daemon` or equivalent). A saved connection with password auth exists.

**Action:** Attempt to connect.

**Expected result:** A password prompt dialog is displayed. A notice is shown stating that credential persistence is unavailable (no keychain). After entering the correct password and connecting, the password is not saved anywhere on disk.

**Criterion PASS/FAIL:** Password prompt appears. Notice about unavailable persistence is shown. After disconnecting and reconnecting, the prompt appears again. No credentials file on disk.

**Security:** Any fallback to plaintext file storage is an immediate FAIL.

---

### CRED-005 — Identity file path traversal rejected at connection time (FS-CRED-006)

**Preconditions:** A saved connection is configured with identity file path `../../etc/shadow`.

**Action:** Attempt to connect using this saved connection.

**Expected result:** The connection attempt is rejected before any network activity. An error message is shown in the UI indicating that the identity file path is invalid. The path `../../etc/shadow` (or any path-traversal form) is never opened or read.

**Criterion PASS/FAIL:** Connection fails at path validation stage. Error message displayed. `/etc/shadow` is not accessed (verify with `strace -e openat` on the TauTerm process).

**Security:** If the path is successfully opened and read, this is a path traversal vulnerability — immediate P0 defect.

---

## Item 10: OSC Title Update Propagation (FS-VT-060 to 062, FS-TAB-006)

**Context:** OSC 0/1/2 sequences are parsed by the VT backend but do not propagate the title to the frontend tab.

---

### OSC-TITLE-001 — OSC 0 sets tab title (FS-VT-060)

**Preconditions:** A pane with the default tab title.

**Action:** Run `printf "\033]0;MyCustomTitle\007"` in the pane.

**Expected result:** The tab header updates to display "MyCustomTitle" within one render cycle (no manual refresh needed).

**Criterion PASS/FAIL:** Tab title changes immediately. The `screen-update` event (or a dedicated `title-changed` event) carrying the new title is received by the frontend.

---

### OSC-TITLE-002 — OSC 2 also sets tab title; OSC 1 (icon title) may be ignored (FS-VT-060)

**Preconditions:** Same as OSC-TITLE-001.

**Action:** Run `printf "\033]2;AnotherTitle\007"`.

**Expected result:** Tab title updates to "AnotherTitle". OSC 1 (icon name) may be silently ignored (no crash, no title corruption).

**Criterion PASS/FAIL:** OSC 2 updates the title. OSC 1 causes no error or garbled title.

---

### OSC-TITLE-003 — Title is sanitized: control characters stripped, length capped at 256 (FS-VT-062)

**Preconditions:** A pane.

**Action 1:** Run `printf "\033]0;Title\x01\x1b[31mRed\007"`. This embeds a C0 control character and an SGR sequence.

**Action 2:** Run `printf "\033]0;$(python3 -c 'print("A"*300)')\007"`. This sends a 300-character title.

**Expected result (Action 1):** Tab title shows "TitleRed" or "Title[31mRed" with control characters stripped, but no color effect or cursor movement. The literal SGR bytes are rendered as plain text or stripped.

**Expected result (Action 2):** Tab title is capped at 256 characters.

**Criterion PASS/FAIL:** Control characters do not affect the terminal display outside the title. Title length does not exceed 256 chars.

**Security:** This test specifically validates that OSC title injection cannot corrupt the terminal or inject display sequences into the UI chrome. An SGR sequence in the title MUST NOT change the tab bar's text color.

---

### OSC-TITLE-004 — User-defined tab label takes precedence over OSC title (FS-TAB-006)

**Preconditions:** The user has set a custom label "MyLabel" on a tab (via double-click or context menu rename).

**Action:** Run `printf "\033]0;ProcessTitle\007"` in that tab's pane.

**Expected result:** The tab title remains "MyLabel". The OSC sequence is processed by the backend (no error) but the user-set label takes precedence in the frontend display.

**Criterion PASS/FAIL:** Tab still shows "MyLabel" after the OSC sequence runs. Clearing the user label (editing and submitting empty string) reverts to OSC-driven title "ProcessTitle".

---

### OSC-TITLE-005 — CSI 21t read-back produces no response (FS-VT-063)

**Preconditions:** A pane running `cat -v`.

**Action:** Run `printf "\033[21t"` (report window title sequence).

**Expected result:** `cat -v` receives no bytes. No title string is injected into the PTY input stream.

**Criterion PASS/FAIL:** Zero bytes appear in cat output after the sequence. This is a security requirement — OSC injection via read-back must be prevented.

**Security:** If any bytes appear in response, this is a terminal injection vulnerability. A crafted sequence could inject arbitrary commands into the shell's input via the title read-back mechanism.

---

## Item 11: Focus Events Mode 1004 (FS-VT-084)

*Coverage provided by MOUSE-008 above (generating ESC [I / ESC [O on pane focus/defocus). The following scenarios cover edge cases not addressed there.*

---

### FOCUS-001 — Focus events not sent when mode 1004 is not active

**Preconditions:** Mode 1004 is not enabled (default state). A pane running `cat -v`.

**Action:** Click away from the pane (defocus) and click back (refocus).

**Expected result:** No focus event bytes appear in cat output.

**Criterion PASS/FAIL:** Zero bytes in cat output for focus/defocus when mode is not set.

---

### FOCUS-002 — Focus events reset on PTY process exit

**Preconditions:** Run a program that enables mode 1004 then exits without disabling it. `cat -v` is running in the same pane after the program exits.

**Action:** Click away and back into the pane.

**Expected result:** No focus event bytes appear. The mode was reset when the PTY's screen state was reset.

**Criterion PASS/FAIL:** No spurious focus bytes after program exit without explicit mode disable.

---

### FOCUS-003 — vim responds to focus events for autoread

**Preconditions:** Mode 1004 active (set by vim automatically). vim is open with `set autoread`.

**Action:** Modify a file externally while vim is defocused, then refocus by clicking the pane.

**Expected result:** vim detects the focus event (ESC [I) and triggers autoread, updating the buffer with the external change.

**Criterion PASS/FAIL:** vim's buffer updates without manual `:e`. This validates end-to-end application compatibility, not just byte generation.

---

## Item 12: DECKPAM / DECKPNM Keypad Application Mode (FS-KBD-010)

**Context:** `ESC =` (DECKPAM) and `ESC >` (DECKPNM) are ignored in `keyboard.ts`.

---

### DECKPAM-001 — Keypad application mode: numpad keys send application sequences

**Preconditions:** Enable keypad application mode: `printf "\033="`. Run `cat -v`.

**Action:** Press the numpad `5` key (KP_5, not the regular digit 5).

**Expected result:** The sequence `ESC O u` is sent to the PTY (`\033Ou` — the application mode encoding for KP_5 in xterm).

**Criterion PASS/FAIL:** `^[Ou` appears in cat output for KP_5 press. If the regular digit `5` appears, FAIL (numeric mode behaviour).

**Note:** Browsers expose numpad keys via `KeyboardEvent.code = "Numpad5"` and `key = "5"` (or `key = "Clear"` depending on NumLock). The frontend must distinguish `Numpad*` codes from `Digit*` codes.

---

### DECKPAM-002 — Keypad numeric mode (DECKPNM): numpad keys send digits

**Preconditions:** Disable keypad application mode: `printf "\033>"` (DECKPNM = numeric mode, default). Run `cat -v`.

**Action:** Press numpad `5`.

**Expected result:** The digit `5` (0x35) is sent to the PTY. No application-mode escape sequence.

**Criterion PASS/FAIL:** `5` appears in cat output (not `^[Ou`).

---

### DECKPAM-003 — Application switches mode; mode persists until explicitly changed

**Preconditions:** Start in numeric mode. Run a program (e.g., a custom script) that sends `ESC =` then reads keypad input, then sends `ESC >`, then exits.

**Action:** Press KP_1 during application mode, then KP_1 again after the program restores numeric mode.

**Expected result:**
- During application mode: `ESC O q` (`\033Oq`) for KP_1.
- After restore: `1` (0x31) for KP_1.

**Criterion PASS/FAIL:** The mode switch is reflected in the encoding immediately. No restart or pane reset needed.

---

### DECKPAM-004 — vim numeric mode operations: cursor keys unaffected by DECKPAM

**Preconditions:** vim is open. vim sets DECKPAM internally.

**Action:** Navigate with arrow keys. Use the numpad Enter key (KP_Enter) for confirmation.

**Expected result:** Arrow keys function correctly in normal and insert mode. KP_Enter sends `ESC O M` in application mode (same as regular Enter in vim's context — vim maps both). vim navigation is fully functional.

**Criterion PASS/FAIL:** vim is usable. Arrow keys navigate. No phantom characters inserted due to incorrect numpad encoding.

---

## Appendix: Traceability Matrix

| Scenario group | FS references |
|---|---|
| SEARCH-001 to 007 | FS-SEARCH-001, 002, 003, 004, 005, 006, 007 |
| SSH-AUTH-001 to 006 | FS-SSH-010, 011, 012, 014 |
| SSH-RECON-001 to 004 | FS-SSH-040, 041, 042, 010 |
| MOUSE-001 to 008 | FS-VT-080, 081, 082, 083, 084, 085, 086 |
| BPASTE-001 to 005 | FS-CLIP-008, 009 |
| PASTE-001 to 003 | FS-CLIP-005, FS-KBD-002, FS-KBD-003 |
| NOTIF-001 to 004 | FS-NOTIF-001, 002, 003, 004 |
| PANE-FOCUS-001 to 004 | FS-PANE-005, FS-PANE-006, FS-KBD-003 |
| CRED-001 to 005 | FS-CRED-001, 002, 003, 004, 005, 006 |
| OSC-TITLE-001 to 005 | FS-VT-060, 061, 062, 063, FS-TAB-006 |
| FOCUS-001 to 003 | FS-VT-084 |
| DECKPAM-001 to 004 | FS-KBD-010 |

**Security scenarios (require dedicated security review pass):**
- SSH-AUTH-002, SSH-AUTH-003 — TOFU model, MITM warning
- SSH-AUTH-004 — credential leakage in IPC/logs
- BPASTE-002 — injection via embedded bracketed paste terminator
- CRED-001, CRED-002, CRED-003, CRED-004, CRED-005 — credential storage and lifecycle
- OSC-TITLE-003, OSC-TITLE-005 — OSC injection, title read-back injection
