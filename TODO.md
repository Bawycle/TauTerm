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

## Audit 2026-04-05 — Lacunes identifiées (FS.md + UXD.md vs code source)

*Source : audit exhaustif par agents spécialisés. Les divergences ont été arbitrées — spec ou implémentation mise à jour selon le verdict.*

---

### Bugs actifs

~~- [ ] **`processor.rs:269` — combining/zero-width chars** (FS-VT-012/013)~~ ✅ `write_char` : width=0 append au graphème précédent sans avancer le curseur. Tests : `combining_char_attaches_to_previous_cell_no_cursor_advance`, `combining_char_at_column_zero_does_not_panic`.

~~- [ ] **CSS invalide sur curseur underline** (UXD §7.3.1)~~ ✅ `--cursor-top` CSS custom property inline, `calc(var(--cursor-top) + 1lh - ...)` dans `.terminal-pane__cursor--underline`.

~~- [ ] **Tokens CSS inexistants dans le banner SSH disconnected** (UXD §7.5.2)~~ ✅ `--spacing-*` → `--space-*`, `--color-surface-overlay` → `--color-bg-overlay`, `--color-on-accent` → `--term-fg`, `--color-accent-hover` → `--color-accent` + `filter:brightness(1.15)`.

~~- [ ] **BEL `0x07` ignoré dans le processeur VT** (FS-VT-090)~~ ✅ `register_bell()` avec rate-limit 100ms, event `bell-triggered` depuis `pty_task.rs`. Tests : `bel_sets_bell_pending`, `bel_rate_limited_second_immediate_bell_ignored`.

---

### Fonctionnalités manquantes — VT Parser

~~- [ ] **CSI S/T (scroll up/down)** (FS-VT-052)~~ ✅ Bras `([], 'S')` et `([], 'T')` câblés dans `dispatch.rs`, respectent DECSTBM. Tests : `csi_scroll_up_moves_content`, `csi_scroll_down_moves_content`.

~~- [ ] **OSC 8 hyperlinks — rendu frontend + Ctrl+Click** (FS-VT-070–073)~~ ✅ Complet : `CellUpdate.hyperlink`, propagé dans `screen.ts`, classe `.--hyperlink { cursor: pointer }`, Ctrl+Click → `invoke('open_url')`. Validation scheme déléguée au backend (`system_cmds.rs`).

~~- [ ] **OSC 52 clipboard write — forwarding vers backend** (FS-VT-075)~~ ✅ `allow_osc52_write` (défaut `false`), `take_osc52_write()`, event `osc52-write-requested`. 3 tests.

~~- [ ] **DECSCUSR — propagation de la forme du curseur** (FS-VT-030)~~ ✅ `cursor_shape` + `cursor_shape_changed` dans `VtProcessor`, event `cursor-style-changed` émis depuis `pty_task.rs`. Tests : `decscusr_sets_cursor_shape_and_flags_change`, `decscusr_same_value_does_not_set_changed_flag`.

~~- [ ] **`cursorBlinkMs` préférence non câblée** (FS-VT-032)~~ ✅ Prop `cursorBlinkMs` propagée depuis préférences. `$effect` réactif recrée le `setInterval` avec cleanup.

---

### Fonctionnalités manquantes — Session / Scrollback

~~- [ ] **`scrollback_lines` préférence non câblée dans VtProcessor** (FS-SB-002)~~ ✅ `VtProcessor::new(cols, rows, scrollback_lines)` — lu depuis `PreferencesStore` dans `create_tab`/`split_pane`. Test : `scrollback_limit_from_constructor_is_respected`.

~~- [ ] **Fermer le dernier tab → fermer la fenêtre** (FS-TAB-008)~~ ✅ `getCurrentWindow().close()` dans `doCloseTab` quand `tabs.length === 0`.

~~- [ ] **Émission backend BackgroundOutput / ProcessExited** (FS-NOTIF-001/002)~~ ✅ `is_active_pane()` dans registry, BackgroundOutput émis sur output de pane inactive, ProcessExited émis à fin de boucle PTY. Limitation : exit code = -1 (sentinelle "inconnu") — `PtySession` n'expose pas `wait()`. Dette documentée.

~~- [ ] **Search match highlighting dans la grille de cellules** (FS-SEARCH-006)~~ ✅ Props `searchMatches`/`activeSearchMatchIndex` propagées TerminalView→SplitPane→TerminalPane. Sets `"row:col"` O(1) pour le rendu. Classes `--search-match` / `--search-active` avec tokens.

---

### Fonctionnalités manquantes — SSH

~~- [ ] **Keyboard-interactive auth** (FS-SSH-012)~~ ✅ `authenticate_keyboard_interactive` dans `auth.rs`, intégré dans `try_authenticate` (pubkey → KI → password). 10 rounds max, réponse par mot de passe si prompt contient "password".

~~- [ ] **Deprecated SSH algorithm warning** (FS-SSH-014)~~ ✅ `algorithms.rs` implémenté : détection ssh-rsa/ssh-dss dans `check_server_key`, event `ssh-warning { pane_id, algorithm, reason }`. Limitation : russh 0.60 n'expose pas les algos négociés du handshake — détection basée sur la clé présentée par le serveur. Frontend banner SSH warning à câbler.

~~- [ ] **Keepalive configurable par connexion** (FS-SSH-020)~~ ✅ `keepalive_interval_secs: Option<u64>` + `keepalive_max_failures: Option<u32>` dans `SshConnectionConfig`, lus dans `connect_task` avec fallback constants. 4 tests.

~~- [ ] **Duplication de connexion sauvegardée** (FS-SSH-033)~~ ✅ Backend : `duplicate_connection` commande Tauri + `PreferencesStore::duplicate_connection()`, label suffixé " (copy)". Frontend : à câbler dans `ConnectionManager.svelte`. 4 tests.

~~- [ ] **Séparateur visuel SSH à la reconnexion** (FS-SSH-042, UXD §7.19)~~ ✅ Event `ssh-reconnected { pane_id, timestamp_ms }` émis après reconnexion. Frontend doit écouter et insérer une ligne separator dans le scrollback (à câbler dans TerminalPane.svelte).

~~- [ ] **Import `~/.ssh/known_hosts` dans TauTerm known hosts** (FS-SSH-011)~~ ✅ `lookup_with_system_fallback()` : TauTerm autoritatif, `~/.ssh/known_hosts` en read-only comme fallback Unknown. Entrées hachées ignorées (non supportées). 6 tests.

---

### Fonctionnalités manquantes — Sécurité / Credentials

~~- [ ] **Zeroize credentials après auth** (FS-CRED-003)~~ ✅ `ZeroizeOnDrop` sur `Credentials` + `drop(credentials)` anticipé après auth dans `manager.rs`.

---

### Fonctionnalités manquantes — Préférences / Thèmes

~~- [ ] **Line height terminal configurable** (FS-THEME-010)~~ ✅ `line_height: Option<f32>` dans `UserTheme` (Rust + TS), `--line-height-terminal` inline sur viewport, champ number 1.0–2.0 dans PreferencesPanel.

~~- [ ] **Theme editor : preview temps réel** (UXD §7.20.5)~~ ✅ Zone preview avec fond, 16 couleurs ANSI fictives et curseur block. `previewStyle` réactif via `$derived.by`.

~~- [ ] **Theme editor : color picker visuel** (UXD §7.20.3)~~ ✅ `<input type="color">` (swatch 44×32px) + TextInput hex côte à côte pour les 16 couleurs.

~~- [ ] **Theme editor : contrast advisory** (UXD §7.20.4)~~ ✅ `src/lib/utils/contrast.ts` — algo WCAG 2.1 complet. Alerte `role="alert"` si ratio < 4.5:1.

~~- [ ] **Theme editor : isolation preview du chrome** (FS-A11Y-007, UXD §7.20)~~ ✅ Variables `--preview-*` injectées uniquement sur `.theme-preview`. Chrome du panneau continue à référencer les tokens système.

---

### Fonctionnalités manquantes — UI / UXD

~~- [ ] **Tab bar scroll horizontal** (UXD §6.2, §12.2)~~ ✅ `ResizeObserver` + `canScrollLeft`/`canScrollRight`, boutons ChevronLeft/Right, scroll 120px smooth, badge `--bell`/`--output` sur onglets masqués avec notification.

~~- [ ] **Pane border pulse — activity indicator** (UXD §7.2.1)~~ ✅ `borderPulse` state, listener `notification-changed`, priorité exit > bell > output. `@media (prefers-reduced-motion: reduce)` supprime les transitions.

~~- [ ] **Copy flash animation** (UXD §7.12)~~ ✅ `selectionFlashing` 80ms après copy, classe `--selected-flash` avec `--term-selection-flash`.

~~- [ ] **Visual bell flash** (UXD §7.11)~~ ✅ `bellFlashing` state + `.terminal-pane--bell-flash` (80ms, `--color-indicator-bell`). Audio 440Hz via `AudioContext` pour `BellType::Audio/Both`. Listener `bell-triggered` + `cursor-style-changed` câblés.

~~- [ ] **Status bar — bouton Settings + dimensions terminal** (UXD §6.4, DIV-UXD-008)~~ ✅ Bouton Settings (Lucide 14px, i18n) + `{cols}×{rows}` en `--font-mono-ui/--font-size-ui-xs/--color-text-tertiary`. Props propagées via `ondimensionschange`.

~~- [ ] **Pane divider double-click reset ratio** (UXD §7.2)~~ ✅ `ondblclick` → `dragRatio = 0.5` dans `SplitPane.svelte`.

~~- [ ] **Context menu : shortcut hints right-aligned** (UXD §7.8.1)~~ ✅ Prop `shortcuts?: Partial<Record<string, string>>`, label+shortcut `justify-between`, shortcut en `--font-size-ui-sm / --font-mono-ui / --color-text-tertiary`.

- [ ] **Distribution — signing GPG + SHA256SUMS** (FS-DIST-006)
  Aucun script de signing dans la CI/CD. Implémenter dans le pipeline de release : génération de `SHA256SUMS`, signature GPG, publication des artefacts signés.

---

### Corrections de divergences — implémentation à aligner sur la spec

*Ces items sont des divergences arbitrées où la spec l'emporte. L'implémentation doit être corrigée.*

~~- [ ] **`--color-tab-inactive-fg` → neutral-400 (`#9c9890`)** (DIV-UXD-002)~~ ✅ `app.css` : `var(--color-neutral-400)`.

~~- [ ] **Status bar background → `--color-bg-base`** (DIV-UXD-003)~~ ✅ `StatusBar.svelte` : `var(--color-bg-base)`.

~~- [ ] **Status bar layout — restructurer left/right** (DIV-UXD-004 + DIV-UXD-005)~~ ✅ Left : shell name + CWD monospace tronqué. Right : indicateur SSH. Zone center et processTitle supprimés. (Settings + cols×rows = DIV-UXD-008, séparé.)

~~- [ ] **Pane divider hit area → `--size-divider-hit` (8px)** (DIV-UXD-006)~~ ✅ `var(--size-divider-hit)` sur les deux axes dans `SplitPane.svelte`.

~~- [ ] **Scrollbar fade-out → Svelte `transition:fade`** (DIV-UXD-007)~~ ✅ `transition:fade={{ duration: 300 }}` sur la scrollbar dans `TerminalPane.svelte`.

~~- [ ] **Pane shortcuts configurables via `KeyboardPrefs.bindings`** (DIV-FS-003, FS-KBD-002)~~ ✅ 7 actions pane (`split_pane_h/v`, `close_pane`, `navigate_pane_*`) dans `defaultShortcuts`, `matchesShortcut(effectiveShortcut(...))` remplace les comparaisons hardcodées.

~~- [ ] **`TextInput.svelte` focus : `ring-2` → `outline-2`** (DIV-UXD-009, UXD §8.2)~~ ✅ `focus-visible:outline-2 focus-visible:outline-(--color-focus-ring) focus-visible:outline-offset-[-2px]`.

~~- [ ] **`Tooltip.svelte` délai → 300ms** (DIV-UXD-011, UXD §7.10)~~ ✅ Default 600→300ms. `TabBar.svelte` conserve `delayDuration={300}` explicite (utilise `Tooltip.Root` Bits UI directement, pas notre wrapper).

~~- [ ] **Dialog destructif : focus initial sur Cancel** (DIV-UXD-012, UXD §7.9.3)~~ ✅ Prop `onopenautoFocus` sur `Dialog.svelte`, `buttonRef` bindable sur `Button.svelte`. Dialog fermeture tab : focus Cancel via `e.preventDefault(); cancelBtn?.focus()`.

~~- [ ] **First-launch hint : délai 2s + transition fade-in** (DIV-UXD-013, UXD §7.13)~~ ✅ `setTimeout(2000ms)` + `transition:fade={{ duration: 300 }}` dans `TerminalView.svelte`.

~~- [ ] **`Toggle.svelte` transition → token `--duration-base`** (DIV-UXD-015)~~ ✅ `duration-(--duration-base)`.

~~- [ ] **`ScrollToBottomButton.svelte` icône → `--size-icon-md` (16px)** (DIV-UXD-016)~~ ✅ `size="var(--size-icon-md)"`.

~~- [ ] **Pane active border 2px + compensation 1px transparent sur inactive** (DIV-UXD-017, UXD §6.6)~~ ✅ `border: 2px solid` uniforme dans `TerminalPane.svelte`, couleur différenciée via `.terminal-pane--active`.

~~- [ ] **`ScrollToBottomButton` + Scrollbar : transitions Svelte** (DIV-UXD-018)~~ ✅ `transition:fade={{ duration: 150 }}` sur ScrollToBottomButton (wrapper div), `transition:fade={{ duration: 300 }}` sur scrollbar. CSS `transition: opacity` supprimé dans `ScrollToBottomButton.svelte`.

~~- [ ] **Occurrences résiduelles `--umbra-*` dans UXD.md** (suite DIV-UXD-001)~~ ✅ Vérification faite : la seule occurrence est une phrase de prose historique (ligne 126), non une référence token. Aucune correction nécessaire.

---

### Tests manquants — fichiers d'intégration Rust à créer

Ces fichiers sont mentionnés dans les protocoles de test mais n'existent pas.

~~- [ ] **`src-tauri/tests/preferences_roundtrip.rs`** (TEST-PREF-001/002)~~ ✅ 21 tests : round-trip JSON↔struct, champs inconnus ignorés, `load_or_default` robuste sur fichier vide/invalide/absent.

~~- [ ] **`src-tauri/tests/preferences_schema_validation.rs`** (TEST-PREF-002)~~ ✅ 26 tests : valeurs limites numériques, variants enum inconnus → fallback, fuzz-minimal (XML, binaire, BOM, chaîne 10ko…).

~~- [ ] **`src-tauri/tests/session_registry_topology.rs`** (TEST-IPC-*)~~ ✅ 21 tests : 100 IDs distincts sans collision, arbres 2/3/4 niveaux, `close_pane`, `reorder_tab`, sérialisation round-trip.

~~- [ ] **`src-tauri/tests/ipc_type_coherence.rs`** (TEST-IPC-001–004)~~ ✅ 40 tests : tous types IPC sérialisables, discriminated unions, SEC-IPC-002 (`TauTermError` typée sur PaneNotFound/TabNotFound/SshError/…).

---

### Tests manquants — cas spécifiques à ajouter

~~- [ ] **`ssh/auth.rs` — zéro test** (TEST-SSH-UNIT-003)~~ ✅ 5 tests de logique pure (sélection branches, mapping prompts KI). Tests complets nécessitent serveur live (documenté dans le protocole fonctionnel).

~~- [ ] **Keepalive avec mock temporel** (TEST-SSH-UNIT-004)~~ ✅ La logique des 3 misses est dans `russh` — pas de timer applicatif à mocker. `classify_disconnect_reason()` extraite en fonction pure, 4 tests couvrant Closed/Disconnected/Auth/ConnectionLost.

~~- [ ] **SEC-IPC-002 — IDs invalides dans les handlers Tauri**~~ ✅ Couvert dans `ipc_type_coherence.rs` (stream H) : `PaneNotFound` → `INVALID_PANE_ID`, `TabNotFound` → `INVALID_TAB_ID`, tous convertis en `TauTermError` typée.

~~- [ ] **SEC-IPC-004 — Payload oversized sur `save_connection`**~~ ✅ Validation dans `save_connection` : hostname/username > 10 000 chars → `TauTermError { code: "VALIDATION_ERROR" }`. 4 tests.

~~- [ ] **TEST-VT-022 — Search sur alternate screen → 0 résultats**~~ ✅ `search()` retourne `Vec::new()` quand `alt_active`. Test `test_vt_022_search_on_alternate_screen_returns_zero_results` dans `vt/search.rs`.

~~- [ ] **Combining chars dans `processor.rs`** (FS-VT-012/013)~~ ✅ Tests `combining_char_attaches_to_previous_cell_no_cursor_advance` + `combining_char_at_column_zero_does_not_panic` ajoutés en stream B.

~~- [ ] **BEL rate-limiting ≤ 1/100ms** (FS-VT-092)~~ ✅ Test `bel_rate_limited_second_immediate_bell_ignored` ajouté en stream B.

~~- [ ] **SEC-SPRINT-008 — split_pane 50 niveaux → pas de stack overflow**~~ ✅ Test `sec_sprint_008_50_nested_splits_no_stack_overflow` dans `session_registry_topology.rs`.

~~- [ ] **OSC 52 per-connection policy** (SEC-OSC-002)~~ ✅ Couvert par `osc52_write_blocked_by_default_policy` + `osc52_write_forwarded_when_policy_allows` dans `processor/tests.rs` (stream C).

~~- [ ] **Fuzz target cargo-fuzz sur VtProcessor** (SEC-PTY-008)~~ ✅ `src-tauri/fuzz/fuzz_targets/vt_processor.rs` créé. `cargo +nightly fuzz build vt_processor` vert. Feature `fuzz-testing` dans Cargo.toml. Commande : `cargo +nightly fuzz run vt_processor -- -max_total_time=86400`.

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
