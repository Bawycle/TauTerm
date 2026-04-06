<!-- SPDX-License-Identifier: MPL-2.0 -->
# Functional Specifications — Remote & SSH

> Part of the [Functional Specifications](README.md). See also: [00-overview.md](00-overview.md), [01-terminal-emulation.md](01-terminal-emulation.md), [02-ui-navigation.md](02-ui-navigation.md), [04-config-system.md](04-config-system.md), [05-scope-constraints.md](05-scope-constraints.md)

---

## 3.10 FS-SSH: SSH Session Management

### 3.10.1 Session Integration

| ID | Requirement | Priority |
|----|-------------|----------|
| FS-SSH-001 | The user MUST be able to open an SSH session in a new tab or a new pane. The SSH session MUST be visually integrated within TauTerm's tab/pane model. | Must |
| FS-SSH-002 | The user MUST be able to distinguish at a glance whether a tab or pane hosts a local or remote (SSH) session. | Must |
| FS-SSH-003 | All terminal emulation requirements (FS-VT-*) apply equally to SSH sessions as to local PTY sessions. | Must |

**Acceptance criteria:**
- FS-SSH-001: The user can open an SSH connection from a UI control, and it appears as a regular tab/pane.
- FS-SSH-002: An SSH tab/pane displays a visual indicator (e.g., icon, badge, or label) distinguishing it from local sessions.

### 3.10.2 Connection Lifecycle

| ID | Requirement | Priority |
|----|-------------|----------|
| FS-SSH-010 | SSH sessions MUST have distinct lifecycle states with visual representation: Connecting, Authenticating, Connected, Disconnected, Closed. State definitions: **Connecting** — TCP connection in progress. **Authenticating** — TCP established, SSH handshake and credential exchange in progress. **Connected** — session fully established and interactive. **Disconnected** — the session was interrupted unexpectedly (network drop, keepalive timeout, or remote process exit with non-zero code); reconnection is possible. **Closed** — the user has explicitly closed the pane or tab hosting the session, or the remote process exited normally (exit code 0 with no unexpected disconnect); the session is no longer active and no reconnection is possible — a new session must be opened to reconnect. | Must |
| FS-SSH-011 | Host key verification MUST follow the TOFU model. **First connection:** the prompt MUST display (a) a human-readable explanation in plain language (e.g., "TauTerm is connecting to `<host>` for the first time. To confirm you are connecting to the right server, verify the fingerprint below with your server administrator. If you are unsure, click Reject."), (b) the host key fingerprint in SHA-256 format, and (c) the key type (e.g., ED25519, RSA). **Key change:** the connection MUST be blocked immediately. A prominent warning dialog MUST be shown displaying: the stored fingerprint, the new fingerprint, a clear warning that a key change may indicate a man-in-the-middle attack, and an explanation of what to do (e.g., "Contact your server administrator to verify this change before accepting."). The default action MUST be rejection. Acceptance MUST require a deliberate non-default action. Accepted keys MUST be stored in TauTerm's own known-hosts file (`~/.config/tauterm/known_hosts`), in OpenSSH-compatible format. TauTerm MUST NOT read from or write to `~/.ssh/known_hosts`. The preferences UI MUST offer an import action to copy entries from `~/.ssh/known_hosts` into TauTerm's known-hosts file. | Must |
| FS-SSH-012 | Authentication MUST be attempted in this order: publickey, keyboard-interactive, password. A saved connection MAY specify a preferred method. | Must |
| FS-SSH-013 | The SSH PTY request MUST include: `TERM=xterm-256color`, terminal dimensions (cols, rows, xpixel, ypixel), and standard terminal modes encoded per RFC 4254 §6.2 and Annex A. The `encoded terminal modes` field MUST contain the following opcode/value pairs (TTY_OP_END = 0 terminates the list): VINTR (opcode 1, value 3 = ^C), VQUIT (opcode 2, value 28 = ^\), VERASE (opcode 3, value 127 = DEL), VKILL (opcode 4, value 21 = ^U), VEOF (opcode 5, value 4 = ^D), VSUSP (opcode 10, value 26 = ^Z), ISIG (opcode 50, value 1), ICANON (opcode 51, value 1), ECHO (opcode 53, value 1). Note: these opcodes are the RFC 4254 Annex A numbering — they are NOT the `termios` struct field indices from the Linux kernel header. | Must |
| FS-SSH-014 | If the negotiated host key algorithm is deprecated (specifically: `ssh-rsa` with SHA-1, or `ssh-dss`), TauTerm MUST display a non-blocking warning in the pane after connection is established. The warning MUST name the deprecated algorithm and state that the server should be updated. The connection MUST NOT be refused. The warning MUST be dismissible by the user. | Must |

**Acceptance criteria:**
- FS-SSH-010: Each lifecycle state is reflected in the pane UI (e.g., status bar, overlay, or icon change). When the user closes an SSH pane or the remote shell exits cleanly (exit code 0), the pane or tab enters the Closed state: no reconnection control is shown and no error indicator is shown. When the connection drops unexpectedly (network interruption, keepalive timeout, or non-zero exit), the pane enters the Disconnected state and displays a reconnection control.
- FS-SSH-011: Connecting to a new host shows a plain-language prompt with the SHA-256 fingerprint and key type. Connecting to a host whose key has changed: connection is blocked, both fingerprints are shown side by side, a MITM warning and actionable instructions are displayed, default action is Reject, acceptance requires a non-default deliberate action.
- FS-SSH-012: A connection using a key file authenticates without prompting for a password.
- FS-SSH-014: Connecting to a server that only offers `ssh-rsa` (SHA-1) shows a visible, dismissible warning in the pane naming the algorithm. The connection is established and functional.

### 3.10.3 Connection Health

| ID | Requirement | Priority |
|----|-------------|----------|
| FS-SSH-020 | SSH keepalive MUST be enabled by default, with an interval of 30 seconds. Three consecutive missed keepalives MUST trigger a transition to the Disconnected state. Keepalive interval and threshold MUST be configurable per connection. | Must |
| FS-SSH-021 | Pane resize MUST trigger an SSH `window-change` channel request with the new dimensions (debounced, same as local PTY). | Must |
| FS-SSH-022 | Connection drop MUST be detected via TCP keepalive, SSH keepalive, or write failure. The Disconnected state MUST be entered within 1 second of detection, with the reason displayed. | Must |

**Acceptance criteria:**
- FS-SSH-020: Blocking the network for 90 seconds triggers the Disconnected state.
- FS-SSH-021: Resizing a pane with an SSH session causes the remote terminal to redraw.
- FS-SSH-022: Disconnection shows a reason (e.g., "Connection timed out").

### 3.10.4 Saved Connections

| ID | Requirement | Priority |
|----|-------------|----------|
| FS-SSH-030 | The user MUST be able to save SSH connections with at minimum: host, port, username, authentication method (identity file path or password reference), and optional label/group. | Must |
| FS-SSH-031 | Saved connections MUST be listed in a dedicated UI (e.g., connection manager panel or quick-open dialog). | Must |
| FS-SSH-031a | The SSH connections panel toggle button MUST be permanently visible in the tab row, outside the scrollable tab area. It MUST NOT be pushed off-screen or occluded when many tabs are open. | Must |
| FS-SSH-032 | From the saved connections list, the user MUST be able to open a connection in a new tab or pane with a single action. | Must |
| FS-SSH-033 | The user MUST be able to create, edit, duplicate, and delete saved connections. | Must |
| FS-SSH-034 | Saved connections MUST be stored persistently as part of user preferences. | Must |

**Acceptance criteria:**
- FS-SSH-030: A saved connection stores host, port, username, and auth method.
- FS-SSH-031: A connection manager UI lists all saved connections.
- FS-SSH-031a: With 20+ tabs open, the SSH connections toggle button remains fully visible at the right edge of the tab row with no overlap or clipping.
- FS-SSH-032: Clicking a saved connection opens an SSH session in a new tab.
- FS-SSH-033: All CRUD operations and duplication work from the UI.

### 3.10.5 Reconnection

| ID | Requirement | Priority |
|----|-------------|----------|
| FS-SSH-040 | When an SSH session is interrupted, the user MUST be able to reconnect to the same connection without reconfiguring. | Must |
| FS-SSH-041 | The reconnection action MUST be accessible directly from the affected tab or pane. | Must |
| FS-SSH-042 | On reconnection, scrollback MUST be preserved. A visual separator MUST be displayed at the reconnection boundary. | Must |

**Acceptance criteria:**
- FS-SSH-040: After disconnection, clicking "Reconnect" re-establishes the SSH session.
- FS-SSH-041: The reconnection button/action is visible in the disconnected pane.
- FS-SSH-042: After reconnection, previous scrollback is intact, with a clear separator line.

---

## 3.11 FS-CRED: Credential Security

| ID | Requirement | Priority |
|----|-------------|----------|
| FS-CRED-001 | Credentials (passwords, passphrases) MUST be stored using the OS keychain via the Secret Service D-Bus API (e.g., `libsecret` / `keyring` crate). They MUST NOT be stored in plain text, in environment variables, or in the preferences file. | Must |
| FS-CRED-002 | Identity files (private keys) MUST be referenced by file path. TauTerm MUST NOT copy, embed, or read private key file content beyond what is needed for authentication. | Must |
| FS-CRED-003 | Credentials retrieved from the keychain for authentication MUST be cleared from process memory as soon as authentication completes or fails. Credentials MUST NOT be cached in application state beyond the duration of the authentication handshake. | Must |
| FS-CRED-004 | Credentials (passwords, passphrases, key material) MUST NOT appear in log output, error messages, IPC payloads, or debug traces, at any log level. | Must |
| FS-CRED-005 | If the OS keychain is unavailable (no Secret Service provider running), TauTerm MUST NOT fall back to insecure storage. Instead, it MUST prompt the user for credentials on each connection attempt and inform the user that credential persistence is unavailable. | Must |
| FS-CRED-006 | Identity file paths stored in saved connections MUST be validated at connection time: the path MUST be resolved to an absolute path, MUST NOT contain path traversal components (e.g., `../`), and MUST point to a regular file. Symlinks MAY be followed. | Must |

**Acceptance criteria:**
- FS-CRED-001: Inspecting the preferences file on disk reveals no plaintext passwords. Credentials are retrievable via `secret-tool lookup`.
- FS-CRED-002: The saved connection configuration contains a path string, not key content.
- FS-CRED-003: After an SSH connection is established, a memory dump of the TauTerm process does not contain the password used for authentication.
- FS-CRED-004: Enabling maximum log verbosity and connecting with a password does not log the password.
- FS-CRED-005: With no keychain available, TauTerm prompts for the password each time and displays a notice about unavailable credential persistence.
- FS-CRED-006: A saved connection with identity path `../../etc/shadow` is rejected at connection time with a clear error.
