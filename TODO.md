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
