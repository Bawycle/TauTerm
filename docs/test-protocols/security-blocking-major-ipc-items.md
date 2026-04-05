<!-- SPDX-License-Identifier: MPL-2.0 -->

# Security Test Protocol — Blocking and Major IPC Items

Version: 1.0 — 2026-04-05
Scope: Linux (x86, x86_64, ARM32, ARM64, RISC-V) — v1 only
Author role: security-expert

---

## Companion documents

- `docs/test-protocols/security-pty-ipc-ssh-credentials-csp-osc52.md` —
  base protocol (SEC-PTY-*, SEC-IPC-*, SEC-SSH-*, SEC-CRED-*, SEC-CSP-*,
  SEC-OSC-*, SEC-PATH-*, SEC-UI-*, SEC-SSH-CH-*)
- `docs/test-protocols/security-blocking-ipc-wiring.md` —
  extension for seven IPC wiring items (SEC-BLK-001–020)

This document adds scenarios for items not fully covered by those two
companions: SSH reconnection, Ctrl+Shift+V paste, notification badges,
pane focus IPC, and DECKPAM. Items already addressed (scrollback search,
SSH interactive auth, credential store, mouse reporting, bracketed paste,
OSC title, focus events mode 1004) are cross-referenced below without
duplication.

---

## Table of Contents

1. [Threat Model — Incremental Additions](#1-threat-model--incremental-additions)
2. [Security Test Scenarios](#2-security-test-scenarios)
   - 2.1 [Scrollback Search](#21-scrollback-search) — cross-reference
   - 2.2 [SSH Interactive Auth](#22-ssh-interactive-auth) — cross-reference
   - 2.3 [SSH Reconnection UI](#23-ssh-reconnection-ui)
   - 2.4 [Mouse Reporting](#24-mouse-reporting) — cross-reference
   - 2.5 [Bracketed Paste](#25-bracketed-paste) — cross-reference
   - 2.6 [Ctrl+Shift+V Paste](#26-ctrlshiftv-paste)
   - 2.7 [Notification Badges](#27-notification-badges)
   - 2.8 [Pane Focus IPC](#28-pane-focus-ipc)
   - 2.9 [Credential Store SSH](#29-credential-store-ssh) — cross-reference
   - 2.10 [OSC Title Update](#210-osc-title-update) — cross-reference
   - 2.11 [Focus Events Mode 1004](#211-focus-events-mode-1004) — cross-reference
   - 2.12 [DECKPAM Keypad Mode](#212-deckpam-keypad-mode)
3. [Validation Requirements Summary](#3-validation-requirements-summary)

---

## 1. Threat Model — Incremental Additions

The base threat model in §1 of the companion document covers the primary
assets (credentials, PTY input/output, clipboard, IPC channel, WebView
renderer) and threat actors (malicious remote server, malicious local
process, network attacker, local user with filesystem access).

This section documents additional attack surface entries specific to the
items covered here.

### 1.1 Additional Attack Surface

| Surface | Entry Point | Notes |
|---------|-------------|-------|
| SSH reconnect command | `SshManager::reconnect` — `src-tauri/src/ssh/manager.rs:574` | Reconnect re-opens a connection using the stored `SshConnectionConfig`. Credential re-fetch must come from the OS keychain, not from in-memory cache (SEC-SSH-CH-007). |
| Desktop notification title/body | `LinuxNotifications::notify` — `src-tauri/src/platform/notifications_linux.rs:22` | The `title` and `body` parameters are passed verbatim to `notify-rust`, which sends them over D-Bus to the notification daemon. The title originates from the tab title (OSC 0/1/2), which is sanitized by `parse_osc()` — but the body path is not yet fully traced. |
| Clipboard read at paste | `get_clipboard` IPC command — invoked by frontend on Ctrl+Shift+V | Clipboard content from an external application is pasted verbatim into the PTY. No sanitization is performed — correct behavior for a terminal. The risk is that the user is unaware of malicious content in the clipboard. |
| `set_active_pane` IPC | `session_cmds::set_active_pane` — `src-tauri/src/commands/session_cmds.rs:79` | Accepts a `PaneId` from the WebView. Could be used to steal focus from a sensitive pane or to redirect mouse/keyboard input unexpectedly. |
| DECKPAM mode state | `ModeState.deckpam` — `src-tauri/src/vt/modes.rs:22` | A malicious PTY process can toggle keypad application mode at will. The mode state is communicated to the frontend via `mode-state-changed` IPC event. If the frontend misapplies the mode flag, key sequences are mis-encoded, potentially injecting unintended input. |

---

## 2. Security Test Scenarios

### 2.1 Scrollback Search

Covered by SEC-BLK-001, SEC-BLK-002, SEC-BLK-003 in
`docs/test-protocols/security-blocking-ipc-wiring.md`.

Key requirements for implementors:
- Use the `regex` crate (linear-time NFA/DFA), not `fancy-regex` or PCRE.
- Validate `query.text.len() <= MAX_SEARCH_QUERY_LEN` (suggested: 1024
  chars) before any regex compilation.
- `SearchMatch` must not include matched text in its payload — position
  fields only.

---

### 2.2 SSH Interactive Auth

Covered by SEC-BLK-004, SEC-BLK-005, SEC-BLK-006 and by SEC-SSH-001
through SEC-SSH-006, SEC-SSH-CH-001 through SEC-SSH-CH-010 in the
companion documents.

Key requirements for implementors:
- `HostKeyPromptEvent.host` MUST originate from `SshConnectionConfig.host`,
  never from server-supplied data.
- `SshStateChangedEvent.reason` MUST NOT contain credential material;
  use a fixed format string for auth-failure cases.
- TOFU confirmation dialog MUST be a frontend DOM overlay driven by IPC
  events, not in-pane text from server output.

---

### 2.3 SSH Reconnection UI

The reconnect flow (`SshManager::reconnect`,
`src-tauri/src/ssh/manager.rs:574`) re-uses the stored `SshConnectionConfig`
and calls `open_connection` with `credentials: None`, forcing a fresh
credential prompt via `CredentialManager`. This design was validated against
SEC-SSH-CH-007. The scenarios below cover residual risks.

#### SEC-RECON-001

| Field | Value |
|-------|-------|
| **ID** | SEC-RECON-001 |
| **STRIDE** | Tampering / Elevation of Privilege |
| **FS requirement(s)** | FS-SSH-040, FS-SSH-042 |
| **Threat** | The reconnect IPC command (`reconnect_ssh`) is called for a pane that is currently in `Connected` state (not disconnected). If the implementation does not guard against this, a malicious script in the WebView can forcibly cycle an active SSH session, causing the running process to be interrupted, and then capture the re-authentication credentials. |
| **Préconditions** | An SSH session is in `Connected` state. |
| **Action** | Call `reconnect_ssh` (or equivalent) with the `pane_id` of a connected pane. |
| **Résultat attendu (sécurisé)** | The command returns `Err(TauTermError)` — reconnect is only valid for panes in `Disconnected` state. The existing connection is not touched. No credential prompt is triggered. |
| **Risque si non mitigé** | High — forced disconnect and re-auth disrupts the user's session and may expose credentials in an unexpected context. |
| **Mitigation attendue dans le code** | `SshManager::reconnect` must check that the existing connection entry is in `Disconnected` state before removing it. If the state is `Connected`, `Connecting`, or `Authenticating`, return `Err(SshError::InvalidState)`. |

---

#### SEC-RECON-002

| Field | Value |
|-------|-------|
| **ID** | SEC-RECON-002 |
| **STRIDE** | Information Disclosure |
| **FS requirement(s)** | FS-SSH-040, FS-CRED-003, FS-CRED-004 |
| **Threat** | Between disconnection and the user triggering reconnect, the `SshConnection` entry in `SshManager.connections` still holds a reference to the `SshConnectionConfig`. If a prior bug caused the `Credentials` struct to be stored on the connection entry (violating SEC-SSH-CH-007), the password would persist in memory for an unbounded duration. |
| **Préconditions** | A password-authenticated SSH session has disconnected. The reconnect UI is visible but the user has not yet clicked Reconnect. |
| **Action** | Code review — inspect `SshConnection` struct fields. Assert no `Credentials` or `SecVec<u8>` field exists on the struct. |
| **Résultat attendu (sécurisé)** | `SshConnection` stores only `config: SshConnectionConfig` (host, port, username, identity path — no password) and lifecycle state. Memory dumps of the TauTerm process in the disconnected-but-not-yet-reconnected window contain no credential material from the previous session. |
| **Risque si non mitigé** | Critical — password remains in memory indefinitely for flaky connections (extends SEC-SSH-CH-007 to the reconnect idle window). |
| **Mitigation attendue dans le code** | Structural review of `src-tauri/src/ssh/manager.rs` — `SshConnection` definition. No `Credentials` field permitted. Credentials cleared by `SecVec` zeroize immediately after authentication in the connection task (per FS-CRED-003). |

---

#### SEC-RECON-003

| Field | Value |
|-------|-------|
| **ID** | SEC-RECON-003 |
| **STRIDE** | Spoofing |
| **FS requirement(s)** | FS-SSH-040, FS-SSH-042 |
| **Threat** | A malicious process in the pane outputs a fake "Reconnect" button via escape sequences, visually identical to TauTerm's ProcessTerminatedPane / DisconnectBanner overlay. The user clicks the fake button, which is actually a hyperlink (OSC 8) pointing to an attacker-controlled URI or a crafted command that triggers the real reconnect with attacker-supplied parameters. |
| **Préconditions** | The pane is in a `Disconnected` SSH state. The malicious output was pre-loaded before the disconnect. |
| **Action** | The pane's last screen state, drawn before disconnect, contains a fake "Reconnect" overlay crafted to look like TauTerm's native UI. The user interacts with it. |
| **Résultat attendu (sécurisé)** | TauTerm's reconnect button is an HTML overlay rendered outside the terminal canvas (a Svelte component), not a PTY-drawn element. The terminal canvas is strictly below the overlay layer in the DOM z-index stack. User clicks on the overlay DOM element trigger the IPC reconnect command directly — they cannot be intercepted by in-pane content. Mouse clicks on the terminal canvas area, when an overlay is visible, are either captured by the overlay or ignored. |
| **Risque si non mitigé** | High — user initiates reconnect via a spoofed UI element, potentially connecting to a different host or supplying credentials to an attacker. |
| **Mitigation attendue dans le code** | Verify that `DisconnectBanner.svelte` (or equivalent) is rendered as an absolutely-positioned DOM overlay with `z-index` above the terminal canvas. When the overlay is visible, pointer events on the terminal canvas below it must be blocked (`pointer-events: none` on the canvas, or an intercepting layer). Architecture review of the pane component layout required. |

---

#### SEC-RECON-004

| Field | Value |
|-------|-------|
| **ID** | SEC-RECON-004 |
| **STRIDE** | Tampering |
| **FS requirement(s)** | FS-SSH-042 |
| **Threat** | The scrollback preserved through reconnect contains sensitive output from the previous session (e.g., a printed password, a private key, environment variables). The reconnect does not clear the scrollback. A new process running in the same pane — potentially a less-trusted session reconnected to a different host — can read the previous session's output via `get_pane_screen_snapshot`. |
| **Préconditions** | A session produces sensitive output. It disconnects. The user reconnects to a different connection profile (or an attacker triggers reconnect to a different host via a race). |
| **Action** | After reconnect to a different host, call `get_pane_screen_snapshot`. Observe whether output from the previous session is present in the snapshot. |
| **Résultat attendu (sécurisé)** | Scrollback is preserved through reconnect to the same connection (FS-SSH-042). However, the reconnect command must validate that the `SshConnectionConfig` used for reconnection matches the original connection's `host` and `port` — the reconnect operation is not a general-purpose "connect to any host on this pane" operation. If the host/port differs, the command must be rejected. This prevents cross-connection scrollback contamination. |
| **Risque si non mitigé** | Medium — sensitive data from session A leaks into session B's visible scrollback if pane reuse crosses connection boundaries without clearing. |
| **Mitigation attendue dans le code** | `reconnect_ssh` IPC command must not accept a new `SshConnectionConfig` as a parameter. It reconnects to the same host using the stored config only. Cross-connection pane reuse requires a `create_tab` + explicit connection initiation. |

---

### 2.4 Mouse Reporting

Covered by SEC-BLK-009, SEC-BLK-010, SEC-BLK-011 in
`docs/test-protocols/security-blocking-ipc-wiring.md`.

Key requirements for implementors:
- `encode_sgr` and `encode_urxvt` must clamp coordinates to screen
  dimensions before encoding.
- The mouse event IPC command must validate `col` and `row` against current
  pane dimensions and reject out-of-range values.
- Mouse events must only be forwarded to the active pane's PTY.
- All mouse reporting modes must be reset on session cleanup.

---

### 2.5 Bracketed Paste

Covered by SEC-BLK-012, SEC-BLK-013, SEC-BLK-014 in
`docs/test-protocols/security-blocking-ipc-wiring.md`.

Key requirements for implementors:
- Strip all occurrences of `\x1b[201~` from paste payload before wrapping
  (Critical — FS-CLIP-008).
- Strip `\x00` null bytes from paste payload.
- When `bracketed_paste == false` and pasted text contains `\n`/`\r`,
  emit a confirmation event before writing to PTY (FS-CLIP-009).

---

### 2.6 Ctrl+Shift+V Paste

Ctrl+Shift+V reads from the CLIPBOARD X selection and writes the content
to the active pane's PTY. The read is performed by the `get_clipboard` IPC
command (`arboard` backend). The write path is the same as any PTY input
(`send_input`). Bracketed paste wrapping applies here (FS-CLIP-008) —
see §2.5 above.

#### SEC-PASTE-001

| Field | Value |
|-------|-------|
| **ID** | SEC-PASTE-001 |
| **STRIDE** | Tampering |
| **FS requirement(s)** | FS-CLIP-005, FS-CLIP-008 |
| **Threat** | A web page or GUI application places a crafted string on the CLIPBOARD that, when pasted into a terminal, executes commands. Classic example: `echo "safe text"\nrm -rf ~/important`. The user copies text believing it is safe; Ctrl+Shift+V pastes and executes. |
| **Préconditions** | Bracketed paste mode is NOT active. The clipboard contains multi-line content including a newline. |
| **Action** | Set clipboard to `"safe text\nrm -rf ~/important"`. Press Ctrl+Shift+V. |
| **Résultat attendu (sécurisé)** | The frontend detects the newline in the pasted content, checks `ModeState.bracketed_paste` (which is `false` for this scenario), and emits a confirmation dialog before writing to PTY. The user must explicitly confirm. The command is not auto-executed. This is the FS-CLIP-009 multi-line confirmation dialog requirement. |
| **Risque si non mitigé** | Critical — silent arbitrary command execution via clipboard paste. A widely-cited attack vector ("clipboard hijacking"). |
| **Mitigation attendue dans le code** | The frontend paste handler (invoked by Ctrl+Shift+V) reads clipboard, checks for `\n` or `\r`, and if found, checks `bracketed_paste` mode. If `false`, emits a UI confirmation event. Only sends to PTY after user confirmation. Integration test: paste a multi-line string with bracketed paste disabled; assert the shell does not execute until confirmation is given. |

---

#### SEC-PASTE-002

| Field | Value |
|-------|-------|
| **ID** | SEC-PASTE-002 |
| **STRIDE** | Tampering |
| **FS requirement(s)** | FS-CLIP-005, FS-CLIP-008 |
| **Threat** | Even with the multi-line confirmation dialog active, a malicious clipboard payload uses invisible Unicode characters (e.g., U+200B zero-width space, U+FEFF BOM, U+00AD soft hyphen) to embed a hidden newline-equivalent or control character that causes command execution after a visually-clean preview is shown in the confirmation dialog. |
| **Préconditions** | The confirmation dialog displays a preview of the clipboard content. The preview renders the content as text, and invisible characters are not visible. |
| **Action** | Set clipboard to `ls\u200B\nrm -rf ~`. The preview shows `ls` (invisible character and second line hidden). User confirms. Both commands execute. |
| **Résultat attendu (sécurisé)** | The confirmation dialog preview shows the raw content with potentially dangerous characters made visible (e.g., newlines displayed as `↵`, control characters displayed as their Unicode control picture equivalents U+2400–U+2426). This is a UX requirement with security implications: the user must be able to see what they are about to paste. The dialog MUST NOT hide or collapse lines. |
| **Risque si non mitigé** | High — the confirmation dialog provides false assurance; user confirms a payload they cannot fully see. |
| **Mitigation attendue dans le code** | The confirmation dialog component must render the paste preview with all control characters and newlines made visible. Suggest: `white-space: pre` CSS on the preview and a character count. This is a frontend UX-security requirement for the paste confirmation dialog implementation. |

---

#### SEC-PASTE-003

| Field | Value |
|-------|-------|
| **ID** | SEC-PASTE-003 |
| **STRIDE** | Information Disclosure |
| **FS requirement(s)** | FS-CLIP-005 |
| **Threat** | The `get_clipboard` IPC call returns clipboard content to the frontend. If the frontend stores the clipboard content in a module-level `$state` variable (not scoped to the paste action), the clipboard data persists in JavaScript memory and is accessible to any code that runs after the paste, including hypothetically injected scripts. |
| **Préconditions** | Clipboard contains sensitive content (e.g., a password copied from a password manager). |
| **Action** | Code review — verify that the result of `invoke('get_clipboard')` is consumed immediately for the paste write and not stored in a persistent reactive variable. |
| **Résultat attendu (sécurisé)** | The clipboard content is a local variable scoped to the paste handler function. It is passed directly to the PTY write call and then goes out of scope. No module-level `$state` stores clipboard content. No `console.log` of clipboard value. |
| **Risque si non mitigé** | Medium — clipboard content persists in JS memory beyond its useful lifetime, increasing the window for exfiltration by a compromised script. |
| **Mitigation attendue dans le code** | Code review of the paste handler in the frontend. The clipboard value must be a transient local. No `$state`, `$derived`, or other reactive storage for clipboard content. |

---

### 2.7 Notification Badges

Activity notification badges are managed by the frontend
(`src/lib/state/notifications.svelte.ts` per ARCHITECTURE.md §10.2).
The `notification-changed` IPC event carries `paneId`, `type`, and `cleared`
fields — no user-generated content. The D-Bus notification path is separate:
`LinuxNotifications::notify(title, body)` is called by the `PtyReadTask` on
BEL in a non-active pane (ARCHITECTURE.md §7.4).

#### SEC-NOTIF-001

| Field | Value |
|-------|-------|
| **ID** | SEC-NOTIF-001 |
| **STRIDE** | Tampering |
| **FS requirement(s)** | FS-NOTIF-004, FS-VT-090 |
| **Threat** | The D-Bus desktop notification sent on bell (BEL 0x07) uses the pane's current tab title as the `title` parameter and a static string as the `body` parameter. If the tab title has been set via OSC 0 by a malicious process to a payload that exploits the notification daemon's markup rendering (e.g., GNOME's libnotify supports a subset of HTML markup in `body`), the notification body may render attacker-controlled HTML. |
| **Préconditions** | The notification daemon (e.g., GNOME Shell) renders HTML in the notification body. A process has set the tab title to a string containing notification markup (e.g., `<b><a href="javascript:...">click me</a></b>`). |
| **Action** | Set the tab title to `<b>alarm</b><a href="file:///etc/passwd">click</a>` via OSC 0. Trigger a BEL. Observe the desktop notification rendered by the system notification daemon. |
| **Résultat attendu (sécurisé)** | The `notify(title, body)` call passes the tab title sanitized by `parse_osc()` (C0/C1 stripped, 256-char truncation, bidi override stripped per SEC-BLK-017). Additionally, HTML markup must be stripped from the title before passing to `notify-rust`, since notification daemons may interpret markup in the body field. The `summary` field (used as title) is typically rendered as plain text by most daemons, but the `body` field may be markup-enabled. No application-generated content should be placed in `body` unless it is HTML-escaped. |
| **Risque si non mitigé** : Medium — attacker-controlled markup in a desktop notification. Impact depends on notification daemon capabilities; GNOME allows limited markup. Clickable links in notifications that open `file://` URIs is the primary risk. |
| **Mitigation attendue dans le code** | `src-tauri/src/platform/notifications_linux.rs` — when constructing the notification, HTML-escape the title string before passing it as `summary`. The `body` field should either be a static string or have all HTML entities escaped. Use `notify_rust::Notification::new().summary(&html_escape(title)).body("Bell event")`. Add `html_escape` dependency or implement a minimal `<`, `>`, `&`, `"` escaper. |

---

#### SEC-NOTIF-002

| Field | Value |
|-------|-------|
| **ID** | SEC-NOTIF-002 |
| **STRIDE** | Denial of Service |
| **FS requirement(s)** | FS-VT-090, FS-VT-092 |
| **Threat** | A malicious process emits a flood of BEL characters (0x07) at maximum PTY throughput (e.g., `yes $'\a' | head -100000`). Each BEL triggers `LinuxNotifications::notify()`, sending a D-Bus message to the notification daemon. At sufficient frequency, this saturates the D-Bus session bus, causes the notification daemon to queue thousands of notifications, and may crash or slow it — affecting all applications sharing the D-Bus session. |
| **Préconditions** | Bell type is configured to `System` (audible or desktop notification). No rate limiting is applied before calling `notify()`. |
| **Action** | Write 10 000 BEL characters to the PTY at maximum speed. Measure the number of D-Bus notification messages sent. |
| **Résultat attendu (sécurisé)** | BEL events are rate-limited to at most one `notify()` call per 100 ms per pane (FS-VT-092). The VT-layer rate limiter (`last_bell_emit: Instant`) prevents D-Bus flooding. A 10 000-BEL burst produces at most 10 notification calls per second, not 10 000. |
| **Risque si non mitigé** | High — D-Bus session bus saturation, notification daemon crash or hang, affecting all desktop applications for the duration. |
| **Mitigation attendue dans le code** | The BEL handler in `PtyReadTask` (or `VtProcessor`) must enforce the 100ms rate limit before calling `notify()`. The rate limit should be per-pane, not global, to avoid cross-pane interference. Unit test: feed 1000 BEL characters; assert `notify()` is called at most `elapsed_ms / 100` times. |

---

#### SEC-NOTIF-003

| Field | Value |
|-------|-------|
| **ID** | SEC-NOTIF-003 |
| **STRIDE** | Information Disclosure |
| **FS requirement(s)** | FS-NOTIF-001, FS-NOTIF-002 |
| **Threat** | The `notification-changed` IPC event carries `paneId` and `type` fields. A malicious script in the WebView (hypothetical XSS) listens to this event and uses the activity pattern — which panes are active, when they receive output or terminate — to infer user behavior (e.g., which pane is running a long computation, which pane has exited). This is a low-bandwidth side-channel. |
| **Préconditions** | XSS present in the WebView (prerequisite — mitigated by CSP per SEC-CSP-001). |
| **Action** | Register a listener for `notification-changed` events. Correlate event timing with user activity. |
| **Résultat attendu (sécurisé)** | The `notification-changed` event payload contains no content from the PTY (no screen text, no command output). It carries only `paneId`, `type` (activity/bell/terminated/cleared), and `cleared`. This is the minimum needed for the frontend to update badge state. The event must not include tab title, working directory, or any PTY-derived content. Code review: verify `NotificationChangedEvent` struct definition contains no PTY content fields. |
| **Risque si non mitigé** | Low — requires pre-existing XSS; the information disclosed (which pane is active) has low sensitivity. |
| **Mitigation attendue dans le code** | `src-tauri/src/events/types.rs` — review `NotificationChangedEvent` struct. Ensure no PTY-derived content fields are added in the future. Add a comment to the struct definition: `// No PTY content — pane ID and notification type only.` |

---

### 2.8 Pane Focus IPC

The `set_active_pane` command
(`src-tauri/src/commands/session_cmds.rs:79`) changes which pane has
keyboard focus and receives subsequent PTY input. It emits a
`session-state-changed` event to the frontend.

#### SEC-FOCUS-001

| Field | Value |
|-------|-------|
| **ID** | SEC-FOCUS-001 |
| **STRIDE** | Tampering / Elevation of Privilege |
| **FS requirement(s)** | FS-PANE-005 |
| **Threat** | A script injected into the WebView (hypothetical XSS) calls `invoke('set_active_pane', { paneId: <target_id> })` to steal keyboard focus from the user's current pane and redirect it to a different pane — e.g., one running an SSH session connected to an attacker's server. Subsequent keystrokes (including passwords) are sent to the attacker's pane. |
| **Préconditions** | XSS present in the WebView (prerequisite — mitigated by CSP per SEC-CSP-001). Multiple panes open. |
| **Action** | Call `set_active_pane` with the `pane_id` of a different pane. |
| **Résultat attendu (sécurisé)** | `set_active_pane` succeeds (this is a legitimate user action mediated by the frontend). The primary mitigation is the CSP that prevents XSS. A secondary mitigation is that `set_active_pane` only accepts valid `PaneId` values (unknown IDs return `INVALID_PANE_ID` per SEC-IPC-002) — it cannot create new panes or open connections. The frontend must also have a visible, non-spoofable active pane indicator (FS-PANE-006) so the user can immediately detect unexpected focus changes. |
| **Risque si non mitigé** | High — keystroke theft, including password entry, if XSS allows focus redirection. |
| **Mitigation attendue dans le code** | Primary: CSP (SEC-CSP-001). Secondary: `set_active_pane` validates `pane_id` existence (already implemented via `registry.set_active_pane`). Frontend: the active pane visual indicator (FS-PANE-006) must be clearly visible and not suppressible by in-pane content. |

---

#### SEC-FOCUS-002

| Field | Value |
|-------|-------|
| **ID** | SEC-FOCUS-002 |
| **STRIDE** | Information Disclosure |
| **FS requirement(s)** | FS-PANE-005 |
| **Threat** | The `session-state-changed` event emitted by `set_active_pane` includes the full `TabState` payload. This payload contains `paneId` values for all panes in the tab. A malicious script listening to this event learns the `pane_id` of every pane, enabling it to target specific panes for subsequent IPC attacks (e.g., `send_input` to a specific pane). |
| **Préconditions** | XSS present in the WebView (prerequisite). |
| **Action** | Register a listener for `session-state-changed`. Collect all `paneId` values from the payload. |
| **Résultat attendu (sécurisé)** | `PaneId` values are UUIDs generated at session creation time. They are not secret — the frontend already knows them (it received them from `create_tab` / `split_pane` responses). The primary mitigation is preventing XSS; the `TabState` payload does not disclose anything not already available to the frontend. No action required beyond CSP. |
| **Risque si non mitigé** | Low — `PaneId` values are already known to the frontend; this event does not disclose new information. |
| **Mitigation attendue dans le code** | Code review only. Confirm `TabState` does not include PTY output, credentials, or any sensitive fields not already available to the frontend. No structural change required. |

---

#### SEC-FOCUS-003

| Field | Value |
|-------|-------|
| **ID** | SEC-FOCUS-003 |
| **STRIDE** | Denial of Service |
| **FS requirement(s)** | FS-PANE-005 |
| **Threat** | `set_active_pane` is called at high frequency (e.g., 10 000 calls/second from a script) causing rapid successive `session-state-changed` event emissions to the frontend. The Tauri event system and the Svelte renderer are saturated, making the UI unresponsive. |
| **Préconditions** | XSS present in the WebView. |
| **Action** | Call `set_active_pane` in a tight loop via XSS. |
| **Résultat attendu (sécurisé)** | CSP prevents XSS (primary mitigation). Secondary: `set_active_pane` is an async Tauri command whose execution requires acquiring a mutex on the `SessionRegistry`. Rapid calls are serialized and bounded by the registry lock contention. The event emission rate is bounded by the registry throughput. No explicit rate limit is required beyond the inherent serialization. |
| **Risque si non mitigé** | Low-Medium — requires pre-existing XSS; the lock contention provides inherent throttling. |
| **Mitigation attendue dans le code** | No additional mitigation required beyond CSP and the existing registry lock. Document the inherent rate-bounding behavior. |

---

### 2.9 Credential Store SSH

Covered by SEC-CRED-001 through SEC-CRED-008, SEC-BLK-007, and SEC-BLK-008.
See companion documents.

Additional requirement for implementors:
- `LinuxCredentialStore` uses `connection_id` (UUID) as the only searchable
  attribute. Username and host are in the human-readable `label` field only,
  not in searchable attributes (SEC-BLK-008).
- `SecVec<u8>` must be used for all credential buffers; zeroing on drop is
  verified by the ARCHITECTURE.md §7.3 integration test.

---

### 2.10 OSC Title Update

Covered by SEC-PTY-006, SEC-BLK-015, SEC-BLK-016, SEC-BLK-017.
See companion documents.

Additional requirement for implementors:
- Strip HTML angle brackets (`<`, `>`) from tab titles in `parse_osc()` as
  defense-in-depth, since tab titles have no legitimate use for HTML markup
  (SEC-BLK-015).
- HTML-escape tab titles before passing to `notify-rust` `summary` field
  (SEC-NOTIF-001 above).

---

### 2.11 Focus Events Mode 1004

Covered by SEC-BLK-018, SEC-BLK-019, SEC-BLK-020.
See `docs/test-protocols/security-blocking-ipc-wiring.md`.

Key requirement for implementors:
- `ModeState::reset_all()` (or equivalent on session cleanup) must set
  `focus_events = false`. Unit test: `set focus_events = true`, call reset,
  assert `false`.

---

### 2.12 DECKPAM Keypad Mode

DECKPAM (`ESC =`) and DECKPNM (`ESC >`) toggle keypad application mode.
The mode state is tracked in `ModeState.deckpam`
(`src-tauri/src/vt/modes.rs:22`). When changed, a `mode-state-changed`
event (`{ paneId, decckm, deckpam }`) is emitted to the frontend, which
adjusts how it encodes numeric keypad input.

#### SEC-DECKPAM-001

| Field | Value |
|-------|-------|
| **ID** | SEC-DECKPAM-001 |
| **STRIDE** | Tampering |
| **FS requirement(s)** | FS-KBD-010 |
| **Threat** | A malicious process rapidly alternates between `ESC =` and `ESC >`, causing a high-frequency stream of `mode-state-changed` events to be emitted to the frontend. The Svelte renderer and the keyboard encoding logic are re-invoked on every event, saturating the event bus and causing the UI to drop user keystrokes. |
| **Préconditions** | No rate limit on `mode-state-changed` emissions. |
| **Action** | Write a loop that alternates `\x1b=\x1b>` at maximum PTY throughput. Measure frontend event rate and UI responsiveness. |
| **Résultat attendu (sécurisé)** | `mode-state-changed` events for DECKPAM are coalesced: if the mode value changes and then changes back within a short window (e.g., 16 ms — one animation frame), only the final state is emitted. Alternatively, the event is emitted at most once per PTY read burst, not once per sequence. No UI jank or dropped keystrokes. |
| **Risque si non mitigé** | Medium — UI DoS via mode toggle flood; user keystrokes may be lost or mis-encoded. |
| **Mitigation attendue dans le code** | The `mode-state-changed` event should be emitted once per `VtProcessor::process()` call (i.e., once per PTY read burst), not once per DECKPAM/DECKPNM sequence encountered. The VtProcessor tracks the final mode state after processing all sequences in the batch; the event is emitted with the final state at the end of `process()`. This is a general event batching principle, not specific to DECKPAM. |

---

#### SEC-DECKPAM-002

| Field | Value |
|-------|-------|
| **ID** | SEC-DECKPAM-002 |
| **STRIDE** | Tampering |
| **FS requirement(s)** | FS-KBD-010, FS-KBD-001 |
| **Threat** | A malicious process sets `deckpam = true` to switch the keypad to application mode. The frontend switches to encoding numeric keys as application sequences (e.g., `KP_0` → `ESC O p` instead of `0`). The user types a number expecting `0` to be sent to the shell. Instead, `ESC O p` is written to the PTY, which may be interpreted as a command by an application that does not expect application keypad sequences. If the user is entering a PIN or password, the encoded value differs from what they typed. |
| **Préconditions** | Keypad application mode is set by a subprocess without the user's awareness. The user then switches to a different context (a shell, a password prompt) in the same pane. |
| **Action** | Enable DECKPAM. Switch the foreground process. Type `1234` on the numeric keypad. Observe what is written to the PTY. |
| **Résultat attendu (sécurisé)** | This is an inherent PTY security model limitation: terminal mode is per-PTY, and any process controlling the PTY can set it. The mitigation is mode reset on foreground-process-group change (same as for mouse reporting and focus events). `ModeState::reset_all()` must set `deckpam = false` when the session's foreground process group changes, restoring numeric keypad mode for the new process. |
| **Risque si non mitigé** | Medium — keypad input mis-encoding after a malicious subprocess sets application mode; potential for incorrect input in subsequent commands or PIN entry. |
| **Mitigation attendue dans le code** | `src-tauri/src/vt/modes.rs` — `ModeState::reset_all()` must include `deckpam = false`. Emit a `mode-state-changed` event after the reset so the frontend reverts to numeric keypad encoding. Unit test: set `deckpam = true`, call `reset_all()`, assert `deckpam == false` and that the emitted event reflects the reset state. |

---

#### SEC-DECKPAM-003

| Field | Value |
|-------|-------|
| **ID** | SEC-DECKPAM-003 |
| **STRIDE** | Tampering |
| **FS requirement(s)** | FS-KBD-010, FS-KBD-001 |
| **Threat** | The `mode-state-changed` IPC event carries `{ paneId, decckm, deckpam }`. The frontend uses this event to update its local mode state replica and adjusts keyboard encoding. If the frontend applies a `mode-state-changed` event intended for a different pane to the currently active pane (pane ID mismatch), keystrokes sent to the active pane are encoded with the wrong mode. |
| **Préconditions** | Multiple panes open with different DECKPAM states. A `mode-state-changed` event arrives for pane B while pane A is active. |
| **Action** | Component test: simulate receiving a `mode-state-changed` event for `paneId = B` while the active pane is `A`. Assert that the active pane's keyboard encoding is not changed by the event. |
| **Résultat attendu (sécurisé)** | The frontend mode state is stored per-pane (keyed by `paneId`), not as a single global flag. The keyboard encoder reads the mode state for the currently active pane ID at keystroke time, not a shared mutable global. A `mode-state-changed` event for pane B updates only pane B's state entry; pane A's state is unaffected. |
| **Risque si non mitigé** | Medium — keyboard encoding errors if mode state is shared across panes; potential for application-mode sequences being sent to a process expecting numeric mode. |
| **Mitigation attendue dans le code** | Frontend `keyboard.ts` or equivalent: mode state must be a `Map<PaneId, ModeState>`, not two global booleans. The active pane's entry is read at keystroke encoding time. Component test: assert per-pane isolation of mode state. |

---

## 3. Validation Requirements Summary

The table below consolidates the validation and sanitization requirements
identified across all 12 items. This is an implementation checklist for
`rust-dev` and `frontend-dev`.

| # | Item | Requirement | File(s) | Severity |
|---|------|-------------|---------|----------|
| 1 | Scrollback search | `query.text.len() <= MAX_SEARCH_QUERY_LEN` (1024); use `regex` crate only | `input_cmds.rs:90`, `search.rs` | High |
| 2 | Scrollback search | `SearchMatch` struct must not include matched text | `search.rs:18` | High |
| 3 | SSH interactive auth | `HostKeyPromptEvent.host` from `SshConnectionConfig`, not server | `ssh/connection.rs` | Critical |
| 4 | SSH interactive auth | Auth-failure `SshStateChangedEvent.reason` — no credential material | `ssh/handler.rs` | Critical |
| 5 | SSH reconnect | Reconnect valid only for `Disconnected` panes | `ssh/manager.rs:574` | High |
| 6 | SSH reconnect | `SshConnection` struct has no `Credentials` field | `ssh/manager.rs` | Critical |
| 7 | SSH reconnect | Reconnect uses stored config only — no new host/port from IPC caller | `ssh_cmds.rs` (reconnect cmd) | High |
| 8 | Mouse reporting | SGR/URXVT coordinate clamping to screen dims | `vt/mouse.rs` | Medium |
| 9 | Mouse reporting | Mouse events only to active pane | Frontend event handler | Medium |
| 10 | Mouse reporting | Mode reset on session foreground-process-group change | `vt/modes.rs`, session cleanup | Medium |
| 11 | Bracketed paste | Strip `\x1b[201~` from payload before wrapping | Paste wrapping function | Critical |
| 12 | Bracketed paste | Strip `\x00` from paste payload | Paste wrapping function | Low |
| 13 | Bracketed paste | Multi-line confirmation dialog when `bracketed_paste == false` | Frontend paste handler | High |
| 14 | Ctrl+Shift+V | Multi-line confirmation dialog (same as #13) | Frontend paste handler | Critical |
| 15 | Ctrl+Shift+V | Paste preview in dialog must show control chars and newlines visibly | Frontend dialog component | High |
| 16 | Ctrl+Shift+V | Clipboard value scoped to local variable — no `$state` persistence | Frontend paste handler | Medium |
| 17 | Notifications | HTML-escape title before `notify-rust` `summary` field | `notifications_linux.rs:25` | Medium |
| 18 | Notifications | BEL rate limit (100ms/pane) before `notify()` call | `PtyReadTask` bell handler | High |
| 19 | Notifications | `NotificationChangedEvent` must not include PTY content fields | `events/types.rs` | Low |
| 20 | Pane focus | `set_active_pane` validates pane ID (already implemented) | `session_cmds.rs:79` | Medium |
| 21 | Pane focus | Active pane indicator non-spoofable from in-pane content | Frontend pane component | High |
| 22 | OSC title | Strip HTML angle brackets from titles in `parse_osc()` | `vt/osc.rs` | Critical |
| 23 | OSC title | OSC title bidi override stripping (U+202E etc.) | `vt/osc.rs` | Medium |
| 24 | OSC title | Rate-limit title change events (100ms/pane) | `VtProcessor` or emitter | Medium |
| 25 | Focus events | `ModeState::reset_all()` sets `focus_events = false` | `vt/modes.rs` | Medium |
| 26 | Focus events | Focus events only to active pane | Frontend focus handler | Low |
| 27 | DECKPAM | `mode-state-changed` emitted once per process batch, not per sequence | `VtProcessor::process()` | Medium |
| 28 | DECKPAM | `ModeState::reset_all()` sets `deckpam = false`; emits event | `vt/modes.rs`, session cleanup | Medium |
| 29 | DECKPAM | Frontend mode state is `Map<PaneId, ModeState>`, not global | `keyboard.ts` or equivalent | Medium |
| 30 | Credential store | `LinuxCredentialStore`: only `connection_id` as searchable attribute | `platform/credentials_linux.rs` | Medium |
