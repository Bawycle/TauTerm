# TODO

## Gaps d'implémentation (analyse docs vs codebase)

~~### Bloquants~~ ✅ *Complétés le 2026-04-05 — voir `docs/test-protocols/report-blocking-ipc-2026-04-05.md`*

~~### Majeurs — Câblage IPC manquant~~ ✅ *Complétés le 2026-04-05*

~~### Majeurs — Fonctionnalités absentes~~ ✅ *Complétés le 2026-04-05 — voir `docs/test-protocols/report-major-sprint-2026-04-05.md`*

~~### Majeurs — Raccourcis clavier~~ ✅ *Complétés le 2026-04-05*

~~### Majeurs — Préférences non câblées~~ ✅ *Complétés le 2026-04-05*

### Mineurs

- [ ] **Scrollbar non interactive** — affichée (`TerminalPane.svelte`) mais `pointer-events: none`. Non cliquable, non draggable. (FS-SB-007)
- [ ] **Premier lancement context menu hint** — backend prêt (`context_menu_hint_shown`, `mark_context_menu_used`), rien dans le frontend. (FS-UX-002)
- [ ] **AppImage non configuré** — `tauri.conf.json` a `"targets": "all"` mais pas de config spécifique AppImage (pas de `linux > appImage`). (FS-DIST-001 à 006)
- [ ] **Strings UI hardcodées** — ARIA labels et textes dans `TabBar.svelte`, `TerminalPane.svelte`, `TerminalView.svelte` non passés par Paraglide. Viole FS-I18N-001.
- [ ] **`file://` scheme rejeté** — `validate_url_scheme` rejette systématiquement `file://`, y compris pour les sessions locales. (FS-VT-073)
~~- [ ] **ENV split_pane incomplètes**~~ ✅ *Corrigé le 2026-04-05*
- [ ] **Paste confirmation multiline** — pas de dialogue de confirmation quand le texte collé contient des newlines et que bracketed paste est inactif. (FS-CLIP-009 — SHOULD)
- [ ] **Tab contrast WCAG AA** — titre de tab inactif à ≈ 2.5:1, sous le seuil 4.5:1. Décision design requise. (TUITC-UX-060)
- [ ] **FS-SSH-013 erratum** — opcodes VKILL/VEOF inversés dans `docs/FS.md`. Implémentation correcte (VKILL=4, VEOF=5 per RFC 4254) ; corriger le doc.
- [ ] **Recherche scrollback cross-row** — `search_scrollback` cherche ligne par ligne ; un mot à cheval sur deux lignes soft-wrappées n'est pas trouvé. Implémenter la jonction cross-row et retirer `#[ignore]` sur `vt/search.rs::search_soft_wrap_word_spanning_two_rows_is_found` (SEARCH-SOFT-001).

### Tests manquants

- [ ] **SecretService integration test** — round-trip D-Bus requiert keyring daemon actif ; bloqué sur l'environnement.
- [ ] **E2E tests** — `pty-roundtrip.spec.ts` + `tab-lifecycle.spec.ts` bloqués sur le wiring PTY SSH → screen-update → DOM.

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
