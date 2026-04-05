# Security Test Protocol — TauTerm

Version: 1.0 — 2026-04-04
Scope: Linux (x86, x86_64, ARM32, ARM64, RISC-V) — v1 only
Author role: security-expert

---

## Table of Contents

1. [Threat Model](#1-threat-model)
   - 1.1 [Assets](#11-assets)
   - 1.2 [Threat Actors](#12-threat-actors)
   - 1.3 [Attack Surface](#13-attack-surface)
2. [Security Test Scenarios](#2-security-test-scenarios)
   - 2.1 [PTY Security](#21-pty-security)
   - 2.2 [IPC Security](#22-ipc-security)
   - 2.3 [SSH Security](#23-ssh-security)
   - 2.4 [Credential Storage](#24-credential-storage)
   - 2.5 [WebView / CSP](#25-webview--csp)
   - 2.6 [OSC 52 Clipboard](#26-osc-52-clipboard)
   - 2.7 [Path and Input Validation](#27-path-and-input-validation)
3. [Penetration Test Checklist](#3-penetration-test-checklist)
4. [Security Regression Policy](#4-security-regression-policy)

---

## 1. Threat Model

### 1.1 Assets

| Asset | Description | Sensitivity |
|-------|-------------|-------------|
| SSH passwords and passphrases | Credentials used for SSH password authentication and private key passphrase unlock. Stored via Secret Service D-Bus API. | Critical |
| SSH private keys | Key material on disk referenced by path. TauTerm reads content only at authentication time and must not retain it. | Critical |
| `known_hosts` file | `~/.config/tauterm/known_hosts` — TOFU records used to detect MITM attacks. Tampering undermines host verification. | High |
| PTY input stream | Bytes written to the PTY master fd. Injecting into this stream is equivalent to typing arbitrary commands as the user. | High |
| PTY output stream | Raw terminal output from child processes, including potentially sensitive data (passwords displayed in clear, environment dumps). | High |
| System clipboard | Content of the CLIPBOARD selection. An application running in a pane could attempt to read or overwrite it. | High |
| IPC command channel | Tauri `invoke()` calls from the WebView to Rust command handlers. Unauthorized invocation can trigger privileged operations. | High |
| `preferences.json` | Persisted preferences including saved SSH connection configs (host, port, username, identity file paths). No passwords stored here, but paths and host metadata are sensitive. | Medium |
| WebView renderer process | The Svelte/WebKit process. Compromise through XSS can lead to arbitrary IPC invocation. | High |

### 1.2 Threat Actors

| Actor | Description | Capability |
|-------|-------------|------------|
| Malicious remote server | An SSH server (or MITM attacker) controlling PTY output sent to TauTerm. Can craft arbitrary escape sequences, OSC payloads, and hyperlink URIs. | Controlled output to VtProcessor; cannot directly invoke IPC. |
| Malicious local process | A process running inside a TauTerm pane (shell, script, or program). Has full PTY output access. In a local session, also has access to the local filesystem and D-Bus. | Same as remote server, plus local fs access. |
| Untrusted web content in WebView | Not applicable under normal operation (CSP blocks remote scripts), but relevant if CSP is misconfigured or a dependency is compromised. | Can call `invoke()` if it runs in the WebView context. |
| Passive network attacker | Can observe unencrypted traffic. SSH encrypts the channel; the risk is at the SSH negotiation level (weak algorithms, host key bypass). | Network eavesdropping; cannot modify encrypted traffic. |
| Active network attacker (MITM) | Can intercept and modify traffic between TauTerm and an SSH server. Mitigated by TOFU host key verification. | MITM on TCP; cannot forge a known host key. |
| Local user with filesystem access | Can tamper with `preferences.json` or `known_hosts` on disk. Not a primary adversary (they already have user-level access), but injection via tampered config is a realistic risk. | Read/write to user's `~/.config/tauterm/`. |

### 1.3 Attack Surface

| Surface | Entry Point | Notes |
|---------|-------------|-------|
| PTY output (VtProcessor) | `VtProcessor::process()` — `src-tauri/src/vt/processor.rs` | Largest attack surface: all VT sequences, OSC payloads, DCS sequences, hyperlink URIs. |
| IPC command boundary | `#[tauri::command]` handlers — `src-tauri/src/commands/` | All Tauri commands accept deserialized JSON from the WebView. |
| SSH negotiation layer | `russh` client, `src-tauri/src/ssh/` | Host key verification, algorithm selection, credential exchange. |
| Credential store | `LinuxCredentialStore` — `src-tauri/src/platform/credentials_linux.rs` | D-Bus calls to Secret Service; availability probe; fallback behavior. |
| Preferences load path | `PreferencesStore::load_or_default()` — `src-tauri/src/preferences/store.rs` | Reads and deserializes `preferences.json`; path traversal in identity file fields. |
| WebView / CSP | `tauri.conf.json` CSP directive | Currently `null` — not yet tightened. Script injection, inline execution. |
| PTY file descriptor hygiene | `portable-pty` `open_session` — `src-tauri/src/platform/pty_linux.rs` | `O_CLOEXEC` on master fd; fd leak to child processes. |
| System browser invocation | `open_url()` — `src-tauri/src/commands/system_cmds.rs` | URI scheme validation before shell delegation. |
| OSC 52 clipboard write | `parse_osc()` — `src-tauri/src/vt/osc.rs` | Controlled by `allow_osc52_write` policy; read is permanently rejected. |

---

## 2. Security Test Scenarios

### 2.1 PTY Security

#### SEC-PTY-001

| Field | Value |
|-------|-------|
| **ID** | SEC-PTY-001 |
| **STRIDE** | Tampering |
| **FS requirement(s)** | FS-VT-063 |
| **Threat** | A process running in a pane emits `CSI 21t` (report window title). The terminal responds by injecting the title string into the PTY input stream, which could contain shell commands if the title was previously set by an attacker via OSC 0. |
| **Test method** | Unit test: feed `\x1b[21t` to `VtProcessor::process()`. Assert that no data is written to the PTY master (no callback, no response queued). Confirm by also feeding an OSC 0 title-setting sequence followed by `CSI 21t` — still no input injection. |
| **Expected mitigation** | `VtProcessor` silently discards `CSI 21t` with no response. Implemented in the `csi_dispatch` handler — read-back sequences are in a permanent blocklist. |
| **Priority** | Critical |
| **Environment** | Standard unit test; no external dependencies. |

#### SEC-PTY-002

| Field | Value |
|-------|-------|
| **ID** | SEC-PTY-002 |
| **STRIDE** | Tampering |
| **FS requirement(s)** | FS-VT-063 |
| **Threat** | OSC query sequences that would cause TauTerm to echo terminal state (e.g., color queries, `DECRQSS`, `DECRPM` responses) into the PTY input buffer. An attacker sets a malicious title then issues `OSC ?` to exfiltrate it as an injection payload. |
| **Test method** | Unit test: feed OSC query variants (e.g., `\x1b]10;?\x07`, `\x1b[?1$p`) to `VtProcessor::process()`. Assert no bytes are written back. |
| **Expected mitigation** | All OSC read-back queries return `OscAction::Ignore`. `DECRQSS` and `DECRPM` responses are discarded. |
| **Priority** | Critical |
| **Environment** | Standard unit test. |

#### SEC-PTY-003

| Field | Value |
|-------|-------|
| **ID** | SEC-PTY-003 |
| **STRIDE** | Denial of Service |
| **FS requirement(s)** | FS-SEC-005 |
| **Threat** | A malicious process emits an OSC sequence with a payload exceeding 4096 bytes (e.g., `\x1b]0;` followed by 100 000 `A` characters). This causes unbounded memory allocation in the VtProcessor's sequence accumulation buffer. |
| **Test method** | Unit test + fuzzing target: feed `\x1b]0;` + 10 000 bytes + `\x07`. Assert the sequence is discarded without panicking. Assert memory usage does not grow proportionally to payload size. Fuzzing: `cargo-fuzz` target on `VtProcessor::process()`. |
| **Expected mitigation** | VtProcessor enforces a 4096-byte limit on individual OSC/DCS sequences. Sequences exceeding the limit are dropped; subsequent input is unaffected. |
| **Priority** | High |
| **Environment** | Unit test: standard. Fuzzing: requires `cargo-fuzz` with `libFuzzer`. |

#### SEC-PTY-004

| Field | Value |
|-------|-------|
| **ID** | SEC-PTY-004 |
| **STRIDE** | Denial of Service |
| **FS requirement(s)** | FS-SEC-005 |
| **Threat** | DCS sequences with excessively large payloads (e.g., `\x1bP` followed by 100 000 bytes). Same memory exhaustion vector as SEC-PTY-003 but via the DCS path. |
| **Test method** | Unit test: feed `\x1bP` + 10 000 bytes + `\x1b\\`. Assert discard and no panic. |
| **Expected mitigation** | Same 4096-byte per-sequence limit applied to DCS accumulation. |
| **Priority** | High |
| **Environment** | Standard unit test. |

#### SEC-PTY-005

| Field | Value |
|-------|-------|
| **ID** | SEC-PTY-005 |
| **STRIDE** | Information Disclosure |
| **FS requirement(s)** | FS-SEC-002 |
| **Threat** | A child process spawned by the user's shell inspects `/proc/self/fd` and discovers open PTY master file descriptors belonging to other panes. This allows cross-pane snooping or injection. |
| **Test method** | Integration test (requires PTY implementation): spawn two PTY sessions. From the shell in pane 1, run `ls -la /proc/self/fd`. Assert that no fd corresponds to pane 2's master PTY. |
| **Expected mitigation** | `portable-pty`'s `UnixPtySystem` opens master fds with `O_CLOEXEC`. Verified at PTY implementation time. |
| **Priority** | High |
| **Environment** | Requires PTY implementation (currently stub — `todo!()`). |
| **Stub dependency** | `LinuxPtySession::write` and `open_session` must be implemented before this test can run. |

#### SEC-PTY-006

| Field | Value |
|-------|-------|
| **ID** | SEC-PTY-006 |
| **STRIDE** | Tampering |
| **FS requirement(s)** | FS-VT-062 |
| **Threat** | A malicious OSC 0 payload sets a tab title containing C0/C1 control characters (e.g., embedded `\x1b[` sequences). If unsanitized, these could be re-interpreted when the title is rendered or echoed. |
| **Test method** | Unit test: feed `\x1b]0;\x01\x0b\x1b[31mInjection\x07` to `parse_osc()`. Assert the resulting `OscAction::SetTitle` contains none of the control characters. |
| **Expected mitigation** | `parse_osc()` filters all control characters (excluding `\t`) from title payloads and truncates to 256 characters. |
| **Priority** | Medium |
| **Environment** | Standard unit test; `osc.rs` already has the filter. |

#### SEC-PTY-007

| Field | Value |
|-------|-------|
| **ID** | SEC-PTY-007 |
| **STRIDE** | Tampering |
| **FS requirement(s)** | FS-VT-016 |
| **Threat** | A malicious PTY output stream contains invalid UTF-8 sequences (overlong encodings such as `0xC0 0xAF`). If passed unsanitized to the frontend or stored in the screen buffer, they could cause rendering issues or be leveraged in injection attacks that rely on encoding ambiguity. |
| **Test method** | Unit test: feed known-invalid UTF-8 byte sequences to `VtProcessor::process()`. Assert they are replaced with U+FFFD and do not propagate raw invalid bytes to `ScreenBuffer`. |
| **Expected mitigation** | The `vte` crate replaces invalid UTF-8 with U+FFFD before dispatch to `Perform` callbacks. |
| **Priority** | Medium |
| **Environment** | Standard unit test. |

#### SEC-PTY-008 (Fuzzing)

| Field | Value |
|-------|-------|
| **ID** | SEC-PTY-008 |
| **STRIDE** | Denial of Service / Tampering |
| **FS requirement(s)** | FS-VT-005, FS-SEC-005 |
| **Threat** | Arbitrary byte sequences fed to the VtProcessor trigger panics, integer overflows, stack overflows, or unbounded memory allocation. This covers all VT parsing paths not individually specified. |
| **Test method** | Fuzzing: implement a `cargo-fuzz` target `fuzz_vt_processor` that repeatedly calls `VtProcessor::process()` with random byte sequences. Run under address sanitizer and with a corpus of known-good terminal streams. Target minimum 24 hours of fuzzing time before a release. |
| **Expected mitigation** | No `panic!`, `unwrap()`, or OOM on any input. The VtProcessor must handle all byte sequences gracefully. |
| **Priority** | High |
| **Environment** | Requires `cargo-fuzz` (nightly toolchain, libFuzzer). |

---

### 2.2 IPC Security

#### SEC-IPC-001

| Field | Value |
|-------|-------|
| **ID** | SEC-IPC-001 |
| **STRIDE** | Tampering |
| **FS requirement(s)** | FS-SEC-001 |
| **Threat** | A script injected into the WebView DOM (via a hypothetical XSS in a dependency) calls `invoke('send_input', { paneId: '...', data: [27, 91, 65] })` to inject arbitrary bytes into a PTY session. |
| **Test method** | Static review: confirm CSP in `tauri.conf.json` has `script-src 'self'` with no `unsafe-inline` or `unsafe-eval`. Confirm no `{@html}` usage with message accessors in Svelte components. Manual test: attempt to inject a `<script>` tag via browser devtools — it must be blocked by CSP. |
| **Expected mitigation** | CSP `script-src 'self'` blocks inline and injected scripts. No `{@html}` with user-controlled data in the frontend. |
| **Priority** | Critical |
| **Environment** | Static analysis + manual test on running app. Requires CSP to be set (currently `null` in `tauri.conf.json` — this is a known stub). |
| **Stub dependency** | CSP must be configured in `tauri.conf.json` before this test is meaningful. |

#### SEC-IPC-002

| Field | Value |
|-------|-------|
| **ID** | SEC-IPC-002 |
| **STRIDE** | Spoofing / Elevation of Privilege |
| **FS requirement(s)** | ARCHITECTURE.md §8.1 |
| **Threat** | A Tauri command receives an invalid or out-of-range `PaneId` / `TabId` / `ConnectionId`. Without validation, this could cause a panic (DoS) or reference a pane belonging to a different tab than intended. |
| **Test method** | Unit tests for each command accepting an ID type: send an empty string, a UUID not present in the registry, and a string of 10 000 characters. Assert that `TauTermError` is returned with code `INVALID_PANE_ID` (or equivalent) — no panic. |
| **Expected mitigation** | Command handlers call `registry.get_pane(pane_id)` which returns `Err(TauTermError)` for unknown IDs. The `PaneId` newtype prevents cross-type confusion. |
| **Priority** | High |
| **Environment** | Standard unit test. |

#### SEC-IPC-003

| Field | Value |
|-------|-------|
| **ID** | SEC-IPC-003 |
| **STRIDE** | Tampering |
| **FS requirement(s)** | ARCHITECTURE.md §8.1, FS-CRED-004 |
| **Threat** | The `provide_credentials` command receives a `Credentials` struct. If the backend logs the deserialized struct at any log level (e.g., via `tracing::debug!("{:?}", credentials)`), passwords appear in log files. |
| **Test method** | Code review: search all `tracing::*` macro calls in `src-tauri/src/` for any that could capture a `Credentials` value. Verify `Credentials` does not implement `Debug` or that the `Debug` impl redacts sensitive fields. |
| **Expected mitigation** | `Credentials` either has no derived `Debug`, or its `Debug` impl redacts `password` and `private_key_path` fields. |
| **Priority** | Critical |
| **Environment** | Static analysis (code review + `grep`). |

#### SEC-IPC-004

| Field | Value |
|-------|-------|
| **ID** | SEC-IPC-004 |
| **STRIDE** | Denial of Service |
| **FS requirement(s)** | ARCHITECTURE.md §4.1 |
| **Threat** | A large JSON payload sent to a Tauri command (e.g., `save_connection` with a hostname field of 1 MB) causes the command handler to allocate excessive memory or take excessive time during deserialization. |
| **Test method** | Unit test: invoke `save_connection` with a `SshConnectionConfig` where `host` is a 1 MB string. Assert the command returns an error (or completes) within 1 second with no OOM. |
| **Expected mitigation** | Serde deserialization is length-limited by the Tauri IPC layer. Individual string fields are validated for length in the command handler (host: max 253 chars per DNS spec; username: max 255 chars per POSIX). |
| **Priority** | Medium |
| **Environment** | Standard unit test. |

#### SEC-IPC-005

| Field | Value |
|-------|-------|
| **ID** | SEC-IPC-005 |
| **STRIDE** | Tampering |
| **FS requirement(s)** | FS-I18N-006, CLAUDE.md constraints |
| **Threat** | The `update_preferences` command receives a `PreferencesPatch` where `language` is set to an unknown string value (e.g., `"de"` or a SQL injection payload). If deserialized as a free `String` rather than the `Language` enum, it could bypass validation. |
| **Test method** | Unit test: send a JSON payload `{"appearance": {"language": "de"}}` to `update_preferences`. Assert serde rejects the unknown variant and returns a `TauTermError` — the preferences are not modified. |
| **Expected mitigation** | `Language` is a `#[serde(rename_all = "camelCase")]` enum. Unknown variants fail deserialization at the IPC boundary (FS-I18N-006). |
| **Priority** | Medium |
| **Environment** | Standard unit test. |

#### SEC-IPC-006

| Field | Value |
|-------|-------|
| **ID** | SEC-IPC-006 |
| **STRIDE** | Tampering |
| **FS requirement(s)** | ARCHITECTURE.md §8.1 |
| **Threat** | The `send_input` command is called with a `data` field containing `Vec<u8>` of arbitrary length. If no length limit is enforced, an attacker can fill the PTY write buffer with a multi-megabyte payload, causing the PTY write task to block or consume excessive memory. |
| **Test method** | Unit test: call `send_input` with `data` of 10 MB. Assert the command returns an error or silently truncates to a reasonable limit (suggested: 65 536 bytes). No blocking of the async runtime. |
| **Expected mitigation** | `send_input` validates `data.len()` and returns `TauTermError` if it exceeds the configured maximum. |
| **Priority** | Medium |
| **Environment** | Standard unit test (PTY write is stubbed, so only the validation layer is tested at this stage). |

---

### 2.3 SSH Security

#### SEC-SSH-001

| Field | Value |
|-------|-------|
| **ID** | SEC-SSH-001 |
| **STRIDE** | Spoofing |
| **FS requirement(s)** | FS-SSH-011 |
| **Threat** | TauTerm connects to a host it has never seen before without displaying a TOFU confirmation prompt. The user silently accepts a key they have not verified, enabling silent MITM on all future connections to that host. |
| **Test method** | Integration test (requires SSH mock): connect to a mock SSH server with a fresh `known_hosts` file. Assert that a `host-key-prompt` event is emitted to the frontend with the correct fingerprint and key type before the connection proceeds. Assert the connection is not established until `accept_host_key` is called. |
| **Expected mitigation** | `known_hosts.rs` checks the host; if not found, it emits `HostKeyPromptEvent` and blocks. The connection proceeds only after explicit `accept_host_key` command. |
| **Priority** | Critical |
| **Environment** | Requires SSH server mock (e.g., `openssh` in a container, or a `russh` test server). |
| **Stub dependency** | `known_hosts.rs` and `auth.rs` are stubs. Full SSH integration pass required. |

#### SEC-SSH-002

| Field | Value |
|-------|-------|
| **ID** | SEC-SSH-002 |
| **STRIDE** | Spoofing |
| **FS requirement(s)** | FS-SSH-011 |
| **Threat** | A host key changes (MITM scenario or server re-key). TauTerm silently accepts the new key and overwrites the stored one. The user is not warned that a MITM attack may be in progress. |
| **Test method** | Integration test: add a known host entry to `~/.config/tauterm/known_hosts`. Connect to a mock server presenting a different key for that host. Assert the connection is immediately blocked. Assert the frontend receives a `host-key-prompt` event displaying both the stored and new fingerprints. Assert the default action is rejection. |
| **Expected mitigation** | `known_hosts.rs` detects the mismatch; blocks the connection; emits a prominent warning event. `accept_host_key` requires explicit invocation to proceed (non-default action). |
| **Priority** | Critical |
| **Environment** | Requires SSH server mock with key rotation capability. |
| **Stub dependency** | Same as SEC-SSH-001. |

#### SEC-SSH-003

| Field | Value |
|-------|-------|
| **ID** | SEC-SSH-003 |
| **STRIDE** | Information Disclosure |
| **FS requirement(s)** | FS-SEC-004 |
| **Threat** | SSH agent forwarding is enabled, allowing remote servers to use the user's SSH agent to authenticate to third-party services. A compromised SSH server can hijack the forwarded agent. |
| **Test method** | Code review: verify no SSH agent forwarding channel is opened in the `russh` client handler. Integration test: connect to a mock server that requests agent forwarding; assert the request is rejected or ignored. |
| **Expected mitigation** | Agent forwarding is permanently disabled (FS-SEC-004). No agent socket is created; any server request for agent forwarding is silently refused. |
| **Priority** | Critical |
| **Environment** | Code review: immediate. Integration test requires SSH mock. |

#### SEC-SSH-004

| Field | Value |
|-------|-------|
| **ID** | SEC-SSH-004 |
| **STRIDE** | Information Disclosure |
| **FS requirement(s)** | FS-SSH-014 |
| **Threat** | TauTerm connects to a server offering only `ssh-rsa` (SHA-1) or `ssh-dss`. These algorithms are considered deprecated (SHA-1 collision attacks; DSS 1024-bit key weakness). The connection is silently established, exposing the user to a degraded security posture. |
| **Test method** | Integration test: connect to a mock server restricted to `ssh-rsa` (SHA-1). Assert a non-blocking in-pane warning event is emitted naming the deprecated algorithm. Assert the connection is established (not refused). Assert the warning is dismissible. |
| **Expected mitigation** | `algorithms.rs` detects the negotiated algorithm post-handshake and emits an in-pane warning banner. |
| **Priority** | High |
| **Environment** | Requires SSH mock configured to offer only deprecated algorithms. |
| **Stub dependency** | `algorithms.rs` is a stub. |

#### SEC-SSH-005

| Field | Value |
|-------|-------|
| **ID** | SEC-SSH-005 |
| **STRIDE** | Information Disclosure |
| **FS requirement(s)** | FS-SSH-013 |
| **Threat** | The SSH PTY request is sent with incorrect or missing RFC 4254 terminal mode opcodes. Specifically, using Linux `termios` struct field indices instead of RFC 4254 Annex A opcodes results in incorrect terminal behavior at the remote end, and may cause the remote side to interpret control characters differently than expected — creating subtle injection opportunities via misrouted signals. |
| **Test method** | Code review: verify `ssh/connection.rs` (or the auth/channel setup) constructs the terminal modes list using RFC 4254 Annex A opcodes (VINTR=1, VQUIT=2, VERASE=3, VEOF=4, VKILL=5, VSUSP=10, ISIG=50, ICANON=51, ECHO=53) and not the Linux `termios` struct indices. |
| **Expected mitigation** | Terminal modes encoded per RFC 4254 §6.2 and Annex A with correct opcode numbers (FS-SSH-013). |
| **Priority** | High |
| **Environment** | Code review only at this stage. |
| **Stub dependency** | `connection.rs` channel setup is a stub. Verify when implemented. |

#### SEC-SSH-006

| Field | Value |
|-------|-------|
| **ID** | SEC-SSH-006 |
| **STRIDE** | Information Disclosure |
| **FS requirement(s)** | FS-SSH-020 |
| **Threat** | SSH keepalive is not implemented or not triggered on timeout. A stale TCP connection remains open in a half-open state. The user continues to type commands that are silently discarded, and credentials entered into the "connected" session may be sent to a server that is no longer reachable or has been hijacked. |
| **Test method** | Integration test: establish an SSH connection, block the network for 90 seconds, assert the pane transitions to `Disconnected` state. Assert the transition occurs within 95 seconds (30-second interval * 3 misses + 5-second margin). |
| **Expected mitigation** | Keepalive sends a `ignore` packet every 30 seconds. Three misses trigger `Disconnected` state transition. |
| **Priority** | High |
| **Environment** | Requires network simulation (e.g., `iptables DROP` on the loopback, or a mock server that stops responding). |
| **Stub dependency** | `keepalive.rs` is a stub. |

---

### 2.4 Credential Storage

#### SEC-CRED-001

| Field | Value |
|-------|-------|
| **ID** | SEC-CRED-001 |
| **STRIDE** | Information Disclosure |
| **FS requirement(s)** | FS-CRED-001 |
| **Threat** | SSH passwords or passphrases are persisted to `preferences.json` in plaintext. An attacker with read access to `~/.config/tauterm/preferences.json` obtains all saved credentials. |
| **Test method** | Manual test: save an SSH connection with a password. Read `~/.config/tauterm/preferences.json`. Assert no password field is present. Verify credentials are retrievable via `secret-tool lookup service tauterm`. |
| **Expected mitigation** | Passwords are stored only in the OS keychain (Secret Service D-Bus). The `SshConnectionConfig` struct contains no password field — only `identity_file` (a path). The `PreferencesStore` never writes credential data. |
| **Priority** | Critical |
| **Environment** | Manual test. Requires Secret Service implementation. |
| **Stub dependency** | `credentials_linux.rs` is a stub (`is_available()` returns `false`). |

#### SEC-CRED-002

| Field | Value |
|-------|-------|
| **ID** | SEC-CRED-002 |
| **STRIDE** | Information Disclosure |
| **FS requirement(s)** | FS-CRED-003 |
| **Threat** | After a successful SSH authentication, the plaintext password (or decoded passphrase) remains in TauTerm's process heap. A memory dump of the TauTerm process (e.g., from `/proc/<pid>/mem`) reveals the credential. |
| **Test method** | Manual test (Linux only): connect to an SSH server using password authentication. After connection is established, use `strings /proc/<pid>/mem` or a debugger to search for the password string in process memory. |
| **Expected mitigation** | The credential byte buffer is zeroed immediately after `russh` consumes it for authentication. Rust's `zeroize` crate (or manual zeroing) applied to the `Credentials` struct after the authentication handshake. |
| **Priority** | Critical |
| **Environment** | Requires SSH implementation and a test SSH server. Root access or `ptrace` capability on the test machine. |
| **Stub dependency** | `auth.rs` and the full SSH connection flow are stubs. |

#### SEC-CRED-003

| Field | Value |
|-------|-------|
| **ID** | SEC-CRED-003 |
| **STRIDE** | Information Disclosure |
| **FS requirement(s)** | FS-CRED-004 |
| **Threat** | At maximum `tracing` verbosity (`RUST_LOG=trace`), the backend logs a debug representation of the `Credentials` struct, revealing passwords or key paths in log output. |
| **Test method** | Code review: verify `Credentials` (in `manager.rs`) has a manually implemented `Debug` that redacts `password` (e.g., `Some("***")`) and `private_key_path`. Unit test: format `Credentials { password: Some("hunter2") }` with `{:?}` and assert the output does not contain `hunter2`. |
| **Expected mitigation** | `Credentials` has a custom `Debug` impl that replaces sensitive fields with `"[REDACTED]"`. |
| **Priority** | Critical |
| **Environment** | Code review + unit test. No external dependencies. |

#### SEC-CRED-004

| Field | Value |
|-------|-------|
| **ID** | SEC-CRED-004 |
| **STRIDE** | Information Disclosure |
| **FS requirement(s)** | FS-CRED-002 |
| **Threat** | TauTerm reads the full content of an SSH private key file into memory and includes it in an IPC payload or logs it. The private key material is exposed beyond the authentication handshake. |
| **Test method** | Code review: verify `SshConnectionConfig.identity_file` stores only a path string. Verify that key file content is never serialized into any IPC payload or Tauri event. Unit test: confirm `SshConnectionConfig` serialized to JSON contains only a path string, not key content. |
| **Expected mitigation** | `SshConnectionConfig` stores the file path only. Key content is read by `russh` directly from the path at authentication time and not retained in application state. |
| **Priority** | Critical |
| **Environment** | Code review + unit test. |

#### SEC-CRED-005

| Field | Value |
|-------|-------|
| **ID** | SEC-CRED-005 |
| **STRIDE** | Information Disclosure |
| **FS requirement(s)** | FS-CRED-005 |
| **Threat** | The Secret Service D-Bus provider is unavailable (e.g., no keychain daemon running). TauTerm falls back to storing the password in `preferences.json` or in a temporary plaintext file rather than prompting the user on each connection. |
| **Test method** | Integration test: with no Secret Service provider active (e.g., `dbus-run-session` without `gnome-keyring-daemon`), attempt to save a password-authenticated SSH connection. Assert TauTerm does not write any credential to disk. Assert the user is shown a prompt for credentials at connection time and informed that persistence is unavailable. |
| **Expected mitigation** | `LinuxCredentialStore::is_available()` returns `false`; the credential command handler detects this, does not persist, and emits a `credential-prompt` event on each connection. |
| **Priority** | High |
| **Environment** | Requires a controlled D-Bus environment without a running Secret Service provider. |
| **Stub dependency** | `credentials_linux.rs` availability probe is a stub. |

#### SEC-CRED-006

| Field | Value |
|-------|-------|
| **ID** | SEC-CRED-006 |
| **STRIDE** | Information Disclosure |
| **FS requirement(s)** | FS-SSH-011 (TOFU), FS-CRED-006 |
| **Threat** | The `known_hosts` file at `~/.config/tauterm/known_hosts` is writable. A local attacker modifies it to add a rogue key for a target host. TauTerm accepts the rogue key silently on the next connection, bypassing TOFU. |
| **Test method** | Unit test: write a known_hosts file with a known entry for `test.host`. Read it back. Modify the entry to a different key. Assert `known_hosts::lookup` returns `Err(HostKeyMismatch)` on the modified entry and does NOT silently accept it. |
| **Expected mitigation** | `known_hosts.rs` always compares the offered key against the stored entry; a mismatch returns `HostKeyMismatch` regardless of file modification time. File permissions should be 0600 (enforced on write). |
| **Priority** | Critical |
| **Environment** | Unit test. No external dependencies. |
| **Stub dependency** | `known_hosts.rs` is a stub. |

#### SEC-CRED-007

| Field | Value |
|-------|-------|
| **ID** | SEC-CRED-007 |
| **STRIDE** | Information Disclosure |
| **FS requirement(s)** | FS-CRED-004, SEC-CRED-003 (extension) |
| **Threat** | After a successful authentication, the private key file is read into a `String` or `Vec<u8>` and remains in process memory. A memory snapshot of the TauTerm process could recover the key material. |
| **Test method** | Code review: verify that private key bytes are never stored in `SshConnectionConfig`, `Credentials`, or any `#[derive(Debug)]` struct. Verify the file content is passed directly to `russh-keys::load_secret_key()` and not cloned into application state. |
| **Expected mitigation** | Private key file content is read by `russh-keys` at authentication time; the return value is a `PrivateKey` handle owned by `russh`. The raw bytes are not copied into TauTerm state. |
| **Priority** | High |
| **Environment** | Code review. |
| **Stub dependency** | SSH auth implementation. |

#### SEC-CRED-008

| Field | Value |
|-------|-------|
| **ID** | SEC-CRED-008 |
| **STRIDE** | Information Disclosure |
| **FS requirement(s)** | SEC-CRED-003, FS-CRED-004 |
| **Threat** | `LinuxCredentialStore::is_available()` returns `false` but the application still attempts to call `store()` or `get()` on it. The stub implementation either panics or silently succeeds (returning `Ok`), masking the unavailability to callers. |
| **Test method** | Unit test: create a `LinuxCredentialStore` in an environment where D-Bus is unavailable. Call `is_available()` — assert `false`. Call `store()` — assert `Err(CredentialError::Unavailable)`. Call `get()` — assert `Ok(None)` (safe fallback). Call `delete()` — assert `Ok(())` (no-op when not found). |
| **Expected mitigation** | `is_available()` probe is authoritative. Callers check it before `store()`. `get()` and `delete()` fail gracefully when the service is unavailable. |
| **Priority** | High |
| **Environment** | Unit test. |
| **Stub dependency** | `credentials_linux.rs` fully implemented. |

---

### 2.5 WebView / CSP

#### SEC-CSP-001

| Field | Value |
|-------|-------|
| **ID** | SEC-CSP-001 |
| **STRIDE** | Elevation of Privilege |
| **FS requirement(s)** | FS-SEC-001 |
| **Threat** | The WebView CSP is currently `null` in `tauri.conf.json`. Inline scripts (e.g., injected by a compromised dependency or via a prototype pollution attack on the Svelte runtime) execute without restriction, and can invoke arbitrary Tauri commands on behalf of the user. |
| **Test method** | Static review: confirm CSP in `tauri.conf.json` is set to the FS-SEC-001 minimum before the v1 release. Manual test: inject a `<script>alert(1)</script>` tag via browser devtools in the WebView. Confirm it is blocked by CSP. |
| **Expected mitigation** | `tauri.conf.json` configured with: `default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'; connect-src ipc: http://ipc.localhost; img-src 'self' asset: http://asset.localhost`. |
| **Priority** | Critical |
| **Environment** | Static review + manual test on running app. |
| **Stub dependency** | CSP field is currently `null`. Must be configured before this test passes. |

#### SEC-CSP-002

| Field | Value |
|-------|-------|
| **ID** | SEC-CSP-002 |
| **STRIDE** | Elevation of Privilege |
| **FS requirement(s)** | FS-SEC-001 |
| **Threat** | `script-src 'unsafe-eval'` is inadvertently added to the CSP (e.g., required by a future dependency). This allows `eval()`, `Function()`, and similar dynamic code execution, which reintroduces script injection risk even without inline script capability. |
| **Test method** | Automated check: parse `tauri.conf.json` in CI and fail the build if `unsafe-eval` appears in the `script-src` directive. |
| **Expected mitigation** | `unsafe-eval` is permanently absent from the CSP. Dependencies requiring `eval` must be rejected or replaced. |
| **Priority** | Critical |
| **Environment** | CI static check on `tauri.conf.json`. |

#### SEC-CSP-003

| Field | Value |
|-------|-------|
| **ID** | SEC-CSP-003 |
| **STRIDE** | Information Disclosure |
| **FS requirement(s)** | CLAUDE.md (i18n), FS-I18N-001 |
| **Threat** | A Svelte component uses `{@html m.some_key()}` with a message accessor. If a locale catalogue entry contains `<script>` or event handler markup, this is rendered as raw HTML in the WebView and executes. |
| **Test method** | Static analysis: search all `.svelte` files for `{@html` and flag any occurrence where the content originates from a message accessor or user-controlled data. |
| **Expected mitigation** | `{@html}` with message accessors is explicitly forbidden by project conventions. All i18n strings are rendered via `{m.key()}` (text interpolation, not HTML). |
| **Priority** | High |
| **Environment** | Static analysis (`grep` over `src/`). |

#### SEC-CSP-004

| Field | Value |
|-------|-------|
| **ID** | SEC-CSP-004 |
| **STRIDE** | Elevation of Privilege |
| **FS requirement(s)** | FS-SEC-001, ARCHITECTURE.md §8.4 |
| **Threat** | `style-src 'unsafe-inline'` (currently required by Tailwind 4's runtime token injection) could be exploited via CSS injection to exfiltrate sensitive information displayed in the WebView (e.g., clipboard content, tab titles) through `url()` data exfiltration or other CSS side-channel techniques. |
| **Test method** | Threat documentation only at this stage. When Tailwind 4's nonce-based CSP support is available, implement SEC-CSP-004b: verify that `unsafe-inline` is removed from `style-src` and replaced with a nonce policy. |
| **Expected mitigation** | Acknowledged risk; `unsafe-inline` for styles is a temporary necessity. Tightening plan documented in ARCHITECTURE.md §8.4. CSS injection impact is limited by the absence of user-controlled CSS input paths — all styles come from design tokens. |
| **Priority** | Medium |
| **Environment** | Threat assessment only until nonce support is available. |

---

### 2.6 OSC 52 Clipboard

#### SEC-OSC-001

| Field | Value |
|-------|-------|
| **ID** | SEC-OSC-001 |
| **STRIDE** | Information Disclosure |
| **FS requirement(s)** | FS-VT-076 |
| **Threat** | A malicious process in a pane (or a remote SSH server) sends `OSC 52 ; c ; ?` to query the clipboard content. TauTerm responds with the clipboard content injected into the PTY input stream, exfiltrating whatever the user copied most recently. |
| **Test method** | Unit test: call `parse_osc()` with `b"52;c;?"`. Assert the result is `OscAction::Ignore`. Integration test: feed `\x1b]52;c;?\x07` to `VtProcessor::process()`. Assert no data is written to the PTY input. |
| **Expected mitigation** | `parse_osc()` permanently returns `OscAction::Ignore` for OSC 52 read queries (`data_b64 == "?"`). This is hardcoded with no configuration override. |
| **Priority** | Critical |
| **Environment** | Unit test: standard (already partially implemented in `osc.rs`). |

#### SEC-OSC-002

| Field | Value |
|-------|-------|
| **ID** | SEC-OSC-002 |
| **STRIDE** | Tampering |
| **FS requirement(s)** | FS-VT-075 |
| **Threat** | OSC 52 clipboard write is disabled by default, but the global `allow_osc52_write` preference is set to `true`. An SSH session (which should not inherit this setting unless explicitly enabled per-connection) gains the ability to overwrite the system clipboard silently. |
| **Test method** | Unit test: construct a `VtProcessor` with `allow_osc52_write = true` (global) but with the current session being an SSH connection with `per_connection_allow_osc52_write = false`. Feed an OSC 52 write sequence. Assert `OscAction::Ignore` is returned. |
| **Expected mitigation** | The per-connection `allow_osc52_write` flag in `SshConnectionConfig` takes precedence over the global preference for SSH sessions. The `VtProcessor` is configured with the resolved policy at session creation time. |
| **Priority** | High |
| **Environment** | Unit test. Requires the policy resolution logic to be implemented. |

#### SEC-OSC-003

| Field | Value |
|-------|-------|
| **ID** | SEC-OSC-003 |
| **STRIDE** | Tampering |
| **FS requirement(s)** | FS-VT-075 |
| **Threat** | An OSC 52 write payload contains a very large base64-encoded string (e.g., 10 MB). This overwrites the system clipboard with a large payload, causing DoS for other applications relying on the clipboard, or consuming excessive memory during base64 decoding. |
| **Test method** | Unit test: call `parse_osc()` with an OSC 52 payload where the base64-encoded data is 1 MB. Assert either `OscAction::Ignore` (size limit enforced) or that the write is rejected before being sent to the clipboard backend. Verify no OOM. |
| **Expected mitigation** | OSC 52 write payloads are subject to the general 4096-byte OSC sequence limit (FS-SEC-005). This effectively limits clipboard write content to approximately 3000 bytes (base64 overhead). |
| **Priority** | Medium |
| **Environment** | Standard unit test. |

#### SEC-OSC-004

| Field | Value |
|-------|-------|
| **ID** | SEC-OSC-004 |
| **STRIDE** | Denial of Service |
| **FS requirement(s)** | FS-CLIP-001, ARCHITECTURE.md §8.1 |
| **Threat** | The `copy_to_clipboard` IPC command is called directly by the WebView (not via OSC 52) with a multi-megabyte payload. This bypasses the 4096-byte OSC limit and writes an arbitrarily large string to the system clipboard, causing memory allocation pressure or downstream DoS for clipboard consumers. |
| **Test method** | Unit test: call `copy_to_clipboard` with `MAX_CLIPBOARD_LEN + 1` bytes. Assert `Err` with code `CLIPBOARD_TOO_LARGE` is returned immediately — no clipboard write attempted. Implementation in `system_cmds.rs` (test `ipc_clip_001`). |
| **Expected mitigation** | `copy_to_clipboard` validates `text.len() <= MAX_CLIPBOARD_LEN` (16 MiB) before invoking `arboard`. The limit is enforced at the IPC boundary before any heap allocation for the clipboard payload. |
| **Priority** | Medium |
| **Environment** | Standard unit test. No display server required (validation fires before arboard is called). |
| **Status** | Implemented and tested — `ipc_clip_001` passing. |

---

### 2.7 Path and Input Validation

#### SEC-PATH-001

| Field | Value |
|-------|-------|
| **ID** | SEC-PATH-001 |
| **STRIDE** | Elevation of Privilege |
| **FS requirement(s)** | FS-CRED-006, FS-SEC-003 |
| **Threat** | An `SshConnectionConfig` saved with `identity_file = "../../etc/shadow"` (path traversal). At connection time, TauTerm reads the file and passes its content to `russh` as the private key. This reads an arbitrary file as the current user. |
| **Test method** | Unit test: call the connection setup path with `identity_file = Some("../../etc/shadow")`. Assert `TauTermError::new("INVALID_IDENTITY_PATH", ...)` is returned before any file read. Also test `"/etc/shadow"` (absolute path to sensitive file) — this is a regular file but should be rejected if it is not in an expected location (security policy decision: warn, or restrict to `~/.ssh/` and `~/.config/tauterm/` prefixes). |
| **Expected mitigation** | Identity file path is resolved to absolute, checked for `..` components, and verified to point to a regular file. Symlinks may be followed. The path is validated at connection time, not at save time (FS-CRED-006). |
| **Priority** | Critical |
| **Environment** | Standard unit test; no filesystem writes required. |

#### SEC-PATH-002

| Field | Value |
|-------|-------|
| **ID** | SEC-PATH-002 |
| **STRIDE** | Tampering |
| **FS requirement(s)** | FS-SEC-003 |
| **Threat** | A tampered `preferences.json` contains a `shellPath` field (or equivalent in `CreateTabConfig`) pointing to a malicious executable via path traversal (e.g., `"../../../tmp/malicious-shell"`). TauTerm spawns this executable in a PTY. |
| **Test method** | Unit test: load preferences with a shell path containing `../` components. Assert the path is rejected during validation with `TauTermError`. Integration test: attempt to create a tab with a shell path that resolves outside of standard binary directories. |
| **Expected mitigation** | Shell path is resolved to absolute, checked for path traversal, and verified to be an existing regular executable file. Preferences schema validation applies on load (FS-SEC-003). |
| **Priority** | High |
| **Environment** | Standard unit test. |

#### SEC-PATH-003

| Field | Value |
|-------|-------|
| **ID** | SEC-PATH-003 |
| **STRIDE** | Tampering / Information Disclosure |
| **FS requirement(s)** | FS-VT-073 |
| **Threat** | A hyperlink URI (from OSC 8) uses the `javascript:` scheme. When the user Ctrl+Clicks the link, TauTerm calls `open_url("javascript:alert(1)")`. The system browser (or `xdg-open`) interprets and executes this as JavaScript. |
| **Test method** | Unit test: call `validate_url_scheme("javascript:alert(1)")` in `system_cmds.rs`. Assert `TauTermError::new("INVALID_URL_SCHEME", ...)` is returned. Also test: `data:text/html,<script>alert(1)</script>`, `blob:`, `vbscript:`, and a custom scheme `foobar:`. |
| **Expected mitigation** | `validate_url_scheme()` uses a strict allowlist: `http`, `https`, `mailto`, `ssh` only. All other schemes are rejected. This is already partially implemented in `system_cmds.rs`. |
| **Priority** | Critical |
| **Environment** | Standard unit test. Already has partial test coverage in `system_cmds.rs`. |

#### SEC-PATH-004

| Field | Value |
|-------|-------|
| **ID** | SEC-PATH-004 |
| **STRIDE** | Information Disclosure |
| **FS requirement(s)** | FS-VT-073 |
| **Threat** | A hyperlink URI uses the `file://` scheme in an SSH session. Ctrl+Clicking opens a local file that may be sensitive (e.g., `file:///etc/passwd`). In an SSH session, the expectation is that `file://` links point to remote files, but the system browser opens local files. |
| **Test method** | Unit test: verify `validate_url_scheme("file:///etc/passwd")` is rejected. Integration test: in an SSH session, verify that OSC 8 links with `file://` URIs are not rendered as clickable hyperlinks (the VtProcessor should suppress them). |
| **Expected mitigation** | `file://` URIs are rejected for all sessions in `validate_url_scheme()`. The spec (FS-VT-073) permits `file://` only for local PTY sessions — but given the attack surface and the v1 scope, `file://` is entirely disallowed in the initial implementation (conservative approach). If local-session `file://` support is added, the session type must be passed through to the validation layer. |
| **Priority** | High |
| **Environment** | Unit test: immediate. Integration test requires a running SSH session. |

#### SEC-PATH-005

| Field | Value |
|-------|-------|
| **ID** | SEC-PATH-005 |
| **STRIDE** | Tampering |
| **FS requirement(s)** | FS-SEC-003 |
| **Threat** | `preferences.json` on disk has been replaced with a crafted file containing oversized arrays (e.g., `connections: [... 1 000 000 entries ...]`) designed to cause excessive memory allocation or CPU time during deserialization. |
| **Test method** | Unit test: call `PreferencesStore::load_or_default()` with a TOML/JSON file containing an array of 100 000 connection entries. Assert the load completes within 1 second and returns default preferences (array size exceeds configured limit). |
| **Expected mitigation** | Schema validation on load caps the number of connections (suggested: 1000 max). Out-of-range or oversized structures are replaced with defaults (FS-SEC-003). |
| **Priority** | Medium |
| **Environment** | Unit test; no external dependencies. |

---

### 2.8 UI Component Security (Sprint session h)

New components introduced in sprint 2026-04-05 session h expose the following additional attack surfaces. Scenarios below are numbered SEC-UI-001 through SEC-UI-006.

#### SEC-UI-001

| Field | Value |
|-------|-------|
| **ID** | SEC-UI-001 |
| **STRIDE** | Spoofing / Tampering |
| **FS requirement(s)** | FS-SSH-030, FS-SEC-003 |
| **Threat** | A user enters `<script>alert(1)</script>` as a hostname in the ConnectionManager edit form. If the hostname is rendered via `{@html}` anywhere (connection list display, status bar, SSH badge), it executes as JavaScript in the WebView. |
| **Test method** | Unit/component test: save a connection with `host = '<script>alert(1)</script>'`. Render the connection list. Assert that the hostname is rendered as escaped text, not executed script. Verify no `{@html}` is used in ConnectionManager or StatusBar for the host field. |
| **Expected mitigation** | Svelte's template interpolation (`{host}`) escapes HTML by default. No `{@html}` used for user-supplied connection fields. The static security scanner (SEC-CSP-003 pattern) should catch any `{@html}` introduction. |
| **Priority** | High |
| **Environment** | Component test; no network. |

#### SEC-UI-002

| Field | Value |
|-------|-------|
| **ID** | SEC-UI-002 |
| **STRIDE** | Information Disclosure |
| **FS requirement(s)** | FS-CRED-001, FS-CRED-004 |
| **Threat** | The ConnectionManager stores the password entered in the edit form in Svelte component state (a `$state` variable). If the component is inspected via browser devtools or a memory dump, the password is readable. Additionally, if the password value is passed to a `console.log()` or Tauri IPC debug trace, it is disclosed. |
| **Test method** | Code review: verify that the password field value in ConnectionManager is NOT stored in a persistent rune (`$state` at module level). It should be cleared after `provide_credentials` IPC call. Verify no `console.log` of the password value. Verify the `provide_credentials` IPC payload does not appear in any debug event log. |
| **Expected mitigation** | Password captured in a local `$state` variable scoped to the form lifecycle. Cleared immediately after IPC call to `provide_credentials`. No logging at any level. |
| **Priority** | Critical |
| **Environment** | Code review + component test. |

#### SEC-UI-003

| Field | Value |
|-------|-------|
| **ID** | SEC-UI-003 |
| **STRIDE** | Denial of Service |
| **FS requirement(s)** | FS-SEARCH-003 |
| **Threat** | The SearchOverlay passes user input directly to the `search_pane` IPC as a regex string (when regex mode is enabled). A crafted ReDoS pattern (e.g., `(a+)+$`) causes the Rust regex engine to consume excessive CPU, blocking the backend command handler for seconds. |
| **Test method** | Unit test (Rust): call `VtProcessor::search()` with a ReDoS pattern on a 1000-line buffer. Assert completion within 200ms. Verify that the `regex` crate (used in `vt/search.rs`) is configured with a time/complexity limit or uses a linear-time engine. Frontend: verify that the `regex` flag in `SearchQuery` is only sent when the user explicitly enables regex mode toggle. |
| **Expected mitigation** | The `regex` crate in Rust uses a linear-time NFA engine and does not support catastrophic backtracking. No additional throttling required, but the regex flag should default to `false` and require explicit user opt-in. Frontend debounce (≥150ms) on keystrokes additionally limits query frequency. |
| **Priority** | Medium |
| **Environment** | Rust unit test; no network. |

#### SEC-UI-004

| Field | Value |
|-------|-------|
| **ID** | SEC-UI-004 |
| **STRIDE** | Tampering |
| **FS requirement(s)** | FS-PREF-001, FS-SEC-003 |
| **Threat** | The PreferencesPanel sends font family and font size values directly to `update_preferences` IPC. A crafted font family string (`'; DROP TABLE users; --`) or an extreme font size (e.g., 999999999) is persisted and applied, potentially breaking the UI rendering or causing memory issues. |
| **Test method** | Component test: enter a font size of `0`, `-1`, `999999`, and a font family containing `<script>`. Assert that the frontend validates and clamps values before emitting the update event. Assert font family is validated to only contain safe characters (no HTML, no control chars). Also test that the Rust backend validates on receipt (FS-SEC-003). |
| **Expected mitigation** | Frontend clamps font size to [8, 32] per UXD §7.6.3. Font family sanitized to alphanumeric plus common safe chars. Backend schema validation (FS-SEC-003) provides second line of defense. |
| **Priority** | Medium |
| **Environment** | Component test; no network. |

#### SEC-UI-005

| Field | Value |
|-------|-------|
| **ID** | SEC-UI-005 |
| **STRIDE** | Information Disclosure |
| **FS requirement(s)** | FS-CLIP, FS-A11Y-006 |
| **Threat** | The ContextMenu has a "Paste" item. If the component reads clipboard content on menu open (to display a preview or to pre-check content), sensitive clipboard data may be exposed in DOM, accessible to any JavaScript executing in the WebView. |
| **Test method** | Code review + component test: verify that the ContextMenu does NOT call `get_clipboard` or read clipboard content until the user explicitly clicks Paste. The Paste menu item is always enabled regardless of clipboard content (no clipboard read on open). |
| **Expected mitigation** | Clipboard is only read via `get_clipboard` IPC at Paste action time, not at menu render time. The menu item does not display clipboard content. |
| **Priority** | High |
| **Environment** | Component test; code review. |

#### SEC-UI-006

| Field | Value |
|-------|-------|
| **ID** | SEC-UI-006 |
| **STRIDE** | Tampering / Spoofing |
| **FS requirement(s)** | FS-PTY-005 |
| **Threat** | The ProcessTerminatedPane renders the exit code from a `session-state-changed` IPC event payload. If the exit code value is a string or object (due to a type coercion bug or malicious event crafting), interpolating it as `{exitCode}` could produce unexpected rendered content. Extreme values (e.g., `exitCode = 2^53`) may cause display anomalies. |
| **Test method** | Component test: mount `ProcessTerminatedPane` with `exitCode = NaN`, `exitCode = Infinity`, `exitCode = -1`, `exitCode = 2147483647`. Assert that the component renders gracefully (no exception, no `{@html}`, exit code displayed as a bounded number string). Verify TypeScript type safety ensures `exitCode` is always `number`. |
| **Expected mitigation** | TypeScript typing enforces `exitCode: number`. The component renders `{exitCode}` (text interpolation, not `{@html}`). Extreme values are displayed as-is (they are unlikely in practice but not harmful when rendered as text). |
| **Priority** | Low |
| **Environment** | Component test. |

---

## 3. Penetration Test Checklist

This checklist consolidates the manual and integration tests that cannot be fully automated. It must be executed before each major release (MINOR or MAJOR version).

### 3.1 PTY and VT Engine

- [ ] Feed `TIOCSTI`-equivalent input injection sequences through PTY output — confirm no input bypass to the controlling terminal (`TIOCSTI` is blocked on Linux 6.2+ by default; verify the kernel version of target platforms)
- [ ] Run `vttest` (standard VT conformance tool) in a TauTerm pane — verify no crashes, hangs, or assertion failures
- [ ] Feed a corpus of 10 000 randomly mutated terminal streams to `VtProcessor::process()` via the fuzzing target — zero crashes required
- [ ] Verify that `ls -la /proc/self/fd` inside any pane does not show master PTY fds of other panes

### 3.2 SSH

- [ ] Connect to an SSH server presenting an unknown key — verify TOFU prompt is shown with SHA-256 fingerprint and key type
- [ ] Connect to a known host whose key has changed — verify connection is blocked, both fingerprints shown, MITM warning displayed, default action is Reject
- [ ] Connect to a server offering only `ssh-rsa` (SHA-1) — verify deprecation warning is shown in-pane, connection established
- [ ] Verify no SSH agent forwarding socket is created during any SSH session (`ls /tmp/ssh-*` before and after connection)
- [ ] After 90 seconds of network blockage, verify pane transitions to Disconnected state
- [ ] Verify `TERM=xterm-256color`, correct terminal dimensions, and RFC 4254 terminal modes are sent in the PTY request (capture with `ssh -v` or a mock server)

### 3.3 Credentials

- [ ] Inspect `~/.config/tauterm/preferences.json` after saving an SSH connection with password — no password field present
- [ ] Run `secret-tool lookup service tauterm host <hostname>` — credential is present and correct
- [ ] With `RUST_LOG=trace`, connect via SSH password — verify no password appears in log output
- [ ] With no Secret Service provider running, attempt to save a password — verify prompt-each-time behavior and user notification
- [ ] After connecting via SSH password, search process memory for the password string (see SEC-CRED-002 procedure)

### 3.4 WebView / CSP

- [ ] Open browser devtools in TauTerm WebView. Attempt to inject `<script>alert(1)</script>` via the DOM. Confirm CSP blocks execution.
- [ ] Attempt `invoke('send_input', ...)` from browser devtools. Verify it succeeds only for valid `paneId` values (tests both IPC access control and pane ID validation).
- [ ] Search all `.svelte` files for `{@html` — zero occurrences expected with user-controlled or i18n content.
- [ ] Review `tauri.conf.json` CSP: confirm no `unsafe-eval`, no `script-src *`, no `connect-src *`.
- [ ] Review `capabilities/default.json`: confirm only the minimum required capabilities are granted.

### 3.5 OSC 52

- [ ] Feed `\x1b]52;c;?\x07` to a running pane (via `send_input` or by piping through the shell). Verify no clipboard data appears in the PTY input.
- [ ] With `allow_osc52_write = true` globally and `allow_osc52_write = false` on a specific SSH connection, verify the SSH session does not write to clipboard.
- [ ] With `allow_osc52_write = true` (per connection), verify a valid OSC 52 write sequence updates the clipboard.

### 3.6 Path and Input Validation

- [ ] Attempt to save an SSH connection with `identityFile = "../../etc/shadow"` — verify rejection.
- [ ] Attempt `open_url` with `javascript:alert(1)` — verify rejection.
- [ ] Attempt `open_url` with `data:text/html,...` — verify rejection.
- [ ] Attempt `open_url` with a 2049-character URL — verify rejection.
- [ ] Replace `preferences.json` with a tampered file containing `"language": "injected"` — verify fallback to English, no crash.

### 3.7 Dependency Audit

- [ ] Run `cargo audit` — zero known vulnerabilities in the dependency tree.
- [ ] Review `Cargo.toml` for unnecessary features enabled on security-sensitive crates (`russh`, `secret-service`, `portable-pty`). Prefer minimal feature sets.

---

## 4. Security Regression Policy

### 4.1 Classification and Response Time

| Severity | Definition | Response |
|----------|------------|----------|
| **Critical** | Exploitable without user interaction; leads to credential theft, PTY injection, or arbitrary code execution | Halt development; patch within 24 hours; security release |
| **High** | Requires limited user interaction (e.g., clicking a crafted link); significant data exposure or privilege escalation | Patch within 1 week; included in next PATCH release |
| **Medium** | Requires specific conditions; moderate impact | Patch within 4 weeks; included in next MINOR release |
| **Low** | Theoretical or minimal impact | Tracked; addressed in next MINOR release |

### 4.2 Test-as-Regression Policy

Every confirmed security vulnerability resolved in TauTerm MUST produce:

1. A unit or integration test that reproduces the vulnerability before the fix.
2. Confirmation that the test fails on the unfixed code.
3. Confirmation that the test passes after the fix.
4. The test is merged alongside or before the fix — never after.

Security tests are never skipped, removed, or marked `#[ignore]` without explicit written justification from both `security-expert` and `moe`.

### 4.3 Stub Dependencies — Tests Blocked on Implementation

The following security tests cannot currently be executed because the underlying implementation is a stub. They are tracked here and MUST be unblocked before the corresponding feature is declared complete.

| Test ID(s) | Blocked by | Required stub |
|------------|------------|---------------|
| SEC-PTY-005 | PTY not implemented | `LinuxPtySession::write`, `open_session` |
| SEC-SSH-001, SEC-SSH-002 | SSH handshake not implemented | `known_hosts.rs` TOFU logic, `auth.rs` |
| SEC-SSH-003 | Agent forwarding rejection not verifiable | `connection.rs` channel handler |
| SEC-SSH-004 | Algorithm detection not implemented | `algorithms.rs` |
| SEC-SSH-005 | Terminal modes not sent | `connection.rs` PTY request |
| SEC-SSH-006 | Keepalive not implemented | `keepalive.rs` |
| SEC-CRED-001, SEC-CRED-002, SEC-CRED-005 | Secret Service not implemented | `credentials_linux.rs` |
| SEC-CSP-001 | CSP is `null` in `tauri.conf.json` | CSP must be configured |
| SEC-OSC-002 | Per-connection OSC 52 policy resolution not wired | Policy resolver in `VtProcessor` setup |

### 4.4 Continuous Security Checks (CI)

The following checks MUST run on every pull request targeting `main` or `dev`:

| Check | Command | Failure condition |
|-------|---------|-------------------|
| Dependency audit | `cargo audit` | Any known vulnerability at High or Critical severity |
| Clippy (security-relevant lints) | `cargo clippy -- -D warnings` | Any warning |
| CSP static check | Parse `tauri.conf.json`, assert no `unsafe-eval` in `script-src` | `unsafe-eval` present |
| `{@html}` audit | `grep -rn "{@html" src/` | Any match |
| `unwrap()` audit | `grep -rn "\.unwrap()" src-tauri/src/` | Any match outside of explicitly allowed init code (requires allowlist) |

### 4.5 Security Sign-off

A feature that touches any of the following MUST receive explicit sign-off from `security-expert` before merging:

- PTY management (`platform/pty_linux.rs`, `session/`)
- SSH connection flow (`ssh/`)
- Credential storage (`platform/credentials_linux.rs`)
- IPC command handlers (`commands/`)
- VtProcessor OSC/DCS/CSI dispatch (`vt/processor.rs`, `vt/osc.rs`)
- `tauri.conf.json` (any field)
- `capabilities/default.json` (any capability addition)

Sign-off is recorded as a review approval on the pull request with a comment citing the specific test IDs verified.
