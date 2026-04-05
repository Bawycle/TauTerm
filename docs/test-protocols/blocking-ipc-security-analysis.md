<!-- SPDX-License-Identifier: MPL-2.0 -->

# Security Analysis — Blocking and Major IPC Items

Version: 1.0 — 2026-04-05
Scope: Linux (x86, x86_64, ARM32, ARM64, RISC-V) — v1 only
Author role: security-expert

---

## Purpose

This document provides the security analysis for the 12 blocking/major IPC
items listed in the implementation plan. It consolidates findings from two
earlier security protocols and adds new scenarios for angles not yet
documented. It is the canonical security sign-off checklist for Phase 1.

## Companion documents (pre-existing — do not duplicate)

| File | Scenarios |
|------|-----------|
| `security-pty-ipc-ssh-credentials-csp-osc52.md` | SEC-PTY-*, SEC-IPC-*, SEC-SSH-*, SEC-CRED-*, SEC-CSP-*, SEC-OSC-*, SEC-PATH-*, SEC-UI-*, SEC-SSH-CH-* |
| `security-blocking-ipc-wiring.md` | SEC-BLK-001–020 (scrollback search, SSH auth, credential store, mouse reporting, bracketed paste, OSC title, focus events 1004) |
| `security-blocking-major-ipc-items.md` | SEC-RECON-*, SEC-PASTE-*, SEC-NOTIF-*, SEC-FOCUS-*, SEC-DECKPAM-* (SSH reconnect, paste, notifications, pane focus, DECKPAM) |

Scenarios in those documents are referenced below by ID and not reproduced.

---

## Table of Contents

1. [Item 1 — Scrollback Search](#item-1--scrollback-search)
2. [Item 2 — SSH Auth Interactive](#item-2--ssh-auth-interactive)
3. [Item 3 — SSH Reconnection UI](#item-3--ssh-reconnection-ui)
4. [Item 4 — Mouse Reporting](#item-4--mouse-reporting)
5. [Item 5 — Bracketed Paste](#item-5--bracketed-paste)
6. [Item 6 — Ctrl+Shift+V Paste](#item-6--ctrlshiftv-paste)
7. [Item 7 — Tab Activity Notifications](#item-7--tab-activity-notifications)
8. [Item 8 — Pane Focus → set_active_pane](#item-8--pane-focus--set_active_pane)
9. [Item 9 — Credential Store SSH (CredentialManager not injected)](#item-9--credential-store-ssh)
10. [Item 10 — OSC Title Update](#item-10--osc-title-update)
11. [Item 11 — Focus Events Mode 1004](#item-11--focus-events-mode-1004)
12. [Item 12 — DECKPAM](#item-12--deckpam)
13. [Implementation Sign-off Checklist](#implementation-sign-off-checklist)

---

## Item 1 — Scrollback Search

**FS requirements:** FS-SEARCH-001 to FS-SEARCH-007
**Implementation gap:** `search_pane` is a stub returning `Ok(Vec::new())`.
**Entry point:** `src-tauri/src/commands/input_cmds.rs:90`

### Threat analysis

The `SearchQuery` struct (`src-tauri/src/vt/search.rs:11`) carries a free
`text: String` and a `regex: bool` flag. When the stub is replaced:

- `text` sourced from WebView user input: unbounded length, arbitrary content.
- `regex: true` compiles the string as a regex: any regex engine is a
  potential DoS surface.
- Search results (`SearchMatch`) expose position metadata for scrollback
  content that may include passwords or private key material.

### Security requirements

1. Validate `query.text.len() <= 1024` before any processing; return
   `QUERY_TOO_LONG` error.
2. Use the `regex` crate (linear-time NFA/DFA — no backtracking). Never
   introduce `fancy-regex` or PCRE.
3. `SearchMatch` struct must contain only position fields: no matched text.
4. Alternate screen content must not be searchable (FS-SEARCH-004).

### Security test scenarios

**SEC-BLK-001** — ReDoS via regex search pattern: see
`security-blocking-ipc-wiring.md`.

**SEC-BLK-002** — Scrollback exfiltration via `SearchMatch` payload: see
`security-blocking-ipc-wiring.md`.

**SEC-BLK-003** — DoS via oversized `query.text`: see
`security-blocking-ipc-wiring.md`.

**SEC-SEARCH-001** (new)

| Field | Value |
|-------|-------|
| **ID** | SEC-SEARCH-001 |
| **STRIDE** | Information Disclosure |
| **FS requirement(s)** | FS-SEARCH-004 |
| **Threat** | With the alternate screen active (e.g., vim open), a script calls `search_pane` to search the alternate screen buffer. If the VtProcessor does not gate the search to the normal screen, the alternate screen content (e.g., a password entry form drawn by vim) is searchable. The attacker learns which lines contain matches on the alternate screen. |
| **Préconditions** | Alternate screen is active. `search_pane` is implemented (not a stub). |
| **Action** | Send `\x1b[?1049h` (enter alternate screen). Draw content containing a known string. Call `search_pane` with that string. |
| **Résultat attendu (sécurisé)** | `search_pane` returns an empty result set when the alternate screen is active. The search function queries only the normal screen scrollback, never the alternate screen buffer. |
| **Risque si non mitigé** | High — alternate screen content (password dialogs, sensitive forms) is discoverable via search. |
| **Mitigation attendue dans le code** | `VtProcessor::search()` (or `ScreenBuffer::search()`) checks `is_alternate_screen_active()` and returns empty results if true. |

---

## Item 2 — SSH Auth Interactive

**FS requirements:** FS-SSH-010 to FS-SSH-014
**Implementation gap:** Frontend has no listeners for `ssh-state-changed`,
`credential-prompt`, or `host-key-prompt` IPC events.
**Entry points:** Frontend event registration; backend already emits these
events from `src-tauri/src/ssh/`.

### Threat analysis

The implementation gap is purely frontend: the backend emits the events
correctly. The security risk surfaces when the frontend is wired:

- `credential-prompt` listener renders a password input. If the password
  field value is stored incorrectly or logged, it leaks.
- `host-key-prompt` listener displays a TOFU confirmation dialog. If the
  dialog is in-pane (not a DOM overlay), it is spoofable via PTY output.
- `ssh-state-changed` listener updates UI state. If state transitions are
  not validated, a rogue event could trigger incorrect UI behavior.

### Security requirements

1. `credential-prompt` dialog: password field value in a component-local
   `let` — never in a `$state` rune at module level.
2. TOFU dialog: DOM overlay above terminal canvas, not in-pane content.
3. `host-key-prompt.host` displayed verbatim — must originate from
   `SshConnectionConfig.host`, not server data (SEC-BLK-004).
4. `ssh-state-changed.reason` field rendered as plain text, not `{@html}`.
5. Frontend must not call `accept_host_key` or `provide_credentials`
   without explicit user interaction — no auto-accept on any condition.

### Security test scenarios

**SEC-BLK-004** — TOFU bypass via Unicode look-alike hostname: see
`security-blocking-ipc-wiring.md`.

**SEC-BLK-005** — Credential leakage in `ssh-state-changed` reason: see
`security-blocking-ipc-wiring.md`.

**SEC-BLK-006** — Prompt injection via SSH server banner: see
`security-blocking-ipc-wiring.md`.

**SEC-SSH-CH-001 through SEC-SSH-CH-010** — TOFU pending map, `accept_host_key`
/ `provide_credentials` command security: see
`security-pty-ipc-ssh-credentials-csp-osc52.md §2.9`.

**SEC-UI-001** — Hostname XSS via ConnectionManager: see
`security-pty-ipc-ssh-credentials-csp-osc52.md §2.8`.

**SEC-UI-002** — Password field persistence in frontend state: see
`security-pty-ipc-ssh-credentials-csp-osc52.md §2.8`.

**SEC-SSHAUTH-001** (new — frontend auto-accept prevention)

| Field | Value |
|-------|-------|
| **ID** | SEC-SSHAUTH-001 |
| **STRIDE** | Spoofing / Elevation of Privilege |
| **FS requirement(s)** | FS-SSH-011 |
| **Threat** | The frontend `host-key-prompt` event listener contains a bug or is written with a default-accept path (e.g., `if (event.isKnown) { invoke('accept_host_key') }` without user interaction). All TOFU prompts are silently accepted, enabling MITM on all connections. |
| **Préconditions** | Frontend listener for `host-key-prompt` is wired. |
| **Action** | Code review — verify that `invoke('accept_host_key', ...)` is called exclusively from a button `onclick` handler. It must never be called from the event listener's body, from `$effect`, or from any auto-running reactive context. |
| **Résultat attendu (sécurisé)** | `accept_host_key` is called only when the user explicitly clicks "Accept" in the TOFU dialog. The `host-key-prompt` event listener only shows the dialog — it does not call any IPC command automatically. |
| **Risque si non mitigé** | Critical — silent MITM acceptance on every connection. |
| **Mitigation attendue dans le code** | Code review of the `host-key-prompt` listener. No IPC call in the listener body itself. The dialog component's Accept button calls `invoke('accept_host_key', { paneId })` on click. The Reject button calls `invoke('reject_host_key', { paneId })`. |

---

## Item 3 — SSH Reconnection UI

**FS requirements:** FS-SSH-040 to FS-SSH-042
**Implementation gap:** `reconnect_ssh` IPC command exists; no frontend
reconnect UI.
**Entry point:** `src-tauri/src/commands/ssh_cmds.rs` (reconnect_ssh),
`src-tauri/src/ssh/manager.rs:574`.

### Threat analysis

The reconnect button is rendered by a frontend overlay component
(`DisconnectBanner.svelte`). The risk is overlay spoofing (an in-pane fake
button) and incorrect state-gating on the IPC command.

### Security requirements

1. Reconnect only valid in `Disconnected` state — backend guard required.
2. `SshConnection` must not store `Credentials` between disconnect and
   reconnect.
3. Reconnect overlay must be a DOM element above the terminal canvas
   (`z-index` higher than the canvas; `pointer-events: none` on canvas
   when overlay is visible).

### Security test scenarios

**SEC-RECON-001 through SEC-RECON-004**: see
`security-blocking-major-ipc-items.md §2.3`.

---

## Item 4 — Mouse Reporting

**FS requirements:** FS-VT-080 to FS-VT-086
**Implementation gap:** X10/Normal/Button-event/Any-event mode mouse events
are not encoded to PTY. The `MouseEvent` struct exists
(`src-tauri/src/vt/mouse.rs`) but no frontend→backend IPC path sends these.

### Threat analysis

When the IPC path is wired, mouse events from the WebView carry `col`,
`row`, `button`, and modifier fields. The Rust encoder
(`MouseEvent::encode`) writes directly to the PTY input stream. Malformed
input from the WebView could inject unexpected bytes into the PTY.

Additionally, mode 1003 (any-event) enables cross-pane surveillance, and
leaving mouse modes active after process exit injects raw sequences into the
successor shell.

### Security requirements

1. Frontend must validate `col` and `row` are within `[1, screen_cols]` and
   `[1, screen_rows]` before sending the IPC call.
2. Backend encodes SGR/URXVT with clamped coordinates.
3. Mouse events sent only to active pane's PTY.
4. All mouse modes reset on session foreground-process-group change.

### Security test scenarios

**SEC-BLK-009 through SEC-BLK-011**: see
`security-blocking-ipc-wiring.md §4`.

---

## Item 5 — Bracketed Paste

**FS requirements:** FS-CLIP-008
**Implementation gap:** `ModeState.bracketed_paste` is tracked in Rust but
the frontend paste handler does not yet check it or apply wrapping.

### Threat analysis

The missing wrapping means all pasted content is currently injected raw into
the PTY — the application's bracketed paste safety filter is never invoked.
Once wired, the new risk is the `ESC[201~` injection vector.

### Security requirements

1. Strip `\x1b[201~` from payload before wrapping — non-negotiable (Critical).
2. Strip `\x00` null bytes from payload.
3. When `bracketed_paste == false` and payload contains `\n`/`\r`, show
   confirmation dialog (FS-CLIP-009).

### Security test scenarios

**SEC-BLK-012 through SEC-BLK-014**: see
`security-blocking-ipc-wiring.md §5`.

---

## Item 6 — Ctrl+Shift+V Paste

**FS requirements:** FS-CLIP-005, FS-KBD-003
**Implementation gap:** Ctrl+Shift+V is not intercepted in
`handleGlobalKeydown` in the frontend.

### Threat analysis

This is the same paste pipeline as item 5. The unique risk for
Ctrl+Shift+V is that clipboard content originates from an external
application, which the user may not have fully inspected. The multi-line
confirmation dialog is the primary safety net.

An additional risk: the clipboard value must not persist in reactive
frontend state beyond the paste action.

### Security requirements

1. Multi-line confirmation dialog mandatory when `bracketed_paste == false`
   and clipboard contains `\n`/`\r`.
2. Confirmation dialog preview must render control characters visibly.
3. Clipboard value scoped to a local `let` in the paste handler — no
   `$state` or `$derived` persistence.
4. The bracketed paste `ESC[201~` stripping (item 5) applies here too.

### Security test scenarios

**SEC-PASTE-001 through SEC-PASTE-003**: see
`security-blocking-major-ipc-items.md §2.6`.

---

## Item 7 — Tab Activity Notifications

**FS requirements:** FS-NOTIF-001 to FS-NOTIF-004
**Implementation gap:** Backend emits `notification-changed` events;
frontend ignores them. The bell (BEL 0x07) path to `LinuxNotifications`
exists but bell rate limiting and D-Bus title injection have not been
hardened.
**Entry point:** `src-tauri/src/platform/notifications_linux.rs:22`

### Threat analysis

Two distinct attack surfaces:

**A. In-app badge notifications (`notification-changed` event):**
The payload carries `paneId` and notification type — no PTY content. Low
attack surface. The risk is that a future extension adds PTY content to the
event payload.

**B. Desktop notification via D-Bus (`LinuxNotifications::notify`):**
The `title` parameter is the tab title, which is set via OSC 0/1/2 and
sanitized by `parse_osc()` (C0/C1 stripped, 256-char truncated). The
sanitized title is passed verbatim to `notify-rust`. If the notification
daemon renders HTML markup in the `summary` or `body` fields, an attacker
controls the notification appearance.

BEL flooding: 0x07 repeated at PTY throughput, no D-Bus rate limit.

### Security requirements

1. HTML-escape the tab title before passing to `notify-rust` `summary`.
2. `body` field: use static string or fully HTML-escaped content only.
3. BEL rate limit: at most one `notify()` call per 100 ms per pane.
4. `NotificationChangedEvent` struct: no PTY content fields — pane ID and
   type only.

### Security test scenarios

**SEC-NOTIF-001 through SEC-NOTIF-003**: see
`security-blocking-major-ipc-items.md §2.7`.

---

## Item 8 — Pane Focus → set_active_pane

**FS requirements:** FS-PANE-005
**Implementation gap:** Clicking a pane in the frontend does not call
`invoke('set_active_pane', { paneId })`.
**Entry point:** `src-tauri/src/commands/session_cmds.rs:79`

### Threat analysis

Once wired, `set_active_pane` becomes callable from the WebView. The primary
threat is XSS-driven focus stealing redirecting keystrokes to an
attacker-controlled pane. The secondary threat is rapid invocation flooding
the `session-state-changed` event bus.

The backend implementation already validates `pane_id` existence (returns
`INVALID_PANE_ID` for unknown IDs). No additional backend work is needed.
The risks are mitigated by CSP (XSS prevention) and the inherent
`SessionRegistry` lock serialization (DoS prevention).

### Security requirements

1. Active pane indicator (FS-PANE-006) must be non-spoofable from in-pane
   content — it is a DOM element outside the terminal canvas.
2. Frontend must only call `set_active_pane` in response to genuine user
   pointer events (click, keyboard navigation) — not from reactive state
   mutations that could be triggered programmatically.

### Security test scenarios

**SEC-FOCUS-001 through SEC-FOCUS-003**: see
`security-blocking-major-ipc-items.md §2.8`.

---

## Item 9 — Credential Store SSH

**FS requirements:** FS-CRED-001, FS-CRED-005
**Implementation gap:** `CredentialManager` / `CredentialStore` is NOT
injected into Tauri managed state in `lib.rs`. The `platform::create_pty_backend()`,
`PreferencesStore`, `SshManager`, and `SessionRegistry` are managed — but
`platform::create_credential_store()` (referenced in ARCHITECTURE.md §7.3)
is missing from `app.manage()` calls (`src-tauri/src/lib.rs:48–57`).

**This is a structural blocker.** Until `CredentialStore` is in managed
state, `provide_credentials` and the SSH auth flow cannot access it. Any
workaround that creates the credential store inside a command handler (not
as managed state) would produce a fresh instance per call, losing the
availability probe result and potentially producing non-deterministic
behavior.

### Threat analysis

**If the gap is worked around by instantiating `CredentialStore` per-command:**

- `is_available()` is called repeatedly; if D-Bus availability changes
  between calls, results are inconsistent.
- The `LinuxCredentialStore` instance is not shared, so multiple concurrent
  auth flows could race on Secret Service D-Bus calls.
- No fallback enforcement: if `is_available()` returns `false` in one call
  and `true` in another (due to a daemon restart), the credential may be
  stored inconsistently.

**If the gap is fixed correctly (managed state):**

- `is_available()` is called once at startup; the result is cached in the
  managed `Arc<dyn CredentialStore>`.
- All command handlers share the same instance, ensuring consistent
  availability state.

### Security requirements

1. `platform::create_credential_store()` must be called once at startup
   and registered via `app.manage(...)`.
2. Command handlers receive it via `State<'_, Arc<dyn CredentialStore>>`.
3. The managed instance probes `is_available()` once; if `false`, all
   subsequent `store()` calls return `Err(Unavailable)` without re-probing.
4. No credential data is stored outside the managed `CredentialStore` —
   no module-level statics, no session-scoped credential caches.

### Security test scenarios

**SEC-CRED-001 through SEC-CRED-008**: see
`security-pty-ipc-ssh-credentials-csp-osc52.md §2.4`.

**SEC-BLK-007 through SEC-BLK-008**: see
`security-blocking-ipc-wiring.md §3`.

**SEC-CREDSTORE-001** (new — managed state injection gap)

| Field | Value |
|-------|-------|
| **ID** | SEC-CREDSTORE-001 |
| **STRIDE** | Information Disclosure |
| **FS requirement(s)** | FS-CRED-001, FS-CRED-005 |
| **Threat** | `CredentialStore` is not in Tauri managed state. A workaround that instantiates it inside `provide_credentials` or `open_ssh_connection` creates a fresh `LinuxCredentialStore` per call. If D-Bus becomes available after the first call (daemon started mid-session), a second call succeeds and stores the credential. But the first call failed silently — the user was prompted each time without being informed. Worse: if the per-call instance bypasses the `is_available()` check, credentials may be stored when the probe would have returned `false`. |
| **Préconditions** | `CredentialStore` is not in managed state. SSH auth with password is attempted. |
| **Action** | Start TauTerm without a running Secret Service daemon. Trigger an SSH auth. Assert the user is informed that credential persistence is unavailable (FS-CRED-005). Start the Secret Service daemon during the session. Trigger a second SSH auth. Assert that `store()` is called with correct availability semantics — consistent with the initial probe. |
| **Résultat attendu (sécurisé)** | The `CredentialStore` availability probe runs once at startup. If `false`, all `store()` calls fail gracefully for the entire session. If the daemon starts later, TauTerm does not auto-detect this — the user must restart to enable persistence. This is the safe behavior. |
| **Risque si non mitigé** | High — non-deterministic credential storage behavior; user may believe credentials are persisted when they are not (or vice versa). |
| **Mitigation attendue dans le code** | `src-tauri/src/lib.rs` — add `app.manage(platform::create_credential_store())` before `setup`. `provide_credentials` and any SSH credential command must receive `State<'_, Arc<dyn CredentialStore>>` as a parameter, not construct the store themselves. |

---

## Item 10 — OSC Title Update

**FS requirements:** FS-VT-060 to FS-VT-062, FS-TAB-006
**Implementation gap:** OSC 0/1/2 sequences update `VtProcessor::title` but
the new title is not emitted to the frontend (no IPC event on title change).

### Threat analysis

When the title propagation is wired, the tab title reaches the frontend
via `ScreenUpdateEvent` or a dedicated `tab-title-changed` event. The title
has been sanitized by `parse_osc()` (C0/C1 stripped, 256-char truncated)
but HTML markup is not stripped. If the frontend uses `{@html title}`
anywhere, XSS is possible.

Additionally, the user's custom label (FS-TAB-006) overrides the OSC title.
The custom label is set via `rename_tab` IPC — a separate path with
different validation requirements.

### Security requirements

1. All frontend components rendering the tab title must use `{title}` text
   interpolation — never `{@html title}`.
2. `parse_osc()` must additionally strip HTML angle brackets from titles
   as defense-in-depth (SEC-BLK-015).
3. Unicode bidirectional control characters must be stripped from titles
   (SEC-BLK-017).
4. OSC title update rate must be limited to one event per 100 ms per pane
   (SEC-BLK-016).
5. The `rename_tab` IPC command validates the custom label: no HTML
   markup, no C0/C1 characters, max 256 characters.

### Security test scenarios

**SEC-PTY-006** — OSC title C0/C1 injection: see
`security-pty-ipc-ssh-credentials-csp-osc52.md §2.1`.

**SEC-BLK-015 through SEC-BLK-017** — XSS via HTML in title, rate
limiting, bidi override spoofing: see `security-blocking-ipc-wiring.md §6`.

**SEC-OSCTITLE-001** (new — custom label `rename_tab` validation)

| Field | Value |
|-------|-------|
| **ID** | SEC-OSCTITLE-001 |
| **STRIDE** | Tampering |
| **FS requirement(s)** | FS-TAB-006, ARCHITECTURE.md §9.4 (IPC boundary validation) |
| **Threat** | The `rename_tab` IPC command accepts a custom label string from the frontend. If the label is not validated, a user (or XSS script) can set a label containing `<script>alert(1)</script>`. If any component renders the label with `{@html}`, it executes. If the label contains C0 characters (e.g., embedded `\x1b` sequences), it may interfere with terminal title parsing if the label is ever echoed. |
| **Préconditions** | `rename_tab` IPC is callable from the WebView. |
| **Action** | Call `rename_tab` with `label = '<script>alert(1)</script>'`. Render the tab. Observe whether the script executes. Also test `label = '\x1b[31mred'` (C0 in label). |
| **Résultat attendu (sécurisé)** | Backend validates: `label.len() <= 256`, no C0/C1 characters (`\x00`–`\x1f`, `\x7f`–`\x9f`), no Unicode bidi overrides. Returns `INVALID_LABEL` error for violations. Frontend renders the (validated) label with `{label}` text interpolation — no `{@html}`. |
| **Risque si non mitigé** | High — XSS or C0 injection via user-controlled tab label. |
| **Mitigation attendue dans le code** | `src-tauri/src/commands/session_cmds.rs` `rename_tab` handler: validate label string. Frontend: use `{label}` interpolation in TabBar. The ARCHITECTURE.md §9.4 table already lists `String fields (tab label)` as an IPC boundary validation vector — verify the implementation matches. |

---

## Item 11 — Focus Events Mode 1004

**FS requirements:** FS-VT-084
**Implementation gap:** `ModeState.focus_events` is tracked; frontend does
not generate `ESC[I` (focus in) or `ESC[O` (focus out) sequences.

### Threat analysis

When wired, the frontend sends focus event bytes to the active pane's PTY
via `send_input`. The security risks are:

- Focus event flooding: the window repeatedly gaining/losing focus (e.g.,
  via an automated external tool) triggers a flood of `send_input` calls.
- Mode left active after process exit: successor shell receives focus
  sequences as raw input.
- Per-pane scoping: focus events must only go to the active pane, not all
  panes with mode 1004 enabled.

### Security requirements

1. Focus events sent only to the active pane's PTY (not broadcast).
2. `ModeState::reset_all()` sets `focus_events = false`.
3. Frontend focus event sender: debounce or deduplicate rapid focus
   in/out sequences (suggested: ignore a focus-out immediately followed
   by a focus-in within 50 ms — common with window manager decorations).

### Security test scenarios

**SEC-BLK-018 through SEC-BLK-020**: see
`security-blocking-ipc-wiring.md §7`.

---

## Item 12 — DECKPAM

**FS requirements:** FS-KBD-010
**Implementation gap:** `mode-state-changed` event carries `deckpam` flag;
`keyboard.ts` ignores it and always uses numeric keypad encoding.

### Threat analysis

When wired, `keyboard.ts` reads the `deckpam` flag from the mode state and
encodes keypad keys as application sequences (`ESC O p` for KP_0) vs.
numeric sequences (`0`). The risks are:

- Per-pane isolation: if the mode state is a global variable, a
  `mode-state-changed` event for pane B corrupts the active pane A's encoding.
- Mode flooding: rapid `ESC =` / `ESC >` alternation saturates the
  `mode-state-changed` event bus.
- Mode not reset on process exit: successor shell receives application-mode
  keypad sequences for numeric keys.

### Security requirements

1. Frontend mode state: `Map<PaneId, { decckm: bool, deckpam: bool }>` —
   not global booleans.
2. `mode-state-changed` emitted at most once per `VtProcessor::process()`
   batch — not once per DECKPAM/DECCKM sequence.
3. `ModeState::reset_all()` sets `deckpam = false`; emits updated event.

### Security test scenarios

**SEC-DECKPAM-001 through SEC-DECKPAM-003**: see
`security-blocking-major-ipc-items.md §2.12`.

---

## Implementation Sign-off Checklist

For each item, security sign-off requires all associated scenarios to pass.
This table is the gate before marking an item as complete.

| # | Item | Critical gate | Protocol reference |
|---|------|---------------|--------------------|
| 1 | Scrollback search | SEC-BLK-001, SEC-BLK-003, SEC-SEARCH-001 | blocking-ipc-wiring §1, this doc |
| 2 | SSH auth interactive | SEC-SSHAUTH-001, SEC-SSH-CH-001, SEC-SSH-CH-005 | this doc, pty-ipc §2.9 |
| 3 | SSH reconnection UI | SEC-RECON-001, SEC-RECON-002, SEC-RECON-003 | blocking-major-ipc §2.3 |
| 4 | Mouse reporting | SEC-BLK-009, SEC-BLK-010, SEC-BLK-011 | blocking-ipc-wiring §4 |
| 5 | Bracketed paste | SEC-BLK-012 (Critical — must pass before merge) | blocking-ipc-wiring §5 |
| 6 | Ctrl+Shift+V paste | SEC-PASTE-001 (Critical), SEC-PASTE-002 | blocking-major-ipc §2.6 |
| 7 | Tab notifications | SEC-NOTIF-001, SEC-NOTIF-002 | blocking-major-ipc §2.7 |
| 8 | Pane focus | SEC-FOCUS-001 (CSP dependency) | blocking-major-ipc §2.8 |
| 9 | Credential store | SEC-CREDSTORE-001, SEC-CRED-001, SEC-CRED-005 | this doc, pty-ipc §2.4 |
| 10 | OSC title | SEC-BLK-015 (Critical), SEC-OSCTITLE-001 | blocking-ipc-wiring §6, this doc |
| 11 | Focus events 1004 | SEC-BLK-018, SEC-BLK-020 | blocking-ipc-wiring §7 |
| 12 | DECKPAM | SEC-DECKPAM-003 | blocking-major-ipc §2.12 |

### Hard blockers (Critical severity — must be resolved before any release)

1. **SEC-BLK-012** — Bracketed paste `ESC[201~` stripping.
2. **SEC-BLK-015** — XSS via HTML markup in OSC tab title rendered in DOM.
3. **SEC-PASTE-001** — Multi-line paste confirmation when `bracketed_paste == false`.
4. **SEC-CREDSTORE-001** — `CredentialStore` must be in Tauri managed state.
5. **SEC-SSHAUTH-001** — No auto-accept of TOFU prompts; `accept_host_key`
   only callable from button `onclick`.
