# TODO

---

## Fonctionnalités manquantes — UI / UXD

- [ ] **Distribution — signing GPG + SHA256SUMS** (FS-DIST-006)
  Aucun script de signing dans la CI/CD. Implémenter dans le pipeline de release : génération de `SHA256SUMS`, signature GPG, publication des artefacts signés.

---

## P0 — Bloquants release


### Tests et CI

- [ ] **Mettre en place le pipeline CI GitHub Actions**
  - Jobs minimum : `cargo clippy -- -D warnings`, `cargo nextest run`, `pnpm check`, `pnpm vitest run`
  - Déclencheur : push sur `dev` et `main`, PR vers `main`

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

---

### Claude Code Agent Teams — tmux control mode (alternative interim)

**Contexte :** En attendant que `CustomPaneBackend` soit implémenté et fusionné dans Claude Code (voir ticket ci-dessus), Claude Code utilise tmux sur Linux pour afficher les agents dans des panes séparés. Sans intégration, tmux tourne *dans* TauTerm comme dans n'importe quel émulateur — double couche de multiplexing, barre de statut tmux visible, keybindings en conflit.

**Solution :** Implémenter le **tmux control mode** (`tmux -CC`). Dans ce mode, tmux ne dessine plus sa propre UI — il envoie des messages structurés (protocole DCS) à l'émulateur, qui crée ses propres panes natifs en réponse. C'est le mécanisme qu'iTerm2 utilise sur macOS.

Référence : `man tmux`, section *CONTROL MODE*. Précédent : [iTerm2 tmux integration](https://iterm2.com/documentation-tmux-integration.html).

- [ ] Parser le protocole DCS de contrôle tmux (`\eP...ST`, messages `%begin`/`%end`, `%output`, `%window-add`, `%pane-*`, etc.)
- [ ] Mapper les événements tmux control mode sur les primitives TauTerm (tab/pane split, resize, close, scrollback)
- [ ] Détecter automatiquement le control mode au lancement d'une session tmux dans TauTerm
- [ ] Tests d'intégration (nextest) couvrant les messages control mode essentiels

**Bénéfice :** Les panes Claude Code Agent Teams s'affichent comme des panes natifs TauTerm — pas de double multiplexing, UX cohérente. Remplacé par `CustomPaneBackend` si/quand ce dernier est disponible.
