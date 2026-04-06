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

### VT Parser — séquences manquantes

- [ ] **ICH / DCH / IL / DL / ESC M** — vim/neovim cassé en édition
  - `CSI @ Ps` (ICH — insert N chars), `CSI P Ps` (DCH — delete N chars)
  - `CSI L Ps` (IL — insert N lines), `CSI M Ps` (DL — delete N lines)
  - `ESC M` (RI — Reverse Index : scroll inverse, utilisé par vim `<C-y>` et less)
  - Tester avec `vim --noplugin` : insertion en mode `i` ne doit pas produire d'artefacts
- [ ] **DECAWM (mode `?7`)** — tmux et htop désactivent le wrap auto, TauTerm l'ignore
  - Ajouter `decawm: bool` dans `ModeState`, appliquer dans `write_char`

### Rust — violations CLAUDE.md actives

- [ ] **`closed_tab_id` absent du payload `SessionStateChangedEvent`** — `src-tauri/src/events/types.rs`
  - Ajouter `closed_tab_id: Option<TabId>` dans la variante `TabClosed` côté Rust
  - Supprimer l'heuristique de fallback dans `TerminalView.svelte` (lignes 311–316)
  - Violation directe règle IPC events CLAUDE.md

### UX/UI — accessibilité et tokens

- [ ] **Bouton Settings 24×24px** dans `StatusBar.svelte` — violation WCAG 2.5.5 (min 44×44px)
  - Agrandir la zone de clic via padding, sans nécessairement agrandir l'icône
- [ ] **`--color-bg-input` non défini** dans `src/app.css` — fond d'input silencieusement transparent
  - Définir le token avec la valeur sémantique correcte dans la couche `component`

### Tests et CI

- [ ] **Mettre en place le pipeline CI GitHub Actions**
  - Jobs minimum : `cargo clippy -- -D warnings`, `cargo nextest run`, `pnpm check`, `pnpm vitest run`
  - Déclencheur : push sur `dev` et `main`, PR vers `main`

---

## P1 — Sprint suivant la release

### VT Parser — correctness

- [ ] **DECSC/DECRC ne restaure pas `attrs` et `charset_slot`** — champs `#[allow(dead_code)]` non câblés
  - `CursorPos` doit sauvegarder et restaurer les attributs courants + charset slot
- [ ] **FS-VT-086 — mode mouse non réinitialisé à la sortie de l'alt screen**
  - Dans `leave_alternate()`, forcer `self.modes.mouse_reporting = MouseReportingMode::None`
  - Sinon, souris captive si une app crash sans envoyer `?1000l`
- [ ] **Underflow `u16` sans garde zéro** dans `dispatch.rs` et `processor.rs`
  - Expressions `p.rows - 1` / `p.cols - 1` sur `u16` sans vérification `> 0`
  - Remplacer par `p.rows.saturating_sub(1)` ou ajouter guard `if p.rows > 0`
- [ ] **Backpressure absente sur la PTY read loop** — `session/pty_task.rs` ligne 15
  - Une app produisant du volume élevé (`yes`, `seq 1 1000000`) inonde le frontend d'events IPC
  - Implémenter coalescing temporel ou rate limiting sur les émissions `screen-update`

### Architecture frontend

- [ ] **Décomposer `TerminalView.svelte` (1315L) et `TerminalPane.svelte` (1438L)**
  - Extraire les stores manquants : `state/session.svelte.ts`, `state/ssh.svelte.ts`, `state/notifications.svelte.ts`, `state/preferences.svelte.ts`, `state/scroll.svelte.ts`
  - Créer les wrappers IPC : `ipc/commands.ts`, `ipc/events.ts`, `ipc/errors.ts`
  - Cible : aucun composant > 250L de logique réactive (spec `docs/arch/05-frontend.md §11.2`)
- [ ] **`mouse_reporting`/`mouse_encoding` : `String` libres dans `ModeStateChangedEvent`**
  - Remplacer par des enums Rust sérialisables (`#[serde(rename_all = "camelCase")]`)
  - Supprimer la conversion manuelle dans `build_mode_state_event()`

### UX/UI

- [ ] **Z-index hardcodés dans 7 fichiers** — remplacer par tokens `z-(--z-xxx)` (Tailwind 4)
  - `ContextMenu.svelte` : `z-[30]` → `z-(--z-dropdown)`
  - `Dropdown.svelte` : `z-[30]` → `z-(--z-dropdown)`
  - `Dialog.svelte` : `z-[49]` → `z-(--z-modal-backdrop)`, `z-[50]` → `z-(--z-modal)`
  - `PreferencesPanel.svelte` : `z-[49]` / `z-[50]` idem
  - `Tooltip.svelte` : `z-[60]` → `z-(--z-tooltip)`
- [ ] **`top:44px` / `bottom:28px` hardcodés** dans `.terminal-view__search-container` (`TerminalView.svelte:1223,1226`)
  - Remplacer par `var(--size-tab-height)` et `var(--size-status-bar-height)`
- [ ] **`ScrollToBottomButton` : 33×33px** — incohérence entre spec §11.5 (44px) et §7.22.4 (33px)
  - Trancher en faveur de WCAG 2.5.5 (44px) et mettre à jour §7.22.4
- [ ] **`aria-label` TerminalPane non différencié** — multi-panes illisible pour les lecteurs d'écran
  - Passer un prop `paneNumber` et générer `aria-label="Terminal {N}"`
- [ ] **`role="region"` → `role="complementary"`** dans `ConnectionManager.svelte:204` (spec §11.3)
- [ ] **~25 occurrences `text-[Npx]`** hardcodées — remplacer par `text-(--font-size-ui-*)`
  - `text-[13px]` → `text-(--font-size-ui-base)`, `text-[12px]` → `text-(--font-size-ui-sm)`, etc.
  - Fichiers : `ConnectionManager`, `PreferencesPanel`, `Dialog`, `TextInput`, `Dropdown`, `ContextMenu`, `Tooltip`
- [ ] **Implémenter SSH Deprecated Algorithm Banner** (UXD §7.21)
  - Le type IPC `dismiss_deprecated_algorithm_banner` existe dans `ipc/types.ts` mais aucun composant ne l'affiche
- [ ] **Implémenter SSH Reconnection Separator** (UXD §7.19)
  - Séparateur visuel injecté dans le scrollback à la reconnexion
- [ ] **Middle-click tab close** (UXD §7.1.2)
  - `onmousedown` avec `button === 1` dans `TabBar.svelte`

### Tests

- [ ] **Créer `src-tauri/tests/vt_processor_integration.rs`** — référencé dans `TESTING.md §14.3` mais inexistant
  - Pipe-pair remplaçant PTY, séquences multi-morceaux, bloc >4096 octets, resize mid-stream
- [ ] **Tests directs pour `vt/charset.rs`, `vt/modes.rs`, `vt/sgr.rs`**
  - `charset.rs` : vérifier les 27 mappings DEC Special Graphics
  - `modes.rs` : tester chaque mode DECSET/DECRST individuellement
- [ ] **Éliminer les 11× `browser.pause()` en E2E** — remplacer par `waitUntil` sur conditions DOM observables
  - Priorité : pauses longues dans `ssh-connection-rollback.spec.ts` (300ms) et `tab-lifecycle.spec.ts` (500ms)

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
