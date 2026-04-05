# TODO

## Gaps d'implémentation (analyse docs vs codebase)

~~### Bloquants~~ ✅ *Complétés le 2026-04-05 — voir `docs/test-protocols/report-blocking-ipc-2026-04-05.md`*

~~### Majeurs — Câblage IPC manquant~~ ✅ *Complétés le 2026-04-05*

~~### Majeurs — Fonctionnalités absentes~~ ✅ *Complétés le 2026-04-05 — voir `docs/test-protocols/report-major-sprint-2026-04-05.md`*

~~### Majeurs — Raccourcis clavier~~ ✅ *Complétés le 2026-04-05*

~~### Majeurs — Préférences non câblées~~ ✅ *Complétés le 2026-04-05*

~~### Mineurs~~ ✅ *Complétés le 2026-04-05 — voir `docs/test-protocols/report-mineurs-sprint-2026-04-05.md`*

~~- [ ] **Scrollbar non interactive**~~ ✅ (FS-SB-007)
~~- [ ] **Premier lancement context menu hint**~~ ✅ (FS-UX-002)
~~- [ ] **AppImage non configuré**~~ ✅ (FS-DIST-001 à 006)
~~- [ ] **Strings UI hardcodées**~~ ✅ (FS-I18N-001)
~~- [ ] **`file://` scheme rejeté**~~ ✅ (FS-VT-073)
~~- [ ] **ENV split_pane incomplètes**~~ ✅ *Corrigé le 2026-04-05*
~~- [ ] **Paste confirmation multiline**~~ ✅ (FS-CLIP-009)
~~- [ ] **Tab contrast WCAG AA**~~ ✅ contraste ≥ 4.5:1 (TUITC-UX-060)
~~- [ ] **FS-SSH-013 erratum**~~ ✅ corrigé dans `docs/FS.md`
~~- [ ] **Recherche scrollback cross-row**~~ ✅ jonction cross-row implémentée, `#[ignore]` retiré (SEARCH-SOFT-001)

### Tests manquants (blocages environnement)

~~- [ ] **SecretService integration test**~~ ✅ Tests écrits (`src-tauri/tests/credentials_integration.rs`, SEC-CRED-INT-001 à 005). Environnement Podman fourni (`Containerfile.keyring-test` + `scripts/run-keyring-tests.sh`). Lancer avec `./scripts/run-keyring-tests.sh`.
~~- [ ] **E2E tests**~~ ✅ Specs débloquées : sélecteurs BEM corrigés, attribut `data-screen-generation` pour la synchronisation, `InjectablePtyBackend` + commande `inject_pty_output` derrière le feature flag `e2e-testing` (ADR-0015). `TEST-PTY-RT-002` reécrit pour utiliser `inject_pty_output` au lieu du clavier. Lancer avec `cargo build --features e2e-testing && pnpm wdio`.

---

## Dette architecturale — création du premier onglet

- [ ] **Déplacer la création du premier onglet dans `lib.rs`** — Actuellement, `TerminalView.svelte` détecte un état de session vide au montage et appelle `invoke('create_tab', ...)` pour pallier le démarrage à vide du backend. C'est un stopgap : la responsabilité de l'état initial de la session appartient au backend. La solution correcte est d'appeler `create_tab` dans `setup()` de `lib.rs` (ou via une méthode dédiée sur `SessionRegistry`) avant que la fenêtre ne soit affichée, de façon à ce que `get_session_state` retourne toujours au moins un onglet. Voir le commentaire STOPGAP dans `src/lib/components/TerminalView.svelte`.

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
