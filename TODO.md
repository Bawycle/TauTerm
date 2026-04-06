# TODO

---

## Audit 2026-04-05 — Lacunes identifiées (FS.md + UXD.md vs code source)

*Source : audit exhaustif par agents spécialisés. Les divergences ont été arbitrées — spec ou implémentation mise à jour selon le verdict.*

---

### Bugs détectés

- [ ] **`StatusBarDimensionsHarness.svelte` — 3 warnings `state_referenced_locally`** (Svelte 5)
  `initCols`, `initRows`, `initDimsVisible` sont référencées comme valeurs initiales hors contexte réactif. Svelte 5 recommande de les capturer dans une closure. Fichier de test uniquement — pas d'impact runtime, mais masque des erreurs de réactivité futures.

### Frontières à risque — `TerminalView.svelte`

*Identifiées lors de l'audit i18n (2026-04-06). Logique de coordination dans un handler de composant racine, non couverte par des tests automatiques.*

- [ ] **`handleContextMenuHintDismiss` — incohérence d'état après dismiss**
  `contextMenuHintDismissed = true` et `contextMenuHintVisible = false` sont mis à jour localement, mais `preferences.appearance.contextMenuHintShown` n'est pas reflété dans l'état local `preferences`. Si `PreferencesPanel` est ouvert après le dismiss, il affiche `contextMenuHintShown: false` au lieu de `true`. Corriger en mettant à jour `preferences` localement après l'IPC `mark_context_menu_used`.

- [ ] **`handleConnectionOpen` — tab orpheline en cas d'échec partiel**
  Séquence `create_tab` → `open_ssh_connection` : si `open_ssh_connection` échoue après que `create_tab` a réussi, un onglet vide est créé sans connexion SSH. Pas de rollback. Comportement en cas d'erreur partielle non spécifié ni testé.

- [ ] **`onMount` — dégradation silencieuse si `get_preferences` échoue**
  Si l'IPC `get_preferences` échoue au démarrage, `preferences` reste `undefined`. Tous les `$derived` qui en dépendent (ex: `activeThemeLineHeight`) retournent `undefined` silencieusement. Définir et tester le comportement de dégradation gracieuse attendu.

---

### Fonctionnalités manquantes — UI / UXD

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
