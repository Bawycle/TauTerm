<!-- SPDX-License-Identifier: MPL-2.0 -->

# Test Report — Blocking Items & IPC Wiring
**Date:** 2026-04-05
**Sprint:** Blocking items + missing IPC wiring (TODO.md §Bloquants + §Majeurs — Câblage IPC manquant)

---

## Summary

| Category | Result |
|---|---|
| Rust tests (nextest) | **235 / 235 passed**, 1 skipped |
| Frontend tests (vitest) | **522 / 522 passed**, 31 todo |
| TypeScript/Svelte check | **0 errors**, 6 pre-existing warnings |
| Rust clippy `-D warnings` | **clean** |

---

## Items Implemented

### Bloquants

#### 1. Recherche scrollback (FS-SEARCH-001 à 007)
- **Rust** : `vt/search.rs` — implémentation complète de `search_scrollback()` avec matcher literal + regex (crate `regex`), encodage wide-char via `cells_to_text()`, fonction `build_matcher()` sécurisée (SEC-BLK-001 : ReDoS non possible avec la crate `regex`)
- **Rust** : `commands/input_cmds.rs` — `search_pane` câblé sur `SessionRegistry::search_pane()` qui délègue à `search_scrollback`
- **Sécurité** : queries > 1 KiB rejetées ; regex compilée avant la boucle ; pas d'exposition de données brutes dans SearchMatch
- **Tests** : 14 nouveaux tests Rust dans `vt/search.rs` (SEARCH-001 à 007, cas limites, sécurité)

#### 2. SSH auth interactive (FS-SSH-010 à 014)
- **Frontend** : `TerminalView.svelte` — listeners `ssh-state-changed`, `host-key-prompt`, `credential-prompt` câblés
- **Frontend** : nouveaux composants `SshHostKeyDialog.svelte` (TOFU + MITM warning) et `SshCredentialDialog.svelte` (password prompt)
- **Sécurité (SEC-BLK-004)** : `host` affiché depuis `config.host`, jamais depuis les données serveur SSH
- **Sécurité (SEC-BLK-015)** : titres rendus via `{host}` text interpolation, jamais `{@html}`
- **Tests** : 25 tests vitest (SshHostKeyDialog + SshCredentialDialog)

#### 3. SSH reconnexion UI (FS-SSH-040 à 042)
- **Frontend** : `TerminalPane.svelte` — bouton "Reconnect" affiché quand `sshState === 'Disconnected'` ou `sshState === 'Closed'`
- **Frontend** : appel `invoke('reconnect_ssh', { tabId, paneId })` sur clic
- **Rust** : `ssh_cmds.rs` — `reconnect_ssh` command handler câblé sur `SshManager::reconnect()` avec récupération du `registry`

#### 4. Mouse reporting (FS-VT-080 à 086)
- **Frontend** : `keyboard.ts` — `encodeMouseEvent()` implémenté pour modes X10 (mode 9), Normal (1000), Button-event (1002), Any-event (1003), encodages X10 et SGR (mode 1006)
- **Frontend** : `TerminalPane.svelte` — handlers `onmousedown`, `onmouseup`, `onmousemove` redirigés vers PTY quand un mode mouse reporting est actif ; sélection de texte inhibée dans ce cas
- **Frontend** : scroll SGR encodé (`ESC[<64;col;rowM` up, `ESC[<65;col;rowM` down)
- **Sécurité** : mode state per-pane (pas d'état global partagé entre panes)

#### 5. Bracketed paste (FS-CLIP-008)
- **Frontend** : `TerminalPane.svelte` — wrapping `ESC[200~…ESC[201~` quand DECSET 2004 actif
- **Sécurité (SEC-BLK-012)** : strip de `\x1b[201~` dans le payload AVANT le wrapping, pour empêcher l'injection via clipboard malveillant

### Majeurs — Câblage IPC manquant

#### 6. Ctrl+Shift+V paste (FS-CLIP-005, FS-KBD-003)
- **Frontend** : `TerminalView.svelte` — interception dans `handleGlobalKeydown`, lecture clipboard via `invoke('read_clipboard')`, envoi via le flux bracketed paste
- **Keyboard** : `keyboard.ts` — `Ctrl+Shift+letter` ne produit plus de séquence C0 (fix `!shiftKey` dans le handler `ctrlKey`) — les application shortcuts `Ctrl+Shift+{T,W,F,V}` ne pollueront jamais le PTY

#### 7. Notifications d'activité tabs (FS-NOTIF-001 à 004)
- **Frontend** : `TerminalView.svelte` — listener `notification-changed` câblé, état `tabNotifications` mis à jour
- **Frontend** : `TabBar.svelte` — badges alimentés depuis `tabNotifications`

#### 8. Pane focus → `set_active_pane` (FS-PANE-005)
- **Frontend** : `TerminalPane.svelte` — `onclick` et `onfocus` appellent `invoke('set_active_pane', { tabId, paneId })`

#### 9. Credential store SSH (FS-CRED-001, FS-CRED-005)
- **Rust** : `credentials.rs` — `CredentialManager` injecté dans l'état Tauri via `manage()` dans `lib.rs`
- **Rust** : `ssh_cmds.rs` — lookup keychain avant connexion ; credentials null si non trouvés (le connect task demandera via `credential-prompt`)
- **Sécurité (SEC-BLK-007)** : credentials jamais loggés, jamais inclus dans les events IPC

#### 10. OSC title update (FS-VT-060 à 062, FS-TAB-006)
- **Rust** : `vt/processor/dispatch.rs` — OSC 0/1/2 appellent `proc.set_title_changed(title)` avec filtrage des caractères de contrôle C0/C1
- **Rust** : `session/pty_task.rs` et `session/ssh_task.rs` — `take_title_changed()` consommé après chaque `process()` ; `registry.update_pane_title()` → `emit_session_state_changed` avec `PaneMetadataChanged`
- **Rust** : `session/registry.rs` — `update_pane_title()` method + `self_ref: Weak<Self>` pour passer `Arc<SessionRegistry>` aux tasks
- **Sécurité (SEC-BLK-015)** : filtrage C0/C1 côté Rust ; `{title}` text interpolation côté frontend (jamais `{@html}`)

#### 11. Focus events mode 1004 (FS-VT-084)
- **Rust** : `events/types.rs` — champ `focus_events: bool` ajouté à `ModeStateChangedEvent`
- **Frontend** : `TerminalPane.svelte` — `onfocus`/`onblur` envoient `\x1b[I` / `\x1b[O` quand mode 1004 actif

#### 12. DECKPAM (FS-KBD-010)
- **Frontend** : `keyboard.ts` — `keypadToVtSequence()` implémenté : quand `deckpam` actif, les touches Numpad envoient les séquences SS3 (`\x1bO{p..y}`)
- **Frontend** : `TerminalPane.svelte` — `appKeypad` state mis à jour depuis `mode-state-changed`, `keypadToVtSequence` appelé en priorité dans le handler keydown

---

## Bonus items implémentés

- **ENV split_pane incomplètes** — `registry.rs` : `split_pane` forward maintenant DISPLAY, WAYLAND_DISPLAY, DBUS_SESSION_BUS_ADDRESS (comme `create_tab`)

---

## Security findings mitigated

| ID | Risque | Mitigation |
|---|---|---|
| SEC-BLK-004 | TOFU host spoofing | `host` affiché depuis `config.host` uniquement |
| SEC-BLK-007 | Credential leakage in events | Credentials jamais sérialisés dans les events IPC |
| SEC-BLK-012 | Bracketed paste injection | `ESC[201~` strippé du payload avant wrapping |
| SEC-BLK-015 | XSS via OSC title | Filtrage C0/C1 Rust + text interpolation Svelte |

---

## Protocols written (Phase 1)

- `docs/test-protocols/functional-blocking-ipc-wiring.md` — 55 scénarios sur 12 items
- `docs/test-protocols/security-blocking-ipc-wiring.md` — 20 scénarios sécurité (SEC-BLK-001 à 020)
- `docs/test-protocols/blocking-major-ipc-items.md` — 47 scénarios (version parallèle)
- `docs/test-protocols/security-blocking-major-ipc-items.md` — protocole sécurité complémentaire

---

## Pre-existing warnings (not introduced by this sprint)

- `ConnectionManager.svelte:193` — state_referenced_locally (pre-existing)
- `ConnectionManager.svelte:495` — unused CSS selector (pre-existing)
- `ContextMenu.svelte:80` — state_referenced_locally (pre-existing)
- `ProcessTerminatedPane.svelte:103/108/112` — unused CSS selectors (pre-existing)

---

## Known remaining gaps (still in TODO.md)

The following items were out of scope for this sprint and remain in TODO.md:

- Split layout arborescent (FS-PANE-001, FS-PANE-003)
- Tab drag-and-drop (FS-TAB-005)
- Tab inline rename (FS-TAB-006)
- Close confirmation dialog (FS-PTY-008)
- ConnectionManager dans l'UI principale (FS-SSH-031, FS-SSH-032)
- Theme editor (FS-THEME-003 à 006)
- Double-click word / triple-click line select (FS-CLIP-002)
- Primary selection X11 (FS-CLIP-004)
- Login shell premier tab (FS-PTY-013)
- Raccourcis pane non interceptés (FS-KBD-003)
- Raccourcis non persistés (FS-KBD-002)
- Préférences terminal incomplètes (FS-PREF-003, FS-PREF-006)
- IPC type drift Rust ↔ TypeScript
- Scrollbar interactive, context menu hint, AppImage, i18n strings hardcodées, etc.
