<!-- SPDX-License-Identifier: MPL-2.0 -->

# ADR-0024 — Passphrase keychain scope

**Date:** 2026-04-11
**Status:** Accepted

## Context

SSH private keys are often encrypted with a passphrase. When TauTerm opens a
connection that uses pubkey authentication with an encrypted key, the user must
provide the passphrase so the key can be loaded.

The question is how passphrases should be treated with respect to the OS keychain
(Secret Service on Linux — the same store used for connection passwords):

1. **Never store** — always prompt on every connection.
2. **Store only for the current session** — keep in process memory, forget on exit.
3. **Store in keychain if the user opts in** — persist across sessions via the OS
   keychain, prompt only on first use per key file.

The `provide_passphrase` IPC command already accepts a `save_in_keychain: bool`
parameter from the frontend, and `SshPassphraseDialog.svelte` shows a
"Save passphrase in keychain" checkbox when the keychain is available.

The connect task (`src-tauri/src/ssh/manager/connect.rs`) already implements the
full two-phase flow:

1. Before prompting, try to retrieve a stored passphrase from the keychain via
   `credential_manager.get_passphrase(key_path)` and verify it decrypts the key.
   If it does, use it silently.
2. If the keychain has no stored passphrase (or the stored one is stale), emit
   `passphrase-prompt` and wait for the user to respond.
3. If the user provided `save_in_keychain: true` and the passphrase decrypts the
   key successfully, call `credential_manager.store_passphrase()` to persist it.
   A keychain storage failure is a non-fatal warning (logged, not surfaced to the
   user), because the passphrase was still accepted for this session.

## Decision

Passphrases for encrypted SSH private keys are **stored in the OS keychain on an
opt-in basis** — the user must explicitly check "Save passphrase in keychain" in
`SshPassphraseDialog.svelte`.

On subsequent connections to the same key file, the connect task first checks the
keychain. If a valid passphrase is found (and it still decrypts the key), the
user is not prompted. If the stored passphrase is stale (key re-encrypted with a
new passphrase, or keychain entry deleted), the flow falls back to the prompt,
and the user can choose to save again.

The keychain key format is `tauterm:key-passphrase:{identity_file_path}` (see
`credentials.rs::passphrase_key`).

## Alternatives considered

**Never store the passphrase (prompt every time)**

Always prompt on every connection attempt, regardless of history. No keychain
interaction.

This is the most conservative security posture. The passphrase is in memory only
for the duration of the `load_secret_key` call. However, it creates significant
UX friction for users who connect frequently to servers using the same encrypted
key. The flow would prompt on every new tab, every reconnect, and every
application restart. Rejected because the UX cost is high for no security gain
over the opt-in model — a user who does not want the passphrase stored simply
does not check the checkbox.

**Store in process memory for the current application session (not persisted)**

Keep the passphrase in a `Mutex<HashMap<PathBuf, Zeroizing<String>>>` in
`SshManager`. Prompt once per application session per key file; clear on exit.

This is a middle ground: better UX than always-prompt, no persistent exposure
in the keychain. However, it means the user is re-prompted on every application
restart, which is the common case for users who close TauTerm between sessions.
The in-memory storage also adds a retention window: the passphrase is held in
process memory for the lifetime of the application, not just for the duration of
key loading. For users with long-running TauTerm sessions, this is a longer
exposure window than the keychain (which can be locked by the OS session lock).
Not chosen in favour of the opt-in keychain approach.

**Always save in keychain (no opt-in)**

Store the passphrase in the keychain after the first successful use, without
asking the user.

Silent keychain writes violate user expectations and informed consent. A user
who considers their passphrase as ephemeral credentials (e.g., a hardware-backed
key with a passphrase) would not expect TauTerm to persist it. Rejected.

## Rationale for opt-in keychain storage

- **User control:** the user explicitly decides whether to persist the passphrase.
  The default is not to store.
- **Consistency with password flow:** `SshCredentialDialog.svelte` also offers a
  "Save in keychain" checkbox for passwords. The passphrase flow mirrors this
  UX pattern.
- **Security model alignment:** users who chose to encrypt their private key
  typically did so to require active entry of the passphrase. The opt-in model
  respects this intent while offering convenience to those who prefer it.
- **OS keychain delegation:** the OS keychain (Secret Service / GNOME Keyring)
  is protected by the user's session lock and managed by the OS. It is at least
  as secure as the user's session.

## Consequences

**Positive:**
- Users who connect frequently to servers using encrypted keys can opt in to
  keychain storage and avoid repeated prompts.
- Users with strict passphrase policies retain full control — the checkbox is
  off by default and they never need to interact with the keychain.
- The keychain lookup is silent and non-blocking (resolves before the prompt is
  shown), so the common case (stored passphrase, not stale) has zero UX overhead.

**Negative / risks:**
- The passphrase is stored in the keychain as `tauterm:key-passphrase:{path}`,
  where `{path}` is the full identity file path. This key format encodes the
  filesystem path — a minor information leak in the keychain entry name (mitigated
  by the fact that the keychain entry is only readable by the current session).
- Stale passphrase detection relies on `russh::keys::load_secret_key` failing.
  If a key is re-encrypted, TauTerm silently falls through to the prompt after
  a failed keychain verification — the user may be confused if they expected the
  stored passphrase to work. The dialog shows `failed: true` on re-prompt, which
  provides a partial signal.
- `store_passphrase` failure is a non-fatal warning. If the keychain daemon is
  unavailable at storage time (e.g., GNOME Keyring not running), the passphrase
  is used for the current session but not persisted. The user will be prompted
  again next time. This is acceptable — the keychain is a best-effort enhancement,
  not a required component.
