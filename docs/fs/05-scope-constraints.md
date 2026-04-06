<!-- SPDX-License-Identifier: MPL-2.0 -->
# Functional Specifications — Scope, Constraints & Review Notes

> Part of the [Functional Specifications](README.md). See also: [00-overview.md](00-overview.md), [01-terminal-emulation.md](01-terminal-emulation.md), [02-ui-navigation.md](02-ui-navigation.md), [03-remote-ssh.md](03-remote-ssh.md), [04-config-system.md](04-config-system.md)

---

## 3.18 FS-DIST: Distribution

| ID | Requirement | Priority |
|----|-------------|----------|
| FS-DIST-001 | TauTerm v1 MUST be distributed as an AppImage. | Must |
| FS-DIST-002 | The AppImage MUST be self-contained: it MUST bundle all application dependencies (Rust runtime, frontend assets, shared libraries not guaranteed to be present on a target system). It MUST NOT require the user to install external packages beyond a standard Linux desktop environment (display server, `libwebkit2gtk-4.1` on Ubuntu 22.04+ or `libwebkit2gtk-4.0` on older distributions). | Must |
| FS-DIST-003 | The AppImage MUST run on the following architectures: x86 (i686), x86_64, ARM32 (armhf), ARM64 (aarch64), RISC-V (riscv64). A separate AppImage binary MAY be produced per architecture. | Must |
| FS-DIST-004 | The AppImage MUST be executable directly after download (no installation step required). A user who downloads and `chmod +x`es the AppImage MUST be able to run TauTerm immediately. | Must |
| FS-DIST-005 | The AppImage SHOULD integrate with the host desktop environment: it SHOULD provide a `.desktop` entry and application icon accessible via the AppImage integration daemon (`appimaged`) or equivalent. | Should |
| FS-DIST-006 | TauTerm release artefacts MUST be cryptographically signed. Each AppImage MUST be accompanied by a detached GPG signature (`.asc`) and a SHA-256 checksum file (`SHA256SUMS`). The `SHA256SUMS` file itself MUST also be GPG-signed. The public signing key MUST be published on a separate trusted channel (project website or a public keyserver). | Must |

**Acceptance criteria:**
- FS-DIST-001: The release artefact is an `.AppImage` file.
- FS-DIST-002: On a clean minimal Linux installation (e.g., Ubuntu Server with a desktop environment added but no TauTerm-specific dependencies), the AppImage runs without prompting for package installation.
- FS-DIST-003: The AppImage (or per-architecture variant) launches and passes a basic smoke test (`pnpm wdio`) on each of the five target architectures.
- FS-DIST-004: `chmod +x TauTerm-x86_64.AppImage && ./TauTerm-x86_64.AppImage` opens the application on a clean x86_64 system.
- FS-DIST-005: After running the AppImage with `appimaged` active, TauTerm appears in the desktop application launcher with its icon.
- FS-DIST-006: Each release includes a `.asc` detached GPG signature and a signed `SHA256SUMS` file alongside every AppImage artefact. Running `gpg --verify TauTerm-x86_64.AppImage.asc TauTerm-x86_64.AppImage` succeeds with the published public key. Running `sha256sum --check SHA256SUMS` passes for every listed artefact.

---

## 4. Out of Scope (v1)

The following are explicitly excluded from TauTerm v1:

| Item | Rationale |
|------|-----------|
| Plugin or extension system | Complexity; no persona requires it in v1. (UR 9) |
| Cloud sync of preferences, themes, or saved connections | Scope control; security implications. (UR 9) |
| Windows and macOS support | Linux-only first version. (UR 9) |
| Session persistence (restoring tabs/panes after restart) | Acknowledged need, deferred to future version. (UR 9) |
| Kitty keyboard protocol | Deferred to future version. v1 uses standard xterm key encoding. Shift+Enter is a known limitation. |
| URXVT mouse encoding (mode 1015) | SHOULD-level; may be deferred if schedule requires. |
| Serial port / local connection types | No persona requires it. |

---

## 5. Domain Constraints

The following constraints arise from the terminal/PTY domain and are non-negotiable. They affect multiple requirements and must be understood by all stakeholders.

| Constraint | Implication |
|------------|-------------|
| PTY is a byte stream with no framing | VT sequences can be split across read boundaries. The parser must handle partial sequences. (Affects FS-VT-005, FS-VT-010) |
| Applications control terminal mode | TauTerm must track all mode state set by applications (cursor mode, mouse reporting, screen buffer, etc.) and behave accordingly. (Affects FS-VT-030–086) |
| SIGWINCH is asynchronous | TauTerm cannot force an application to redraw after a resize. It can only signal the resize and wait. (Affects FS-PTY-009) |
| Alternate screen is opaque | Scrollback, search, and selection operate on the normal screen only while the alternate screen is active. (Affects FS-SB-005, FS-SEARCH-004) |
| SSH adds only transport layer | All VT behavior specifications apply equally to local and SSH sessions. (Affects FS-SSH-003) |
| Character width is a rendering problem | Disagreement between the terminal and the application on character width (e.g., wcwidth) causes cursor positioning errors. TauTerm must use a consistent and up-to-date width table. (Affects FS-VT-011, FS-VT-014) |
| Ctrl+key encoding is lossy | Ctrl+A and Ctrl+Shift+A both produce 0x01 in standard xterm encoding. TauTerm cannot distinguish them at the PTY level. v1 known limitation. (Affects FS-KBD-004, FS-KBD-013) |

---

## 6. Traceability Matrix

This matrix maps every functional specification to its originating user requirement in [UR.md](../UR.md).

| FS Requirement | UR Source |
|----------------|-----------|
| **FS-VT: Terminal Emulation** | |
| FS-VT-001, FS-VT-002 | UR 1 (terminal emulator); Domain expert |
| FS-VT-003, FS-VT-004, FS-VT-005 | UR 1; Domain expert (xterm-256color compatibility) |
| FS-VT-010 – FS-VT-016 | UR 1; Domain expert (character set handling) |
| FS-VT-020 – FS-VT-025 | UR 8 §8.2 (ANSI palette in themes); Domain expert (color codes) |
| FS-VT-030 – FS-VT-034 | UR 1; Domain expert (cursor modes) |
| FS-VT-040 – FS-VT-044 | UR 1; Domain expert (screen modes) |
| FS-VT-050 – FS-VT-054 | UR 1; Domain expert (scrolling regions) |
| FS-VT-060 – FS-VT-062 | UR 4 §4.1 (tab titles); Domain expert (OSC sequences) |
| FS-VT-070 – FS-VT-073 | Domain expert (hyperlinks, security) |
| FS-VT-080 – FS-VT-086 | Domain expert (mouse reporting) |
| FS-VT-090 – FS-VT-093 | UR 4 §4.1 (activity notification); Domain expert (bell) |
| **FS-PTY: PTY Lifecycle** | |
| FS-PTY-001 – FS-PTY-004 | UR 4 §4.1, §4.2 (independent PTY per tab/pane); Domain expert |
| FS-PTY-005 – FS-PTY-006 | Domain expert (exit handling) |
| FS-PTY-007 – FS-PTY-008 | UR 4 §4.1 (close tabs); Domain expert (SIGHUP, confirmation) |
| FS-PTY-009 – FS-PTY-010 | UR 4 §4.2 (pane resize); Domain expert (SIGWINCH) |
| FS-PTY-011 – FS-PTY-014 | Domain expert (environment, shell fallback) |
| **FS-TAB: Multi-Tab** | |
| FS-TAB-001 – FS-TAB-007 | UR 4 §4.1 (multi-tab); UR 6 §6.1 (shortcuts) |
| **FS-PANE: Multi-Pane** | |
| FS-PANE-001 – FS-PANE-006 | UR 4 §4.2 (multi-screen/panes); UR 3.1 (dual modality) |
| **FS-KBD: Keyboard Input** | |
| FS-KBD-001 – FS-KBD-003 | UR 6 (keyboard shortcuts, distinction app vs PTY) |
| FS-KBD-004 – FS-KBD-012 | UR 6 §6.2 (PTY passthrough); Domain expert (key encoding) |
| FS-KBD-013 | Domain expert (known limitation) |
| **FS-CLIP: Clipboard** | |
| FS-CLIP-001 – FS-CLIP-009 | UR 6 §6.3 (clipboard); Domain expert (selection, bracketed paste) |
| **FS-SB: Scrollback** | |
| FS-SB-001 – FS-SB-008 | UR 7 §7.1 (scrollback); Domain expert (buffer behavior) |
| **FS-SEARCH: Search** | |
| FS-SEARCH-001 – FS-SEARCH-007 | UR 7 §7.2 (search in output); Domain expert (search constraints) |
| **FS-NOTIF: Notifications** | |
| FS-NOTIF-001 – FS-NOTIF-004 | UR 4 §4.1 (activity notification); UR 2.2 (Jordan — notifications) |
| FS-NOTIF-005 | UR 4 §4.1 (tab bar overflow, activity notification) |
| **FS-SSH: SSH Sessions** | |
| FS-SSH-001 – FS-SSH-003 | UR 9 §9.1 (SSH integration) |
| FS-SSH-010 – FS-SSH-014 | UR 9 §9.1 (connection state, notification); Domain expert (lifecycle, PTY request); Security review (deprecated algorithms) |
| FS-SSH-020 – FS-SSH-022 | UR 9 §9.1 (interruption detection); Domain expert (keepalive, drop detection) |
| FS-SSH-030 – FS-SSH-034 | UR 9 §9.2 (saved connections) |
| FS-SSH-040 – FS-SSH-042 | UR 9 §9.4 (reconnection) |
| **FS-CRED: Credentials** | |
| FS-CRED-001 – FS-CRED-002 | UR 9 §9.3 (security, keychain, key paths) |
| **FS-THEME: Theming** | |
| FS-THEME-001 – FS-THEME-002 | UR 8 §8.1 (default theme) |
| FS-THEME-003 – FS-THEME-007 | UR 8 §8.2 (user-created themes) |
| FS-THEME-008 – FS-THEME-009 | UR 8 §8.3 (design tokens) |
| FS-THEME-010 | UR 8 §8.2 (user-created themes, font/line height customisation) |
| **FS-PREF: Preferences** | |
| FS-PREF-001 | UR 5 §5.1 (persistence) |
| FS-PREF-002 – FS-PREF-004 | UR 5 §5.2 (preferences UI, sections) |
| FS-PREF-005 | UR 6 §6.1 (Ctrl+, shortcut); UR 3.1 (dual modality) |
| FS-PREF-006 | UR 5 §5.2; UR 7 §7.1 (scrollback config); Domain expert (cursor, bell, delimiters) |
| **FS-A11Y: Accessibility** | |
| FS-A11Y-001 – FS-A11Y-004 | CLAUDE.md (WCAG 2.1 AA, contrast, targets, keyboard, color-only) |
| FS-A11Y-005 | UR 3.1 (dual modality); UR 3.3 (PTY exception) |
| FS-A11Y-006 | UR 3.1 (dual modality); UR 3.2 (discoverable UI) |
| FS-A11Y-007 | UR 8 §8.2 (theme editing); UR 5 §5.2 (preferences UI accessibility) |
| **FS-UX: UX Cross-Cutting** | |
| FS-UX-001 | UR 2 (personas, esp. Sam §2.3); UR 3.2 (discoverable UI) |
| FS-UX-002 | UR 2 §2.3 (Sam — no config required for basic use); UR 3.2 (discoverable UI) |
| **FS-SEC: Security Hardening** | |
| FS-SEC-001 – FS-SEC-005 | Security review; CLAUDE.md (security constraints) |
| **FS-VT-063** | Security review (title read-back injection) |
| **FS-VT-075 – FS-VT-076** | Security review (OSC 52 clipboard control) |
| **FS-CRED-003 – FS-CRED-006** | Security review (credential lifecycle) |
| **FS-I18N: Internationalisation** | |
| FS-I18N-001 – FS-I18N-006 | UR 10 §10.1 (language support, selection, persistence, fallback) |
| FS-I18N-007 | UR 10 §10.2 (PTY locale env vars not modified) |
| **FS-DIST: Distribution** | |
| FS-DIST-001 – FS-DIST-005 | UR 11 §11.1 (AppImage, self-contained, multi-arch) |
| FS-DIST-006 | UR 11 §11.1 (release artefact integrity); Security review |

---

## 7. Review Notes

> **Reviewer:** user-rep (User Representative)
> **Date:** 2026-04-04
> **Status:** Open issues requiring team discussion

### 7.1 Open Issues

**RN-001: FS-CLIP-003 — RESOLVED.**
Default delimiter set specified (Option B): space and punctuation delimiters; `/`, `.`, `-`, `_`, `:`, `@`, `=` are non-delimiters by default. User-configurable. Decision: accepted 2026-04-04.

**RN-002: FS-SSH-011 — RESOLVED.**
Prompt: plain-language explanatory text required (Option B). Fingerprint format: SHA-256 only (Option A). Key change behavior: block by default with explanation and actionable instructions (Option A + explanation). Decision: accepted 2026-04-04.

**RN-003: RESOLVED.**
Added FS-UX-001 (section 3.15): cross-cutting requirement on error message quality — plain language, actionable next step, technical details as secondary element only. Decision: accepted 2026-04-04.

**RN-004: RESOLVED.**
Added FS-UX-002 (section 3.15): on first launch, a non-intrusive hint signals the context menu (right-click); disappears after first right-click. Option B. Decision: accepted 2026-04-04.

**RN-005: FS-VT-091 — RESOLVED.**
`SHOULD` → `MUST`: visual bell is the mandatory default. Decision: accepted 2026-04-04.

**RN-006: FS-SB-002 — RESOLVED.**
No artificial maximum: user-configurable without upper limit. Preferences UI displays real-time memory estimate per pane. Default remains 10,000 lines. Decision: Option D, accepted 2026-04-04.

**RN-007: RESOLVED.**
UR.md fully renumbered: sections 4–9 corrected, all subsections aligned. Traceability matrix updated accordingly. Decision: accepted 2026-04-04.

**RN-008: FS-TAB-006 — RESOLVED.**
Interaction specified: double-click for inline editing (primary) + context menu "Rename" (discoverable). Enter confirms, Escape cancels. Clearing label reverts to process-driven title. Option C. Decision: accepted 2026-04-04.

**RN-009: FS-KBD-003 — RESOLVED.**
Ctrl+Tab / Ctrl+Shift+Tab retained as fixed defaults (universal convention). Split and pane navigation shortcuts: defaults now resolved from UXD.md §11.2 (Ctrl+Shift+D, Ctrl+Shift+E, Ctrl+Shift+Right/Left/Up/Down, Ctrl+Shift+Q). F2 added as default shortcut for inline tab rename. Decision: accepted 2026-04-04, updated 2026-04-04.

**RN-010: FS-SSH-010 Closed state — RESOLVED.**
Added definition for the Closed state (intentional/clean termination) and clarified distinction from Disconnected (unexpected interruption). Decision: accepted 2026-04-04.

**RN-011: FS-THEME-010 line height — RESOLVED.**
Added FS-THEME-010 (Should): `--line-height-terminal` token is user-overridable; UI chrome line height is fixed. Decision: accepted 2026-04-04.

**RN-012: FS-NOTIF-005 scroll arrow badges — RESOLVED.**
Added FS-NOTIF-005 (Should): scroll arrows display a dot badge when scrolled-out tabs have pending notifications; bell takes priority over activity. Decision: accepted 2026-04-04.

**RN-013: FS-A11Y-007 theme editor isolation — RESOLVED.**
Added FS-A11Y-007 (Must): theme editor chrome renders with active Umbra system tokens; only the preview area reflects the custom theme. Decision: accepted 2026-04-04.

---

## 8. Security Review Notes

> **Reviewed by:** security-expert
> **Date:** 2026-04-04
> **Scope:** Full FS document, all sections

### 8.1 Changes Applied (Critical and High)

The following security issues were identified and corrected directly in this document:

| ID | Severity | Issue | Section Modified |
|----|----------|-------|------------------|
| C1 | Critical | OSC 52 clipboard manipulation not addressed -- enables clipboard exfiltration and poisoning by remote/malicious programs | Added FS-VT-075, FS-VT-076 (section 3.1.9) |
| C2 | Critical | Title read-back sequences (CSI 21t, OSC queries) not blocked -- enables input injection attacks | Added FS-VT-063 (section 3.1.7) |
| C3 | Critical | FS-CRED incomplete -- no spec for in-memory credential lifetime, logging prohibition, keychain absence fallback, or identity path validation | Added FS-CRED-003 through FS-CRED-006 (section 3.11) |
| C4 | Critical | TOFU host key change specification too weak -- did not require connection blocking, fingerprint comparison, or safe default action | Strengthened FS-SSH-011 (section 3.10.2) |
| H1 | High | OSC 8 URI validation insufficient -- did not address file: in SSH context, URI length, control characters, or unknown schemes | Strengthened FS-VT-073 (section 3.1.8) |
| H2 | High | Bracketed paste did not specify stripping of end-sequence within pasted text -- enables paste injection | Strengthened FS-CLIP-008 (section 3.6) |
| H4 | High | No CSP specification -- WebView open to XSS | Added FS-SEC-001 (section 3.15) |
| H5 | High | PTY fd O_CLOEXEC not specified -- fd leak to child processes | Added FS-SEC-002 (section 3.15) |
| H3 | High | Preferences file not validated -- potential injection via tampered config | Added FS-SEC-003 (section 3.15) |

### 8.2 Medium Items (Recommended for Architecture/Implementation Phase)

These items do not require FS-level specification changes but SHOULD be addressed during architecture design or implementation:

| ID | Issue | Recommendation |
|----|-------|----------------|
| M1 | TIOCSTI injection | On Linux kernels < 6.2, a child process can inject keystrokes into the terminal via `ioctl(TIOCSTI)`. The architecture should document whether TauTerm relies on kernel 6.2+ `TIOCSTI` restriction or implements its own mitigation (e.g., verifying input source). |
| M2 | Security event logging | SSH authentication failures, host key changes (accepted or rejected), and rejected OSC sequences (OSC 52, CSI 21t) should be logged at INFO level for forensic purposes. No credentials in logs (per FS-CRED-004). |
| M3 | OSC rate-limiting | Title change sequences (OSC 0/1/2) and hyperlink creation (OSC 8) should be rate-limited similarly to bell (FS-VT-092) to prevent UI disruption attacks and memory exhaustion from rapid hyperlink generation. |
| M4 | Known-hosts file validation | The known-hosts file should be parsed defensively: invalid lines skipped with a warning, file corruption should not prevent new connections (degrade to prompting). |
| M5 | SSH agent forwarding | Explicitly declared out of scope (FS-SEC-004). If added in a future version, it requires per-connection opt-in with clear risk disclosure to the user. |

### 8.3 Low Items (Hardening Suggestions)

| ID | Issue | Recommendation |
|----|-------|----------------|
| L1 | VT sequence size limit | Addressed by FS-SEC-005 (4096 byte limit on individual OSC/DCS). Implementation should also enforce a limit on the number of CSI parameters (suggested: 32). |
| L2 | Scrollback memory | Addressed by FS-SB-002 (RN-006 resolved 2026-04-04): preferences UI displays real-time memory estimate. |
| L3 | Environment variable sanitization | FS-PTY-011/012 specify what to set/inherit, but the child process environment should be constructed from scratch (allow-list) rather than inherited wholesale. Dangerous variables (`LD_PRELOAD`, `LD_LIBRARY_PATH`) MUST NOT be inherited. This is an architecture decision. |

### 8.4 Open Questions (Require Team Decision)

1. **OSC 52 write -- RESOLVED.** Per-connection setting (Option B): each saved connection has its own OSC 52 write toggle; unsaved local sessions use the global default (disabled). FS-VT-075 updated. Decision: accepted 2026-04-04.
2. **Known-hosts file location — RESOLVED.** Separate file `~/.config/tauterm/known_hosts` (OpenSSH-compatible format). TauTerm never touches `~/.ssh/known_hosts`. Preferences UI offers import from `~/.ssh/known_hosts`. FS-SSH-011 updated. Option B. Decision: accepted 2026-04-04.
3. **Minimum SSH key strength — RESOLVED.** Added FS-SSH-014: non-blocking, dismissible warning in the pane when deprecated algorithms are negotiated (`ssh-rsa` SHA-1, `ssh-dss`). Connection proceeds. Option C. Decision: accepted 2026-04-04.
