<!-- SPDX-License-Identifier: MPL-2.0 -->

# Security Test Protocol — Blocking IPC Wiring Items

Version: 1.0 — 2026-04-05
Scope: Linux (x86, x86_64, ARM32, ARM64, RISC-V) — v1 only
Author role: security-expert

Companion to: `docs/test-protocols/security-pty-ipc-ssh-credentials-csp-osc52.md`
This document covers security scenarios specific to the seven feature areas
listed below. Scenarios are numbered starting at SEC-BLK to avoid collision
with the parent protocol.

---

## Table of Contents

1. [Scrollback Search Security](#1-scrollback-search-security)
2. [SSH Interactive Auth Security](#2-ssh-interactive-auth-security)
3. [Credential Store — Hardening Gaps](#3-credential-store--hardening-gaps)
4. [Mouse Reporting Security](#4-mouse-reporting-security)
5. [Bracketed Paste Security](#5-bracketed-paste-security)
6. [OSC Title Update Security](#6-osc-title-update-security)
7. [Focus Events Mode 1004 Security](#7-focus-events-mode-1004-security)

---

## 1. Scrollback Search Security

The `search_pane` IPC command (`src-tauri/src/commands/input_cmds.rs:90`)
accepts a `SearchQuery` (`src-tauri/src/vt/search.rs:11`) that includes a
free `text: String` and a `regex: bool` flag. The search operates on
scrollback buffer content which may contain sensitive output (passwords
echoed by programs, private keys printed to stdout, environment dumps).

### SEC-BLK-001

**Vecteur d'attaque** : ReDoS via regex search query from the frontend.

**Préconditions** : `SearchQuery.regex` is `true`. The application accepts
the query string without length or complexity pre-validation.

**Action** : Call `search_pane` with a catastrophically backtracking regex
pattern such as `(a+)+b` applied to a scrollback buffer containing 50 000
lines of `a`-characters. The frontend sends this via `invoke('search_pane',
{ paneId, query: { text: '(a+)+b', caseSensitive: false, regex: true } })`.

**Résultat attendu (sécurisé)** : The search completes within 200 ms and
returns results (or an empty list). The Tokio runtime is not blocked. No
panic occurs. Note: the `regex` crate uses an NFA/DFA engine with linear
time guarantees, so catastrophic backtracking is impossible by construction.
This test confirms the correct crate is used and that no naive
`std::regex`-equivalent is introduced by a future refactor.

**Risque si non mitigé** : High — a blocked Tokio runtime stalls all IPC
commands for the duration of the regex evaluation.

**Mitigation attendue dans le code** : The `regex` crate (not `fancy-regex`
or PCRE) must be used for the search implementation. Verify in `Cargo.toml`
that `regex` is the dependency and `fancy-regex` is absent. Additionally,
the frontend must send `regex: true` only when the user explicitly enables
the regex toggle (FS-SEARCH-003).

---

### SEC-BLK-002

**Vecteur d'attaque** : Scrollback content exfiltration via search match
result payload exposed to the WebView.

**Préconditions** : The scrollback buffer contains sensitive output (e.g.,
a `cat /etc/shadow`-equivalent has been executed). The search feature is
enabled.

**Action** : A malicious script running in the WebView (hypothetical XSS
via a compromised dependency) calls `invoke('search_pane', { paneId,
query: { text: 'root:', caseSensitive: true, regex: false } })`. The
`SearchMatch` results return column positions. The script then calls
`get_pane_screen_snapshot` to extract the matching lines using those
coordinates.

**Résultat attendu (sécurisé)** : This attack is only possible if XSS is
already present in the WebView. The primary mitigation is the CSP
(`script-src 'self'` — SEC-CSP-001). Secondary mitigation: `search_pane`
returns only match positions (`scrollback_row`, `col_start`, `col_end`),
not the matched text itself. The matched content is not included in the
`SearchMatch` struct. Verify this invariant is preserved when the full
search implementation replaces the stub in `input_cmds.rs:99`.

**Risque si non mitigé** : High — if `SearchMatch` ever returns matched
text, scrollback content is retrievable via a single IPC call without
requiring the more expensive `get_pane_screen_snapshot`.

**Mitigation attendue dans le code** : `SearchMatch` struct
(`src-tauri/src/vt/search.rs:18`) must contain only position fields — no
`matched_text: String` field. Code review required when the stub is
replaced.

---

### SEC-BLK-003

**Vecteur d'attaque** : Denial of service via unbounded search query text
length.

**Préconditions** : The `search_pane` IPC command accepts `SearchQuery`
with a free `text: String` field.

**Action** : Call `search_pane` with `query.text` set to a 10 MB string.
The stub currently discards the query (`let _ = (inner, query)`), but when
the real search is implemented, a 10 MB pattern string is compiled into a
regex or scanned linearly against all scrollback rows.

**Résultat attendu (sécurisé)** : The command returns `Err(TauTermError)`
with code `QUERY_TOO_LONG` immediately, without compiling the regex or
scanning the buffer. Suggested maximum query length: 1024 characters
(consistent with typical terminal search UX).

**Risque si non mitigé** : Medium — CPU and memory exhaustion during regex
compilation or linear scan of a pathologically long pattern.

**Mitigation attendue dans le code** : `search_pane` validates
`query.text.len() <= MAX_SEARCH_QUERY_LEN` before passing to the search
engine. Implement at the top of `search_pane` in `input_cmds.rs:90`.

---

## 2. SSH Interactive Auth Security

These scenarios extend SEC-SSH-001 through SEC-SSH-006 and SEC-SSH-CH-001
through SEC-SSH-CH-010. They cover gaps specific to the interactive auth
flow and the TOFU prompt as newly wired IPC commands.

### SEC-BLK-004

**Vecteur d'attaque** : TOFU bypass via hostname spoofing — a malicious SSH
server presents a hostname string containing embedded control characters or
Unicode that renders as a look-alike of a trusted hostname in the TOFU
prompt UI.

**Préconditions** : The `HostKeyPromptEvent` includes a `host: String` field
that is displayed verbatim in the frontend TOFU confirmation dialog.

**Action** : Connect to an SSH server at `evil-server.example.com` which
presents itself with a `host` field containing the Unicode look-alike string
`аpple.com` (Cyrillic `а` U+0430 rather than ASCII `a`). The TOFU prompt
displays `аpple.com` to the user who mistakes it for `apple.com` and
accepts the key.

**Résultat attendu (sécurisé)** : The `host` value stored in `KnownHostsStore`
is the actual TCP connection host (from `SshConnectionConfig.host`), not a
string supplied by the remote server. The `HostKeyPromptEvent.host` field
is derived from the application's own connection config, not from any
server-supplied data. The frontend renders the hostname in a monospace font
with Punycode normalization for IDN hostnames, and the dialog clearly
displays the raw bytes / Punycode form alongside the display form.

**Risque si non mitigé** : Critical — a user accepts a MITM attacker's key
believing they are connecting to a legitimate host.

**Mitigation attendue dans le code** : In `src-tauri/src/ssh/` —
`check_server_key` (or its caller) must populate `HostKeyPromptEvent.host`
from `SshConnectionConfig.host` (the value the user configured), never from
data received over the network. Frontend: the TOFU dialog must display the
hostname in a way that makes look-alike attacks visible (monospace, full
display of percent-encoded characters).

---

### SEC-BLK-005

**Vecteur d'attaque** : Credential leakage in the `ssh-state-changed` event
payload when an auth failure occurs.

**Préconditions** : Password authentication is attempted. The `russh` error
returned on auth failure may include a diagnostic string.

**Action** : Connect to an SSH server with a wrong password. The auth
sequence fails. The `disconnected()` handler in `TauTermSshHandler` formats
the error and emits a `SshStateChangedEvent` with a `reason` field. If
`russh` includes the attempted credential in the error string (e.g., `auth
failed for user: alice with password: hunter2`), the password propagates
to the WebView via the event.

**Résultat attendu (sécurisé)** : The `reason` field in
`SshStateChangedEvent` contains only a generic diagnostic (e.g.,
`"Authentication failed"`) — no credential material. All `SshError`
variants that originate from auth failures are audited to ensure they do
not embed credential strings. This extends SEC-SSH-CH-010 specifically to
the auth failure path.

**Risque si non mitigé** : Critical — password disclosed to the WebView
renderer via an IPC event, visible in browser devtools and to any injected
script.

**Mitigation attendue dans le code** : Audit all match arms in
`src-tauri/src/ssh/handler.rs` (or equivalent) where auth errors are
converted to `SshStateChangedEvent.reason`. Ensure the conversion uses a
fixed format string with no interpolation of `Credentials` fields. Unit
test: trigger an auth failure, capture the emitted `SshStateChangedEvent`,
assert `reason` does not contain the password string.

---

### SEC-BLK-006

**Vecteur d'attaque** : Prompt injection via a malicious SSH banner
displayed before auth — a crafted server banner contains terminal escape
sequences that manipulate the TauTerm UI (e.g., clearing the TOFU dialog
from the screen, simulating a key press, or rewriting the host key
fingerprint display).

**Préconditions** : TauTerm displays the SSH server's pre-auth banner text
in the pane or in the TOFU dialog.

**Action** : The SSH server sends a banner message containing
`\x1b[2J\x1b[H` (clear screen + cursor home) followed by a crafted fake
TOFU confirmation UI drawn with escape sequences, making it appear that a
different host key is being confirmed.

**Résultat attendu (sécurisé)** : The TOFU confirmation dialog and host key
fingerprint are rendered by the Svelte frontend using structured data from
the `HostKeyPromptEvent` IPC event — not by writing the banner to the
PTY/screen buffer. If the banner is displayed in the pane, it goes through
`VtProcessor` and affects only the screen buffer, not the overlay dialog.
The overlay dialog is a DOM element rendered outside the terminal canvas,
immune to VT sequence injection.

**Risque si non mitigé** : High — user is tricked into accepting a spoofed
fingerprint display.

**Mitigation attendue dans le code** : TOFU and credential prompt dialogs
MUST be rendered as frontend overlay components driven by IPC events, never
as in-pane text generated from server-supplied data. Banner text, if
displayed, must pass through `VtProcessor` with the same sanitization as
all PTY output.

---

## 3. Credential Store — Hardening Gaps

These scenarios complement SEC-CRED-001 through SEC-CRED-008. They focus on
gaps that become relevant once the full SSH auth IPC wiring is operational.

### SEC-BLK-007

**Vecteur d'attaque** : Credential leakage into the `HostKeyPromptEvent`
IPC event — the pending TOFU entry in the in-process map accidentally
includes the `Credentials` struct alongside the key material.

**Préconditions** : The TOFU pending map stores entries keyed by `pane_id`.
If the auth task stores credentials in the same map entry as the pending
TOFU data (for convenience during reconnect), the credentials are held in
memory in a struct that may be serialized into events.

**Action** : Code review — inspect the TOFU pending map entry type. If it
contains a `Credentials` field, assert that this field is NOT serialized
into `HostKeyPromptEvent`. Also assert that the `Debug` representation of
the pending map entry does not expose the password.

**Résultat attendu (sécurisé)** : The TOFU pending map entry type contains
only `(host: String, key_type: String, key_bytes: Vec<u8>)` — no
`Credentials` field. Credentials are stored separately in the pending
credentials map (oneshot channel pattern, per SEC-SSH-CH-005). The two maps
are structurally separate.

**Risque si non mitigé** : Critical — if credentials are co-located with
TOFU pending data, they are exposed whenever TOFU data is logged or
serialized.

**Mitigation attendue dans le code** : Structural review of the SSH manager
state types in `src-tauri/src/ssh/manager.rs` (or equivalent). The TOFU
pending map and the credentials pending map must be distinct data structures
with no shared state.

---

### SEC-BLK-008

**Vecteur d'attaque** : Secret Service label disclosure — the attribute
label used to store a credential in the Secret Service keychain includes
sensitive metadata (username, host, port) that is readable by any process
running as the same user via `secret-tool search`.

**Préconditions** : `LinuxCredentialStore::store()` sets attributes on the
keychain entry to enable retrieval by `get()`. The attribute set typically
includes `service`, `username`, and `host`.

**Action** : After saving an SSH credential via TauTerm, run `secret-tool
search service tauterm` in a shell. Inspect the returned attributes. If
the attribute set includes the plaintext password as an attribute value
(rather than as the secret payload), it is disclosed by `secret-tool search`.

**Résultat attendu (sécurisé)** : The password is stored only as the secret
payload (the value returned by `secret-tool lookup`), never as an attribute.
Attributes contain only non-sensitive lookup keys: `service=tauterm`,
`connection_id=<uuid>`. No hostname, username, or password appears as an
attribute value that would be returned by `secret-tool search`.

**Risque si non mitigé** : Medium — metadata leakage to any local process
with access to the user's D-Bus session.

**Mitigation attendue dans le code** : `src-tauri/src/platform/credentials_linux.rs` —
review the `store()` call's attribute map. Ensure only `service` and an
opaque `connection_id` are set as attributes. Username and host go into the
`label` field (human-readable display only), not into searchable attributes.
Integration test: store a credential, run `secret-tool search service
tauterm`, assert no password value appears in the output.

---

## 4. Mouse Reporting Security

Mouse reporting (DECSET 1000/1002/1003/1006) causes TauTerm to encode user
mouse interactions and write them to the PTY. The `MouseEvent` struct
(`src-tauri/src/vt/mouse.rs:21`) is received from the frontend via IPC and
encoded into byte sequences injected into the PTY input buffer.

### SEC-BLK-009

**Vecteur d'attaque** : Coordinate overflow in mouse event encoding leads to
injection of unexpected bytes into the PTY input stream.

**Préconditions** : Mouse reporting is active (mode 1000 or 1003). The
frontend sends a `MouseEvent` with out-of-range `col` or `row` values.

**Action** : Invoke the mouse event IPC command (when implemented) with
`col = u32::MAX` and `row = u32::MAX`. In X10 encoding, coordinates are
clamped to 223 (`src-tauri/src/vt/mouse.rs:83`). In SGR encoding
(`encode_sgr`), the values are formatted directly into the sequence string
via `format!` without bounds checking. With `col = u32::MAX`, SGR produces
`\x1b[<cb;4294967295;4294967295M` — a valid but extreme coordinate that
applications must handle. Verify no integer overflow or panic occurs.

**Résultat attendu (sécurisé)** : X10 encoding: coordinates clamped to 223
(already implemented). SGR and URXVT encodings: coordinates clamped to a
reasonable maximum (suggested: the actual terminal dimensions, i.e., `col <=
screen_cols` and `row <= screen_rows`). No panic. No malformed byte sequence.
The frontend must also validate coordinates against the pane dimensions
before sending the IPC call.

**Risque si non mitigé** : Medium — an extreme coordinate value in SGR
encoding produces a long numeric string in the PTY input stream which,
while unlikely to cause injection, may confuse applications with narrow
integer parsing.

**Mitigation attendue dans le code** : `src-tauri/src/vt/mouse.rs` —
`encode_sgr` and `encode_urxvt` should clamp `self.col` and `self.row` to
the current screen dimensions. The IPC command receiving `MouseEvent` from
the frontend must validate that coordinates are within `[1, screen_cols]`
and `[1, screen_rows]` and reject out-of-range values.

---

### SEC-BLK-010

**Vecteur d'attaque** : Mouse tracking exfiltration — an application enables
mode 1003 (any-event tracking) and uses the resulting PTY input stream of
mouse coordinates to reconstruct the user's screen activity (e.g., which
menu items they hover, which text they select in a password manager running
in another pane).

**Préconditions** : Mode 1003 is enabled by a process in one pane. Mouse
events in TauTerm are global (not pane-scoped).

**Action** : A malicious application in pane A enables mode 1003. The user
interacts with pane B (e.g., clicking in a password manager's output area).
TauTerm sends mouse events to pane A's PTY, disclosing the coordinates of
the user's interactions in pane B.

**Résultat attendu (sécurisé)** : Mouse events are only sent to the PTY of
the currently active pane. Mouse events generated by the user interacting
with an inactive pane (or a non-pane area of the UI) are not forwarded to
any application. Mode 1003 does not grant cross-pane surveillance. This is
a design-level constraint that must be verified at the architecture review.

**Risque si non mitigé** : Medium — coordinate-level side-channel between
panes, revealing gross user attention patterns to a malicious pane
application.

**Mitigation attendue dans le code** : The frontend must only forward mouse
events to the PTY of the active pane. Events originating from outside the
active pane's WebView area must not be routed to any PTY. Architecture
review: confirm mouse event dispatch is pane-scoped in the frontend event
handler.

---

### SEC-BLK-011

**Vecteur d'attaque** : Mouse mode left active after application exit or
pane session termination — a subsequent interactive shell inherits mouse
reporting mode and receives all mouse coordinates typed by the user.

**Préconditions** : A process enables mouse reporting mode 1003 and then
exits without resetting it (e.g., via SIGKILL). The shell that replaces it
in the same pane now receives raw mouse escape sequences as if they were
keyboard input.

**Action** : Run a mouse-aware application in a pane. Kill it with SIGKILL
(bypassing its exit handler). Click in the terminal. Observe whether raw
mouse sequences appear in the shell prompt.

**Résultat attendu (sécurisé)** : TauTerm resets all mouse reporting modes
(1000, 1002, 1003, 1006) when a pane's foreground process group changes
(i.e., when a new process takes the controlling terminal) or when a session
ends. FS-VT-086 requires reset on application exit. Code review: verify
`VtTerminalModes::reset_all()` is called at session cleanup.

**Risque si non mitigé** : Medium — raw escape sequences injected into the
shell prompt may confuse the shell or produce accidental command execution
if the sequences happen to match valid characters.

**Mitigation attendue dans le code** : `src-tauri/src/vt/modes.rs` —
`ModeState` (or equivalent) must be fully reset when the session's
controlling process exits. Verify in the PTY session lifecycle (`src-tauri/
src/session/`).

---

## 5. Bracketed Paste Security

FS-CLIP-008 (`src-tauri/src/vt/modes.rs` — bracketed paste mode 2004)
requires that pasted text be wrapped with `ESC[200~` / `ESC[201~` and that
any embedded `ESC[201~` sequences be stripped from the payload before
wrapping.

### SEC-BLK-012

**Vecteur d'attaque** : Bracketed paste escape sequence injection — pasted
content contains a crafted `ESC[201~` (the paste-end marker) followed by
arbitrary commands. If the end marker is not stripped from the payload, the
receiving application terminates the bracketed paste early and interprets
the remainder as typed commands.

**Préconditions** : Bracketed paste mode (DECSET 2004) is active. The user
pastes text that contains `\x1b[201~` embedded in the middle.

**Action** : With zsh or bash in bracketed paste mode, paste the string
`safe_text\x1b[201~\nrm -rf ~`. Without stripping, the application receives
`\x1b[200~safe_text` as the bracketed paste content and then `rm -rf ~` as
a separate command. With stripping, the application receives
`\x1b[200~safe_text\nrm -rf ~\x1b[201~` — the entire string as paste
content, preventing execution.

**Résultat attendu (sécurisé)** : All occurrences of `\x1b[201~` within the
pasted payload are stripped before wrapping. The test must verify this at
the Rust level (unit test on the paste wrapping function) and at the
integration level (paste into a running zsh with bracketed paste, verify
the malicious command is not executed).

**Risque si non mitigé** : Critical — arbitrary command injection via
clipboard paste, exploitable by any content on the clipboard (e.g., a web
page that uses invisible Unicode to embed the escape sequence).

**Mitigation attendue dans le code** : The paste wrapping function (to be
implemented in `src-tauri/src/` or `src/lib/`) must call
`.replace("\x1b[201~", "")` on the payload before prepending `\x1b[200~`
and appending `\x1b[201~`. This is mandated by FS-CLIP-008. Unit test:
assert that `wrap_bracketed("text\x1b[201~more")` produces
`"\x1b[200~textmore\x1b[201~"`.

---

### SEC-BLK-013

**Vecteur d'attaque** : Bracketed paste mode bypass via crafted SGR/CSI
sequence — an application toggles DECRST 2004 (disable bracketed paste) by
injecting the reset sequence into the PTY output stream via a crafted output
payload, then exploits the unbracketed state to inject commands via the
user's subsequent paste.

**Préconditions** : Bracketed paste is enabled by the shell. A malicious
process running in the pane can write to stdout (PTY slave).

**Action** : The malicious process outputs `\x1b[?2004l` (DECRST 2004 —
disable bracketed paste). TauTerm processes this via `VtProcessor` and
updates its mode state to `bracketed_paste = false`. The user then pastes
a multi-line command. Without bracketed paste, the paste executes
immediately without the shell's paste-safety filter.

**Résultat attendu (sécurisé)** : This is an inherent limitation of the
PTY security model — an application running in the PTY has legitimate
control over terminal modes. TauTerm cannot distinguish a legitimate
application disabling bracketed paste from a malicious one. The mitigation
is the multi-line paste confirmation dialog (FS-CLIP-009): when bracketed
paste is NOT active and the pasted text contains newlines, TauTerm MUST
display a confirmation dialog. This provides a safety net even when
bracketed paste is disabled.

**Risque si non mitigé** : High — silent multi-line command execution via
paste when bracketed paste is disabled.

**Mitigation attendue dans le code** : `src-tauri/src/commands/input_cmds.rs`
(or the paste handler) — before writing pasted text to the PTY, check
`VtTerminalModes.bracketed_paste`. If `false` and `payload` contains `\n`
or `\r`, emit a confirmation event to the frontend. Only write to PTY after
frontend confirmation. Implement the confirmation dialog in the frontend
paste handler.

---

### SEC-BLK-014

**Vecteur d'attaque** : Null byte injection via bracketed paste — the pasted
content contains null bytes (`\x00`) which are passed through the bracketed
paste wrapper and written to the PTY. Some shells and programs treat null
bytes as string terminators, potentially truncating commands or causing
unexpected behavior.

**Préconditions** : Bracketed paste mode is active. The clipboard contains
content with embedded null bytes (possible with binary clipboard content).

**Action** : Paste content containing `\x00` characters. Observe whether
null bytes are passed through to the PTY write call.

**Résultat attendu (sécurisé)** : Null bytes in paste content are stripped
or replaced with a safe character before the payload is written to the PTY.
This prevents null-byte injection into shell command lines. The behavior
should be documented as a paste sanitization policy.

**Risque si non mitigé** : Low — null bytes in most shells are ignored or
cause truncation rather than exploitation, but the behavior is
implementation-defined and should be deterministic.

**Mitigation attendue dans le code** : The paste wrapping function strips
`\x00` from the payload in addition to stripping `\x1b[201~`. Unit test:
assert `wrap_bracketed("foo\x00bar")` produces `"\x1b[200~foobar\x1b[201~"`.

---

## 6. OSC Title Update Security

OSC 0/1/2 sequences set the tab title. The title sanitization is specified
in FS-VT-062 and partially covered by SEC-PTY-006. The scenarios below
address gaps in the rendering pipeline between backend sanitization and
frontend display.

### SEC-BLK-015

**Vecteur d'attaque** : XSS via OSC title in the frontend — the sanitized
title string is passed from the backend in a `ScreenUpdateEvent` and
rendered in the TabBar component. If any component uses `{@html title}`
instead of `{title}`, a title containing `<img src=x onerror=alert(1)>`
(which passes the C0/C1 filter since it contains no control characters)
executes as HTML.

**Préconditions** : `VtProcessor::parse_osc()` strips C0/C1 characters and
truncates to 256 characters, but does not strip HTML tags (HTML is not a
control character concern at the VT level). The sanitized title reaches the
frontend via IPC event.

**Action** : Feed `\x1b]0;<img src=x onerror=alert(1)>\x07` to
`VtProcessor`. The `SetTitle` action is returned with the payload
`<img src=x onerror=alert(1)>` (no C0/C1 characters, passes the current
filter). This string reaches the frontend and is rendered in the TabBar.

**Résultat attendu (sécurisé)** : The TabBar and all other frontend
components that render the tab title use Svelte's `{title}` text
interpolation (not `{@html title}`), which escapes HTML entities. The
rendered DOM contains `&lt;img src=x onerror=alert(1)&gt;` as text, not an
`<img>` element. No script executes.

**Risque si non mitigé** : Critical — XSS in the WebView, enabling arbitrary
Tauri IPC invocation from the renderer context.

**Mitigation attendue dans le code** : Static analysis — grep all `.svelte`
files for `{@html` and any variable that originates from `tabTitle`,
`paneTitle`, or any field from `ScreenUpdateEvent`. All must use text
interpolation. Additionally, the backend `parse_osc()` should strip HTML
angle brackets as a defense-in-depth measure, since tab titles have no
legitimate use for HTML markup.

---

### SEC-BLK-016

**Vecteur d'attaque** : OSC title rate-limiting bypass — a malicious process
emits thousands of OSC 0 title-change sequences per second, causing the
frontend to re-render the TabBar on every frame and consuming excessive CPU
in both the Rust event emitter and the Svelte renderer.

**Préconditions** : No rate limit is applied to OSC 0/1/2 title updates.

**Action** : Write a loop that outputs `\x1b]0;title_N\x07` at maximum PTY
throughput (e.g., 100 000 iterations). Measure frontend CPU usage during
this event.

**Résultat attendu (sécurisé)** : OSC title updates are rate-limited to at
most one title change event emitted to the frontend per 100 ms per pane.
Intermediate title values are discarded; only the most recent title in each
100 ms window is emitted. This is consistent with the bell rate limit
(FS-VT-092). The open recommendation M3 in `docs/FS.md` addresses this.

**Risque si non mitigé** : Medium — UI thread saturation; frontend becomes
unresponsive. This is a DoS vector against the UI, not a data exfiltration
risk.

**Mitigation attendue dans le code** : Add a rate limiter to the OSC title
dispatch path in `VtProcessor` or in the Tauri event emitter. Suggested
implementation: track `last_title_emit: Instant` per pane and suppress
emission if `elapsed() < Duration::from_millis(100)`.

---

### SEC-BLK-017

**Vecteur d'attaque** : OSC title contains a bidirectional text override
sequence (Unicode RLO U+202E) that causes the rendered tab label to display
in reverse order, making `malicious.exe` appear as `exe.suoicilam` or
constructing a spoofed filename.

**Préconditions** : The tab title sanitization strips C0/C1 control
characters but does not filter Unicode bidirectional control characters
(U+200E, U+200F, U+202A–U+202E, U+2066–U+2069, U+061C).

**Action** : Feed `\x1b]0;\u202Egnp.exe\x07` to `VtProcessor`. The
resulting title, if rendered in a browser/WebKit with bidi enabled, displays
as `exe.png` (right-to-left override reversal).

**Résultat attendu (sécurisé)** : The `parse_osc()` title sanitization
strips Unicode bidirectional control characters in addition to C0/C1
characters. Unit test: feed a title containing U+202E and assert the
resulting `SetTitle` action contains the cleaned string without the
bidirectional override.

**Risque si non mitigé** : Medium — spoofed tab titles mislead users about
which process is running in a pane, potentially masking a privilege
escalation or social engineering scenario.

**Mitigation attendue dans le code** : `src-tauri/src/vt/osc.rs` —
`parse_osc()` title filter must reject or strip Unicode bidi control
characters: U+200E, U+200F, U+202A, U+202B, U+202C, U+202D, U+202E,
U+2066, U+2067, U+2068, U+2069, U+061C. Add these to the existing C0/C1
filter.

---

## 7. Focus Events Mode 1004 Security

Focus events (DECSET 1004) cause TauTerm to write `\x1b[I` (focus in) and
`\x1b[O` (focus out) to the PTY when the terminal window gains or loses
input focus. This is a low-bandwidth but reliable side-channel.

### SEC-BLK-018

**Vecteur d'attaque** : Focus event fingerprinting — a malicious process
enables mode 1004 and uses the timing of `\x1b[I` / `\x1b[O` sequences
to infer user behavior patterns (e.g., when the user switches away from
TauTerm, when they return, how long sessions last). This information is
exfiltrated via the PTY output channel (e.g., encoded in shell commands
sent to a remote server in an SSH session).

**Préconditions** : Mode 1004 is enabled. The pane is connected to an SSH
server controlled by an attacker.

**Action** : The malicious remote shell script reads focus events from its
stdin and timestamps them, building a usage fingerprint. The fingerprint is
exfiltrated to the attacker's C2 server.

**Résultat attendu (sécurisé)** : This is a legitimate use of the DECSET
1004 protocol — focus events are an intentional feature. The mitigation is
informational: focus events should only be sent to the active pane (the
pane that actually has keyboard focus), not to all panes. A background pane
receiving focus events would be purely a side-channel with no legitimate
use case. Verify that focus events are scoped to the active pane only.

**Risque si non mitigé** : Low — timing-only side-channel. No credential or
content disclosure.

**Mitigation attendue dans le code** : The frontend must only send focus
events (window focus in/out) to the currently active pane's PTY. When the
window gains or loses focus, only one `invoke('send_input', ...)` call is
made — to the active pane. Inactive panes do not receive focus events even
if they have mode 1004 enabled. Architecture review required.

---

### SEC-BLK-019

**Vecteur d'attaque** : Focus event injection — a process in a pane writes
`\x1b[?1004h` (enable focus events) and then immediately writes crafted
input simulating focus events to a co-resident process via PTY output
manipulation. This is not directly possible via the PTY model (a process
cannot write to its own stdin via stdout), but it becomes relevant if
TauTerm ever processes focus event responses differently from regular input.

**Préconditions** : Mode 1004 is enabled. The process expects
`\x1b[I` / `\x1b[O` sequences on its stdin as authentic focus change
notifications.

**Action** : Code review — verify that focus event injection via PTY output
is impossible. The `\x1b[I` and `\x1b[O` sequences in PTY output should be
treated as regular input to the receiving process (they are not TauTerm's
own focus notifications echoed back). TauTerm generates focus events
exclusively in response to OS-level window focus changes, not in response to
VT sequences in PTY output.

**Résultat attendu (sécurisé)** : TauTerm's focus event generation is driven
exclusively by the WebView's `focus` and `blur` DOM events (or equivalent
OS-level signals), not by any VT sequence received from the PTY. A process
that outputs `\x1b[I` into the terminal does not trigger TauTerm to write
another `\x1b[I` to the PTY input. This is structurally guaranteed by the
separation between PTY output processing (VtProcessor) and PTY input
generation (focus event handler).

**Risque si non mitigé** : Low — the injection path does not exist in the
current architecture, but the scenario is worth documenting for future
reference if the event pipeline is modified.

**Mitigation attendue dans le code** : Code review of the focus event
dispatch path. Confirm there is no feedback loop between `VtProcessor`
parsing focus-related sequences in output and the focus event input
generator. No test required beyond code review.

---

### SEC-BLK-020

**Vecteur d'attaque** : Focus mode left active after session termination —
analogous to SEC-BLK-011 for mouse reporting. If DECSET 1004 is not reset
when the foreground process exits, a subsequent shell receives
`\x1b[I` / `\x1b[O` as raw input on focus changes, potentially confusing
readline or other line-editing libraries.

**Préconditions** : A process enables DECSET 1004 and exits via SIGKILL.
The shell that replaces it inherits the pane's mode state.

**Action** : Enable mode 1004 in a test process. Kill it with SIGKILL.
Switch away from the TauTerm window and back. Observe whether `\x1b[I`
appears in the shell prompt.

**Résultat attendu (sécurisé)** : All DECSET modes (including 1004) are
reset when the pane's foreground process group changes or when the session
ends. FS-VT-086 covers mouse modes; the same reset policy must apply to
focus events. Verify that `ModeState::reset_all()` (or equivalent in
`src-tauri/src/vt/modes.rs`) includes `focus_events = false`.

**Risque si non mitigé** : Low-Medium — unexpected escape sequences in
shell input may cause readline history corruption or accidental key
binding triggers, but not command injection.

**Mitigation attendue dans le code** : `src-tauri/src/vt/modes.rs` —
`ModeState` reset on session foreground-process-group change. Unit test:
set `focus_events = true`, call `reset_all()`, assert `focus_events == false`.
