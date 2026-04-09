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

### Performance — pistes à valider par mesure

Ces deux optimisations ont été explicitement mises hors scope de la campagne de performance initiale. Le benchmark `write_1mb_ascii` établit la baseline actuelle à **~19.6 MiB/s** — à comparer après chaque changement.

- [ ] **P5 — Flat buffer pour `ScreenBuffer`**
  Remplacer `Vec<Vec<Cell>>` par un unique `Vec<Cell>` de taille `rows × cols` avec accès par `row * cols + col`. Élimine un niveau d'indirection et améliore la cache locality lors des lectures séquentielles (snapshot, partial update). Estimation d'impact : 3–10× sur `write_1mb_ascii`. **Risque** : breaking change sur toutes les APIs qui exposent `&[Cell]` par ligne (`get_row`, `scroll_up`, etc.) — refactor non trivial.

- [ ] **P12 — Rendu canvas pour `TerminalPaneViewport`**
  Remplacer le rendu DOM cellule-par-cellule (~11 000 `<span>` pour 220×50) par un `<canvas>` dessiné via Canvas 2D API **en TypeScript** (pas Rust/WASM). Le bottleneck est le layout/reflow DOM, pas le calcul — Canvas 2D l'élimine. Rust/WASM n'apporterait rien ici : les appels `fillText`/`fillRect` doivent traverser la frontière WASM→JS de toute façon, annulant tout gain ; un software renderer pixel-par-pixel en Rust serait une réécriture complète sans garantie de gain sur WebKitGTK (WebGL/WebGPU instable). Estimation d'impact : latence de rendu 2–5× inférieure sur grandes grilles. **Risque** : perte de la sélection texte OS et des lecteurs d'écran natifs — nécessite une couche d'accessibilité séparée (ARIA live region ou `<textarea>` invisible). Changement architectural majeur côté frontend.

---

### Détachement d'onglet et déplacement inter-fenêtres

Permet de détacher un onglet de sa fenêtre pour en créer une nouvelle (comme Firefox), et de déplacer un onglet d'une fenêtre à l'autre par drag-and-drop.

#### Détachement d'onglet → nouvelle fenêtre

- [ ] Commande Tauri `detach_tab(tab_id)` : crée une nouvelle fenêtre Tauri, transfère la session PTY existante (sans la fermer ni la recréer) dans le registre de la nouvelle fenêtre, et ferme l'onglet dans la fenêtre d'origine
- [ ] Exposer le détachement dans le menu contextuel de l'onglet ("Détacher dans une nouvelle fenêtre")
- [ ] Raccourci clavier configurable (non assigné par défaut)
- [ ] Cas limite : détacher le dernier onglet d'une fenêtre doit fermer la fenêtre d'origine après ouverture de la nouvelle

#### Déplacement d'onglet entre fenêtres (drag-and-drop)

- [ ] Drag initié depuis la tab bar : détecter un drag qui sort de la tab bar vers une zone hors fenêtre → déclencher `detach_tab` et ouvrir une nouvelle fenêtre positionnée au curseur (comme Firefox)
- [ ] Drop sur une tab bar d'une autre fenêtre : protocole de transfert inter-fenêtres (Tauri multi-window messaging ou IPC dédié) pour déplacer la session PTY sans interruption
- [ ] Indicateur visuel pendant le drag (ghost tab, zone de drop highlight sur les autres tab bars)
- [ ] Cas limite : drop annulé (Escape ou release hors cible valide) → aucun changement d'état

#### Backend Rust

- [ ] Abstraire le registre de sessions (`SessionRegistry`) pour qu'un **ensemble de sessions** (toutes les panes de l'onglet — PTY locaux et SSH) soit transférable entre contextes de fenêtre sans être détruit/recréé
- [ ] Le transfert opère au niveau de l'onglet, pas de la session individuelle : toutes les panes (y compris les layouts split) migrent atomiquement
- [ ] Les sessions SSH (connexion TCP + canal SSH + PTY distant) doivent être traitées au même titre que les PTY locaux : le canal reste ouvert, seule l'appartenance de la session à une fenêtre change dans le registre
- [ ] Événement IPC `tab-transferred { tab_id, source_window_id, target_window_id }` émis après transfert réussi (discriminated payload, `#[serde(tag = "type")]`)
- [ ] Tests nextest : transfert d'un onglet multi-panes, transfert avec session SSH active, détachement du dernier onglet, annulation de transfert

#### Contraintes

- Aucune session (PTY local ou SSH) **ne doit être interrompue** pendant le transfert — pas de kill/respawn, pas de déconnexion SSH
- L'état VT complet de chaque pane (screen buffer, scrollback, cursor) doit être préservé intégralement
- Le layout des panes (splits, ratios) doit être reproduit à l'identique dans la fenêtre de destination
- Chaque fenêtre Tauri doit avoir un identifiant stable pour le routage des événements IPC

---

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
