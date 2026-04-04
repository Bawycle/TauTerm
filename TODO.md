# TODO

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
