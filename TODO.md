# TODO

---

## Fonctionnalités manquantes — UI / UXD

- [ ] **Mode plein écran** (FS-FULL-001 à FS-FULL-010, UXD §7.22)
  Spécifié dans `docs/fs/06-fullscreen.md` et `docs/uxd/03-components.md §7.22`. À implémenter :
  - Commande Tauri `toggle_fullscreen` (Rust) + IPC frontend (`invoke`)
  - Raccourci F11 intercepté dans `handleGlobalKeydown` (avant PTY) — ajout dans `defaultShortcuts` et `FS-KBD-003`
  - Bouton UI discoverable dans la tab bar (badge `Minimize2`, opacity 0.7 au repos)
  - Masquage/rappel de la tab bar et de la status bar en plein écran (hover 4px bord haut, auto-hide 1,5 s)
  - SIGWINCH envoyé à tous les PTY après stabilisation de la géométrie (debounce existant FS-PTY-009)
  - Persistance de l'état dans les préférences (FS-FULL-009)
  - Token `--z-fullscreen-chrome: 45` déjà ajouté dans `docs/uxd/02-tokens.md`
  - Annonce `aria-live="polite"` pour les lecteurs d'écran

- [ ] **Distribution — signing GPG + SHA256SUMS** (FS-DIST-006)
  Aucun script de signing dans la CI/CD. Implémenter dans le pipeline de release : génération de `SHA256SUMS`, signature GPG, publication des artefacts signés.

---

## P0 — Bloquants release


### Tests et CI

- [ ] **Mettre en place le pipeline CI GitHub Actions**
  - Jobs minimum : `cargo clippy -- -D warnings`, `cargo nextest run`, `pnpm check`, `pnpm vitest run`
  - Déclencheur : push sur `dev` et `main`, PR vers `main`

---

## P1 — Sprint suivant la release

### Architecture frontend

- [x] **Décomposer `TerminalView.svelte` (1315L) et `TerminalPane.svelte` (1438L)**
  - Stores créés : `state/session.svelte.ts`, `state/ssh.svelte.ts`, `state/notifications.svelte.ts`, `state/preferences.svelte.ts`, `state/scroll.svelte.ts`
  - Wrappers IPC créés : `ipc/commands.ts`, `ipc/events.ts`, `ipc/errors.ts`
  - Composables créés : `composables/useTerminalView.svelte.ts`, `composables/useTerminalPane.svelte.ts`
  - Résultat : TerminalView.svelte 32L script + TerminalPane.svelte 130L script (< 250L chacun)
### UX/UI


---

## P2 — Roadmap

### Sécurité

- [ ] **`MAX_FIELD_LEN` hostname trop permissif** (`connection_cmds.rs:57`) — 10 000 octets vs DNS max 253
  - Définir `MAX_HOSTNAME_LEN = 253`, `MAX_USERNAME_LEN = 255`
- [ ] **Probe SecretService avec `EncryptionType::Plain`** (`credentials_linux.rs:60`)
  - Utiliser `EncryptionType::Dh` pour le probe de disponibilité aussi
- [ ] **Validation SSH path incohérente** — `validate_identity_file_path` (light) vs `validate_ssh_identity_path` (stricte)
  - Unifier : appeler `validate_ssh_identity_path` dès la sauvegarde
- [ ] **`unsafe { set_var }` dans tests hors modules `platform/`** (`connection_cmds.rs:167,170`, `preferences/schema.rs:428,435`)
  - Documenter explicitement la dépendance à nextest dans le commentaire SAFETY ou déplacer dans des helpers platform

### Rust — conventions

- [ ] **`char as u8` dans `dispatch.rs:25,31`** — remplacer par `u8::try_from(c)` (convention CLAUDE.md)
  - La garde `is_ascii()` est présente donc pas de bug, mais violation de convention
- [ ] **Commentaire incorrect dans `dispatch.rs`** sur DCS/DECRQSS
  - `unhook` est un no-op pur — le commentaire "handled in unhook" est faux, le corriger

### Documentation

- [ ] **Synchroniser `docs/arch/02-backend-modules.md §3.2`** avec le code réel
  - Ajouter : `ssh_task.rs`, `security_load.rs`, `security_static_checks.rs`, `pty_injectable.rs`, `validation.rs`
- [ ] **Synchroniser `docs/arch/03-ipc-state.md §4.2`** avec les commandes enregistrées
  - Ajouter : `set_active_tab`, `duplicate_connection` ; retirer : `update_connection`

### UX/UI

- [ ] **Placeholder `'Select…'` hardcodé** dans `Dropdown.svelte:39` — passer par Paraglide
- [ ] **Fallback `rgba(239, 68, 68, 0.1)`** dans `SshHostKeyDialog.svelte:113` — remplacer par `var(--color-error-bg)`
- [ ] **Ajouter variante `alertdialog`** à `Dialog.svelte` pour les confirmations destructives (ARIA)
- [ ] **`prefers-reduced-motion`** manquant sur la transition `fade` du banner d'erreur dans `TerminalView.svelte:1148`
- [ ] **`Dropdown.svelte` : `id` prop sans uid fallback** — appliquer le même pattern que `TextInput.svelte`
- [ ] **Définir `--z-fullscreen-chrome: 45`** dans `src/app.css` (token spécifié dans `docs/uxd/02-tokens.md §3.10`, absent du CSS)

---

## P3 — Backlog technique

- [ ] **CSP `font-src`** — ajouter directive explicite `font-src 'self' asset: http://asset.localhost`
- [ ] **Limite du nombre de connexions sauvegardées** (SEC-PATH-005) — prévoir max ~1000 pour prévenir DoS via prefs malformées
- [ ] **Code erreur `INVALID_PANE_ID`** trompeur dans `ssh_prompt_cmds.rs` — exposer `NO_PENDING_CREDENTIALS` distinct
- [ ] **`tracing::warn!`** pour host key Mismatch accepté dans `ssh_prompt_cmds.rs` (SEC-SSH-CH-004)
- [ ] **Tests E2E split pane** — feature split-pane sans test de comportement UI complet
- [ ] **Tests E2E recherche UI** — boucle search-query → backend → highlight → navigation non exercée en E2E
- [ ] **Tests unitaires events IPC dans TerminalPane** — actuellement E2E-deferred ; résoudre le problème de mock `listen()`

---

## Backlog

### Claude Code Agent Teams — multi-pane support

**Condition préalable : [anthropics/claude-code#26572](https://github.com/anthropics/claude-code/issues/26572)**

Claude Code expose actuellement le multi-pane aux agents uniquement via tmux et iTerm2. Une proposition d'extension (`CustomPaneBackend`) définit un protocole JSON-RPC 2.0 permettant à n'importe quel terminal de s'enregistrer comme backend de panes. Ce ticket n'est pas encore fusionné.

**Si et quand ce ticket est implémenté**, implémenter le support dans TauTerm :

- [ ] Daemon Rust exposant le protocole `CustomPaneBackend` (JSON-RPC 2.0, stdio ou socket Unix)
  - `initialize` — handshake et identification du contexte courant
  - `spawn_agent(argv, cwd, env, metadata)` — ouvrir un nouvel agent dans un pane
  - `write(context_id, data)` — envoyer des données à stdin d'un pane
  - `capture(context_id, lines?)` — lire le scrollback buffer
  - `kill(context_id)` — fermer un pane
  - `list()` — lister les contextes actifs
  - `context_exited` (push event) — notifier Claude Code quand un contexte se termine
- [ ] Primitives de gestion de panes côté backend Rust (split, resize, kill, scrollback)
- [ ] Enregistrement automatique de `CLAUDE_PANE_BACKEND` au lancement de TauTerm
- [ ] Tests d'intégration du protocole (nextest)

**Bénéfice :** TauTerm devient un terminal de première classe pour Claude Code Agent Teams, sans dépendre de tmux ou iTerm2.
