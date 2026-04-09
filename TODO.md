# TODO

---

## Fonctionnalités manquantes — UI / UXD

- [ ] **Distribution — signing GPG + SHA256SUMS** (FS-DIST-006)
  Aucun script de signing dans la CI/CD. Implémenter dans le pipeline de release : génération de `SHA256SUMS`, signature GPG, publication des artefacts signés.

- [ ] **Accessibilité — `aria-controls` et `role="tabpanel"` manquants sur la tab bar** (WCAG 4.1.2)
  Spécifié dans `docs/uxd/05-accessibility.md §11.3` mais non implémenté. `TabBarItem.svelte` a `role="tab"` + `aria-selected` mais manque `aria-controls={panelId}`. Aucun `role="tabpanel"` + `aria-labelledby` n'existe dans `TerminalPane.svelte` ou le conteneur de pane.
  Actions requises : ajouter `aria-controls` sur chaque tab item + `role="tabpanel"` + `aria-labelledby` sur le conteneur de pane correspondant.

- [ ] **Accessibilité — `:focus-visible` manquant sur l'input de recherche** (WCAG 2.4.7)
  `SearchOverlay.svelte` : l'input principal a `outline: none` sans substitut `:focus-visible`. Un utilisateur naviguant au clavier n'a pas d'indicateur de focus visible sur cet input.
  Action : ajouter `:focus-visible { outline: 2px solid var(--color-focus-ring); outline-offset: -1px; }`.

- [ ] **Token fantôme `--size-tab-bar-height` dans `ConnectionManager.svelte`**
  Ligne 470 : `top: var(--size-tab-bar-height, 44px)` — le token canonique est `--size-tab-height`. Le fallback hardcodé `44px` sera utilisé si le token change, cassant silencieusement le positionnement du panneau.
  Action : remplacer par `var(--size-tab-height)`.

- [ ] **`hide_when_typing` — curseur souris masqué pendant la frappe** *(quick win — v1)*
  Curseur souris passé à `cursor: none` dès le premier `keydown` dans le viewport terminal, restauré sur `mousemove`. Alacritty l'implémente comme option configurable. Aligné avec AD.md §1.1 ("le chrome doit s'effacer pendant le travail") — un curseur souris au milieu de la sortie terminal est du bruit visuel non sollicité.
  Actions requises :
  1. Ajouter le comportement dans le composant terminal (CSS conditionnel + handlers `keydown`/`mousemove`).
  2. Exposer une option dans Préférences > Apparence ("Masquer le curseur souris pendant la frappe", activé par défaut).
  3. Spécifier dans `docs/uxd/04-interaction.md §8.1`.

- [ ] **Ligatures — vérifier la faisabilité dans WebKitGTK** *(à tester, probablement bloqué architecturalement)*
  Les ligatures de polices (FiraCode, Cascadia Code) sont la demande la plus upvotée d'Alacritty (issue #50, 1031 👍, ouverte depuis 2017 — refusée pour raisons architecturales OpenGL). Dans TauTerm, le rendu passe par WebKitGTK : les ligatures pourraient être activées via CSS `font-feature-settings: "liga" 1; font-variant-ligatures: contextual`.
  **Blocage architectural probable :** le modèle span-par-cellule de TauTerm (chaque caractère dans un `<span>` individuel) brise le contexte de shaping CSS — le moteur de rendu n'a pas accès aux glyphes adjacents pour former les ligatures. CSS shaping context exige que les glyphes adjacents soient dans le même nœud texte. Ce n'est pas un problème CSS, c'est une contrainte de l'arbre DOM. Tabby et Hyper contournent ce blocage via `@xterm/addon-ligatures` (`tabby-terminal/src/frontends/xtermFrontend.ts`), qui utilise HarfBuzz compilé en WASM pour mesurer les glyphes en canvas — non transposable à un renderer DOM.
  Action : tester avec FiraCode et Cascadia Code dans le renderer actuel pour confirmer ou infirmer le blocage. Si bloqué, documenter explicitement et lier à P12a (dirty tracking avec regroupement des `<span>` de même style en texte contigu).

- [ ] **Recherche — scroll-centering sur le match actif** (FS-SEARCH-006)
  La navigation prev/next entre résultats (`handleSearchNext`/`handleSearchPrev` dans `src/lib/composables/useTerminalView.io-handlers.svelte.ts:229-236`) incrémente uniquement `searchCurrentIdx` sans appeler `scroll_pane`. Si le match actif est dans le scrollback hors de la fenêtre visible, le viewport ne bouge pas — l'utilisateur ne voit pas le résultat actif.
  Actions requises :
  - Frontend : après chaque `handleSearchNext`/`handleSearchPrev`, calculer l'offset viewport cible depuis `match.scrollbackRow` et `scrollbackLines`, puis appeler `scrollPane(paneId, targetOffset)`.
  - Le calcul du match actif est déjà disponible dans `useTerminalPane.svelte.ts:222-233` (`activeSearchMatchSet`) — exposer ou dupliquer la logique de position dans le handler de navigation.

---

## P0 — Bloquants release

### Sécurité

- [ ] **SSH — clés privées protégées par passphrase** (FS-SSH-019a)
  `src-tauri/src/ssh/auth.rs:115` appelle `keys::load_secret_key(key_path, None)` — la passphrase est toujours `None`. Si la clé est chiffrée, l'authentification échoue silencieusement avec une erreur SSH sans prompt utilisateur.
  Actions requises :
  - Détecter l'erreur "clé chiffrée" retournée par `russh_keys::load_secret_key`.
  - Émettre un événement IPC `passphrase-prompt` (discriminated payload : `{ pane_id, key_path_label }` — ne pas inclure le chemin complet).
  - Ajouter un composant frontend de prompt de passphrase (analogue à `SshCredentialDialog`).
  - Intégrer SecretService : lookup par `identity_file` comme scope key (FS-CRED-008) ; option "Sauvegarder" opt-in non cochée par défaut.
  - Tests nextest : clé non chiffrée (chemin nominal), clé chiffrée avec bonne passphrase, mauvaise passphrase (retry), passphrase depuis keychain.

- [ ] **SEC-CRED-004 — Stockage du mot de passe à la sauvegarde d'une connexion SSH**
  `handleConnectionSave()` reçoit un mot de passe optionnel depuis `ConnectionManager` mais ne peut pas le stocker : aucune commande Tauri IPC n'expose `CredentialManager::store_password` (credentials.rs).
  Travail nécessaire :
  1. Ajouter `#[tauri::command] store_connection_password(connection_id: String, password: String)` côté Rust, en appelant `CredentialManager::store_password`.
  2. Ajouter le wrapper typé correspondant dans `src/lib/ipc/commands.ts`.
  3. Câbler l'appel dans `handleConnectionSave()` après que `saveConnection()` retourne le vrai `id` (ne jamais stocker le mot de passe sous l'`id` placeholder envoyé pour les nouvelles connexions).
  Jusqu'à ce que ce soit fait, le mot de passe saisi dans le gestionnaire de connexions n'est pas persisté (pas de perte silencieuse — `void password` rend l'intention explicite dans le code).

- [ ] **PTY — environnement hérité non filtré (fuite silencieuse de secrets)** *(sévérité haute)*
  `session/registry/tab_ops.rs` et `pane_ops.rs` construisent un `Vec<(&str, &str)>` de variables explicites, mais `portable-pty`'s `CommandBuilder::env()` **ajoute** à l'environnement hérité sans l'effacer. Toute variable présente dans l'environnement du processus TauTerm (`AWS_SECRET_ACCESS_KEY`, `GITHUB_TOKEN`, `DATABASE_URL`, etc.) est transmise silencieusement au shell fils et à tous ses sous-processus.
  **Note :** Alacritty a le même gap (`env_clear()` non appelé). C'est une lacune commune aux terminaux existants — TauTerm a l'opportunité d'être plus sécurisé que les références du secteur sur ce point.
  Actions requises :
  1. Appeler `cmd.env_clear()` avant les `cmd.env(key, val)` dans `LinuxPtyBackend::open_session()` — vérifier que `portable-pty::CommandBuilder` expose bien `env_clear()`.
  2. Définir explicitement `LANG`, `SHELL`, `HOME`, `USER`, `LOGNAME`, `PATH` (manquants par rapport à FS-PTY-011 — actuellement hérités de façon implicite).
  3. Ajouter un test nextest qui vérifie qu'une variable absente de la liste explicite n'est pas présente dans l'environnement du shell fils après spawn.
  4. Documenter la liste des variables autorisées dans FS-PTY-011.

- [ ] **SECURITY.md — politique de divulgation responsable** *(sévérité haute)*
  Aucun fichier `SECURITY.md` n'existe dans le repo. Sans lui, un chercheur qui découvre une vulnérabilité n'a aucun canal de signalement défini.
  Actions requises : créer `SECURITY.md` à la racine avec canal de contact privé (GitHub Security Advisories), délai de réponse attendu, politique de divulgation coordonnée (90 jours standard), liste des versions supportées.

- [ ] **SSH — test d'invariant "authentification jamais avant validation du host key"** *(leçon CVE-2024-48460 Tabby)*
  L'invariant est garanti architecturalement par `russh` (le callback `check_server_key` bloque `connect()`) mais n'est attesté par aucun test. Si `russh` changeait son comportement, la régression serait invisible.
  Actions requises :
  1. Ajouter un test d'intégration (mock SSH server) qui simule un serveur avec host key `Unknown` : vérifier que `try_authenticate` n'est jamais appelé.
  2. Ajouter un commentaire `// SECURITY:` dans `connect.rs` documentant cet invariant.

### Tests et CI

- [ ] **Mettre en place le pipeline CI GitHub Actions**
  - Jobs minimum : `cargo clippy -- -D warnings`, `cargo nextest run`, `pnpm check`, `pnpm vitest run`
  - Ajouter `cargo audit` et `cargo deny` (avec `deny.toml`) pour bloquer les advisories et licences non conformes
  - Déclencheur : push sur `dev` et `main`, PR vers `main`

---

## Backlog

### Performance — benchmarking

#### Criterion — couverture manquante

Le seul fichier bench existant (`src-tauri/benches/vt_throughput.rs`) couvre uniquement le parser VT sur ASCII et quelques primitives `DirtyRows`. Tout le pipeline aval et la dimension latence sont aveugles.

**Axe débit (throughput)**

- [ ] **Contenu VT réaliste** — Ajouter des benchmarks sur des séquences CSI, SGR, OSC, déplacement curseur, wide chars (contenu type `htop`, `vim`, `ls --color`). Une régression dans le dispatch CSI est actuellement indétectable.
- [ ] **Unicode/emoji hot path** — Benchmarker `write_char()` sur des codepoints wide et Regional Indicators (U+1F1E6–), qui activent `pending_ri`/`pending_emoji` et des allocations `CompactStr`. Zéro couverture actuelle.
- [ ] **Éviction scrollback** — Benchmarker `scroll_up` avec `scrollback.len() >= scrollback_limit` pour mesurer le coût du `pop_front()` + désallocation du `Vec<Cell>` évincé sur le hot path.
- [ ] **`build_screen_update_event` full redraw** — C'est le point le plus vulnérable : ~11 000 `String` allouées par event sur un viewport 220×50 (`.to_string()` par cellule). Déclenché à chaque resize, clear-screen, alt-screen toggle, premier affichage. Aucun bench existant.
- [ ] **Sérialisation JSON du payload IPC** — Le payload `ScreenUpdateEvent → serde_json` atteint 500 Ko–1 Mo en full redraw. Benchmarker `serde_json::to_string()` sur ces structures pour quantifier le coût et évaluer l'impact d'un codec plus compact.
- [ ] **`ProcessOutput::merge()` en burst** — Benchmarker la coalescence répétée des `DirtyRegion` sur 50+ messages dans une fenêtre de debounce (12 ms), pas un seul mark+iterate isolé.

**Axe latence perçue**

Aucun benchmark orienté latence n'existe dans le projet.

- [ ] **Cycle complet process→emit** — Benchmarker le chemin `process() → take_dirty() → build_screen_update_event()` en isolation pour mesurer la part du budget 12 ms consommée côté Rust. C'est la seule mesure permettant de poser un budget réaliste et de détecter des régressions de latence perçue.
- [ ] **`build_scrolled_viewport_event`** — Benchmarker la reconstruction du viewport composite scrollback + écran live (plus coûteuse qu'un full redraw). Une régression ici se manifeste directement par un scroll saccadé.
- [ ] **Contention RwLock Task1/Task2** — Task1 acquiert `vt.write()` par chunk PTY, Task2 acquiert `vt.read()`. Benchmarker le temps moyen d'acquisition sous charge concurrente simulée.
- [ ] **Allocations hot path partial update** — Benchmarker le chemin partiel `build_screen_update_event` incluant les 220 `String::to_string()` par ligne sale, sous `vt.read()`, sur le thread async Tokio.

#### vtebench — benchmark de comparaison inter-terminaux

- [ ] **Script de benchmarking vtebench** ([alacritty/vtebench](https://github.com/alacritty/vtebench))
  Écrire un script `scripts/bench-vtebench.sh` qui :
  - Clone/installe vtebench si absent, ou détecte un binaire existant dans `$PATH`
  - Lance TauTerm en mode headless ou dans un Xvfb si nécessaire
  - Exécute la suite vtebench standard contre TauTerm
  - Produit un rapport comparatif (JSON ou Markdown) avec date et commit git
  - Documente les cas de bench couverts et la méthode de comparaison avec d'autres terminaux (Alacritty, foot)

---

### VT / Émulation — bugs de correctness et fonctionnalités manquantes

- [ ] **Cursor sur cellule phantom d'un wide char — normalisation manquante** *(bug de correctness)*
  Quand `CUP`, `HVP`, ou tout mouvement de curseur absolu atterrit sur une cellule phantom (width=0, trailing slot d'un wide char), le curseur devrait être normalisé vers la base cell (col − 1). Ni le code (`csi_cursor.rs`) ni les specs ne couvrent ce cas. `DSR CPR` (CSI 6 n) retourne alors une position incorrecte ; les éditeurs qui écrivent à cette position (vim, helix, tmux) corrompent le wide char silencieusement.
  **Référence Alacritty** (`alacritty_terminal/src/term/mod.rs`, fn `goto()`) **:** patch #8786 — la correction est triviale : après tout positionnement absolu, insérer `if col > 0 && buf[row][col].flags.contains(WIDE_CHAR_SPACER) { cursor.col -= 1; }`. Toute la logique existe déjà — c'est une guard de 2 lignes à appeler systématiquement. La normalisation côté rendu existe déjà dans `RenderableCursor::new` mais n'affecte pas la position logique utilisée par `DSR CPR`.
  Actions requises :
  1. Ajouter un FS-VT-0xx : "When a cursor positioning command results in the cursor landing on a phantom cell, the cursor MUST be adjusted to the base cell (col - 1)."
  2. Implémenter `normalize_cursor_position()` dans le VT processor (`src-tauri/src/vt/processor/dispatch/csi_cursor.rs`), appelée après tout mouvement absolu (`cup`, `vpa`, `cha`, `hpa`, `decrc`). Vérifier `cell.flags.contains(WIDE_CHAR_SPACER)` et décrémenter `col` si vrai.
  3. Tester avec vim et helix en locale CJK.

- [ ] **OSC 7 (CWD reporting) — non implémenté, non spécifié** (FS-TAB-006)
  OSC 7 (`ESC ] 7 ; file://hostname/path ST`) est émis par fish, zsh (oh-my-zsh), bash (`__vte_prompt_command`) pour signaler le répertoire courant. Actuellement ignoré dans `src-tauri/src/vt/osc.rs` (branche `_ => OscAction::Ignore`). Non spécifié dans `docs/fs/`.
  Impact : les titres d'onglets ne reflètent pas le CWD sans config shell manuelle ; les futurs "ouvrir un pane ici" ne peuvent pas utiliser le CWD courant.
  Actions requises :
  1. Ajouter un FS-VT-0xx : "OSC 7 SHOULD be received and stored as the current working directory of the pane. The CWD MUST be used as the initial directory for new panes/tabs opened from the same pane."
  2. Parser OSC 7 dans `osc.rs`, stocker dans `PaneSession`, exposer via IPC.
  3. Utiliser le CWD pour le titre d'onglet par défaut (priorité : user-label > OSC 0/2 > OSC 7 CWD basename > nom du processus).
  4. Utiliser le CWD lors de `split_pane` / `create_tab`.

- [ ] **Titres d'onglets — fallback process name non implémenté** (FS-TAB-006)
  FS-TAB-006 mentionne "process name" comme fallback de titre, mais la mécanique n'est pas définie ni implémentée. `pane_state.rs` expose `tcgetpgrp` mais ne lit pas `/proc/{pgid}/comm`. Sans OSC 0/2/7 émis par le shell, le titre reste vide.
  Action : lire `/proc/{pgid}/comm` via le PGID retourné par `tcgetpgrp`, et l'utiliser comme titre de fallback. Spécifier la chaîne de priorité dans FS-TAB-006.

- [ ] **Mouse mode 1003 (AnyEvent) — déjà correctement implémenté, test manquant uniquement**
  Le backend tracke correctement le mode `AnyEvent` (enum `MouseReportingMode`), et la frontend (`src/lib/composables/useTerminalPane.svelte.ts:576–591`) transmet bien les événements `mousemove` sans bouton pressé via `send_mouse_event` quand `mouseReportingMode === 'anyEvent'`. **TauTerm est en avance sur Tabby et Hyper** qui délèguent entièrement à xterm.js et ne gèrent pas 1003 nativement.
  Action restante : ajouter un test fonctionnel (scénario dans `docs/test-protocols/functional-pty-vt-ssh-preferences-ui-ipc.md`) couvrant : activation mode 1003 → move souris sans bouton → vérifier que les bytes encodés arrivent bien au PTY.

- [ ] **Pixel dimensions nulles dans `pty-req` SSH et ouverture PTY locale** (FS-SSH-013)
  `connect.rs` et `pty_linux/backend.rs` passent `pixel_width: 0, pixel_height: 0`. FS-SSH-013 exige que `xpixel`/`ypixel` soient transmis. Nécessaire pour les applications qui calculent la taille des polices ou utilisent les pixel dimensions (futurs protocoles graphiques en SSH).
  Action : la frontend doit calculer et transmettre `cell_pixel_width × cell_pixel_height` au backend lors de l'ouverture et du resize. Le backend propage dans `TIOCSWINSZ` (local) et `window-change` (SSH).

- [ ] **Terminal modes RFC 4254 §8 — validation on-the-wire absente**
  Les terminal modes (`TERMINAL_MODES` dans `ssh/manager.rs`) sont définis correctement en intention, mais aucun test ne vérifie que `russh` encode effectivement les opcodes RFC 4254 (VINTR=1, VQUIT=2, … ECHO=53) et non les constantes POSIX `termios.h`. Une divergence causerait des signaux incorrects sur le serveur distant (Ctrl+C ne tue pas le process, etc.).
  Action : ajouter un test d'intégration capturant la trame SSH et vérifiant les opcodes encodés dans le `pty-req`, ou inspecter le code source de `russh` pour confirmer le mapping.

- [ ] **PTY EIO — vérifier le comportement sur erreur de lecture** *(robustesse)*
  Sur Linux, `read()` retourne `EIO` quand le côté maître du PTY est lu après la mort du processus fils. La réponse correcte est de terminer proprement la boucle de lecture et d'émettre `ProcessExited`. Un `break` immédiat sur toute erreur `io::Error` (pattern commun mais incorrect) peut masquer des erreurs transitoires et interrompre la session prématurément.
  **Référence Alacritty** (`alacritty_terminal/src/tty/unix.rs`, `alacritty_terminal/src/event_loop.rs`, `alacritty_terminal/src/tty/mod.rs`) **:** traite `EIO` comme signal de fin normale du process (non comme erreur) — `continue` sur les erreurs transitoires, `break` uniquement sur `EIO` et fermeture explicite. À vérifier dans `src-tauri/src/platform/pty_linux/backend.rs` : la boucle de lecture PTY distingue-t-elle `EIO` des autres erreurs ?
  Action : inspecter la boucle de lecture PTY, s'assurer que `EIO` → `ProcessExited` (non `SessionError`) et que les erreurs transitoires (`EINTR`, `EAGAIN`) ne terminent pas la boucle.

- [ ] **SSH reconnect — re-injection des credentials manquante** (FS-SSH-040, FS-SSH-041)
  L'architecture de reconnexion est en place (bouton dans `TerminalPaneBanners.svelte`, `handleReconnect`, commande `reconnect_ssh`). Cependant, `SshManager::reconnect()` retourne `Ok(())` sans réinjecter les credentials (stub documenté dans le security protocol). Pour les connexions password-auth, la reconnexion est un leurre.
  Actions requises :
  1. Implémenter la re-injection dans `SshManager::reconnect()` : lookup keychain par (host, port, username), ou prompt `credential-prompt` si indisponible (FS-CRED-007).
  2. Coordonner avec la completion des stubs SSH (`auth.rs`, `credentials_linux.rs`).
  3. Ajouter un test nextest : reconnexion d'une session password-auth avec mock credential store.

### VT / Émulation — tests manquants

- [ ] **Mouse mode PTY — tests d'activation/désactivation et compatibilité tmux** (FS-VT-080–082)
  L'encodage des trois formats est testé. Lacunes : aucun test d'activation/désactivation des modes 1002/1003/1006/1015 ni de leur interaction (ex. `?1000h` + `?1006h` → SGR encoding actif). Aucun scénario "tmux" (1000+1002+1006 SGR) dans le protocole fonctionnel.
  Actions requises :
  1. Ajouter dans `src-tauri/src/vt/processor/tests/modes.rs` : tests d'activation et de reset des modes 1002h/1003h/1006h/1015h.
  2. Ajouter un test de round-trip : set mode → `MouseEvent::encode()` → bytes attendus (matrice reporting × encoding).
  3. Ajouter un scénario tmux dans `docs/test-protocols/functional-pty-vt-ssh-preferences-ui-ipc.md`.

- [ ] **Suite de non-régression SSH/credential explicite** (FS-CRED-001–009, SEC-CRED-001/002/005)
  Trois scénarios de sécurité critiques restent `BLOCKED` dans `docs/test-protocols/security-pty-ipc-ssh-credentials-csp-osc52.md` : SEC-CRED-001 (credentials.json), SEC-CRED-002 (zeroize), SEC-CRED-005 (fallback keyring). Aucune suite de non-régression dédiée n'est définie comme critère de merge.
  Actions requises (après levée des stubs) :
  1. Créer `src-tauri/tests/credential_regression.rs` : suite taggée non-régression incluant SEC-CRED-001/002/003/004/005/006.
  2. Ajouter cette suite aux critères de merge dans `docs/testing/TESTING.md`.

- [ ] **PTY teardown — test end-to-end `close_pane` → `ProcessExited`** (FS-PTY-lifecycle)
  La séquence de teardown (Drop-cascade → SIGHUP → `ProcessExited` event) est implémentée mais non couverte end-to-end. Aucun test nextest ne vérifie qu'après `close_pane`, l'event `ProcessExited` est émis avec le bon `pane_id` avant la destruction de l'entrée de registre.
  Action : ajouter un test d'intégration qui spawne une vraie session PTY, appelle `close_pane`, et vérifie la réception de `ProcessExited`.

### Architecture — dette documentée

- [ ] **Schéma de prefs — versioning manquant** *(dette de migration future)*
  `Preferences` n'a pas de champ `schema_version: u32`. La compatibilité forward repose exclusivement sur `#[serde(default)]` : un champ renommé ou supprimé dans une version future réinitialise silencieusement la valeur utilisateur sans avertissement ni chemin de recovery. ADR-0012 et ADR-0016 reconnaissent cette dette sans en être propriétaires.
  **Référence Tabby** (`tabby-core/src/services/config.service.ts`, fn `migrate()`) **:** migration séquentielle v0→v8 opérant sur `serde_json::Value` *avant* désérialisation — pattern : `if version < N { transform_raw_json(&mut raw); version = N; }` répété par étape. Avantage : chaque étape est testable en isolation et la désérialisation typed ne voit jamais de données incohérentes.
  Actions requises :
  1. Écrire un ADR documentant la décision (accepter la dette et en définir les limites, ou ajouter `schema_version` + moteur de migration séquentielle).
  2. Si version ajoutée : implémenter `migrate_from(version, raw: serde_json::Value) -> serde_json::Value` dans `preferences/store/` avec les étapes séquentielles. Commencer à `schema_version: 1` (v0 = absence du champ, traitée par `unwrap_or(0)`).

- [ ] **Parse Don't Validate — champs texte non validés à la frontière IPC** (SEC-IPC-005)
  Les champs numériques et enums sont validés à la frontière (`validate_and_clamp`). Les champs texte libres ne le sont pas : `font_family`, `theme_name`, `word_delimiters`, `SshConnectionConfig.host`, `SshConnectionConfig.label` acceptent des valeurs arbitraires incluant caractères de contrôle et chaînes vides.
  **Référence Alacritty** (`alacritty/src/config/bindings.rs`, `alacritty/src/config/ui_config.rs`, derive macro dans `alacritty_config_derive/src/lib.rs`) **:** pattern `RawBinding` → newtypes avec `impl TryFrom<String>` faisant la validation à la construction, décorés `#[serde(try_from = "String")]`. La désérialisation produit soit un type valide, soit une erreur serde — le code consommateur n'a plus besoin de valider.
  Action : ajouter des newtypes validés (`SshHost`, `FontFamily`) dans `preferences/types.rs` avec `TryFrom<String>` + `#[serde(try_from = "String")]`. Contrôles minimaux : longueur maximale, absence de caractères de contrôle, non-vide. Prioriser `SshConnectionConfig.host` (risque modéré : alimente la logique SSH).

- [ ] **ADR — stratégie de teardown de session PTY**
  La séquence Drop-cascade → SIGHUP → `ProcessExited` est implémentée et documentée dans `docs/arch/` §5.1 et §7.1, mais sans ADR. Le choix Drop-cascade (vs. shutdown explicite ordonné) a des conséquences (pas de garantie de flush avant `abort()`) qui méritent d'être documentées et conscientisées.
  Action : écrire un ADR documentant le contrat de teardown, le rationale Drop-cascade, et la garantie de livraison de SIGHUP.

- [ ] **ADR — stratégie de coalescing du rendu (debounce 12 ms, canal 256 slots)**
  La stratégie est implémentée et documentée dans `docs/arch/` §6.2 et §6.5, mais sans ADR. Le choix du debounce à 12 ms, du canal bounded à 256, et du pipeline deux-tâches mérite d'être formalisé avec son rationale (comportement sur saturation du canal, budget latence).
  Action : écrire un ADR documentant ces décisions.

### Sécurité — backlog

- [ ] **OSC52 — test du flag per-connexion SSH vs. flag global**
  Le flag `allow_osc52_write` existe à deux niveaux (global `TerminalPrefs` et per-connexion `SshConnectionConfig`). La logique de surcharge est documentée dans `docs/arch/06-appendix.md §8.2` mais n'est couverte par aucun test. Une régression dans `propagate_osc52_allow()` rendrait le flag per-connexion inopérant.
  Action : ajouter deux tests nextest : (1) connexion SSH avec `allow_osc52_write: true` et global `false` → écriture autorisée ; (2) inverse.

- [ ] **`capabilities/default.json` — inventaire de `core:default`**
  Le preset `core:default` n'a jamais été audité. Son contenu exact est opaque — s'il inclut des capacités non utilisées par TauTerm (`core:process:allow-terminate`, etc.), le principe de moindre privilège est violé sans que l'équipe le sache.
  Action : consulter la liste officielle Tauri 2 de `core:default`, documenter les capacités incluses dans un commentaire dans `default.json`, et remplacer par une liste explicite si des capacités superflues sont identifiées.

- [ ] **CSP `style-src 'unsafe-inline'` — critère de suppression et vérification WebKit**
  Documenté comme "future tightening" dans `docs/arch/06-appendix.md §8.4` mais sans vérification de faisabilité (les nonces de style sont-ils supportés par WebKit2GTK + Tauri 2 ?) ni critère d'entrée concret.
  Action : vérifier expérimentalement le support des nonces. Si non supporté, créer un ADR documentant cette contrainte comme permanente v1 (éviter une fausse promesse d'architecture). Si supporté, définir un critère de sortie dans FS-SEC-001.

### Documentation — gaps mineurs

- [ ] **`docs/uxd/02-tokens.md §3.4`** — ajouter la table complète `--term-color-0` à `--term-color-15` (actuellement renvoyée vers `AD.md §3.2`) et documenter explicitement le choix `--term-dim-opacity` (opacité) vs tokens de couleur dim séparés.
- [ ] **`docs/uxd/04-interaction.md §8.4`** — documenter pourquoi aucun overlay shim n'est nécessaire pendant le drag de divider (`setPointerCapture` résout le problème ; éviter une régression future par ajout d'un shim inutile).
- [ ] **`docs/uxd/03-components.md §7.2`** — documenter explicitement que la distinction pane active/inactive passe par la couleur du border uniquement (pas d'opacité sur le contenu viewport), avec la justification (WCAG safe-by-default, lisibilité préservée).

### Roadmap v1.1 — killer features identifiées par analyse comparative

*Fonctionnalités absentes des specs actuelles, validées par l'analyse de Tabby, Alacritty et Hyper. À spécifier dans `docs/UR.md` et `docs/fs/` avant implémentation.*

- [ ] **Hints cliquables + OSC 8 (hyperliens dans le terminal)**
  La killer feature d'Alacritty absente de TauTerm. Deux niveaux :
  - **OSC 8 passif** : reconnaître les séquences OSC 8 (`ESC ] 8 ; params ; uri ST`) émises par des outils comme `ls --hyperlink`, `git log`, `delta`, et rendre l'URI cliquable (Ctrl+clic → ouvrir dans le navigateur/éditeur configuré). Standard IETF, WCAG-compatible, aligné avec AD.md §1.3.
  - **Hints actifs** : sur raccourci configurable, afficher un overlay de labels courts sur toutes les URLs/chemins détectés par regex dans la vue courante — appuyer sur le label déclenche l'action (ouvrir, copier). Style vim-hints / Alacritty hints.
  Personas concernés : Alex (stack traces, chemins de fichiers), Jordan (URLs dans les logs).
  **Architecture Alacritty** (`alacritty_terminal/src/term/cell.rs` pour le stockage, `alacritty/src/display/hint.rs` pour l'overlay hints) **— `CellExtra` avec `Option<Arc<HyperlinkInner>>`** : l'URI est stockée de manière paresseuse dans les cellules — `Cell` a un `extra: Option<Box<CellExtra>>` alloué uniquement si la cellule a des attributs non-standard (hyperlink, etc.). Les cellules ordinaires ont un coût mémoire nul pour ce champ. À adopter : ajouter `hyperlink: Option<Arc<HyperlinkUri>>` (lazy) dans `Cell` de TauTerm, sans impacter les performances sur contenu normal.
  **Séquençage recommandé :**
  1. **Phase 1 (OSC 8 passif)** : parser OSC 8 dans `osc.rs` → stocker l'URI dans `Cell` → exposer via IPC → afficher un underline décoré côté frontend avec Ctrl+clic.
  2. **Phase 2 (Hints actifs)** : overlay DOM généré à la demande par regex scan du buffer visible → labels courts → action configurable.
  Actions requises : spécifier dans FS + UXD, implémenter en deux phases.

- [ ] **Session persistence — restauration des onglets au relancement**
  Absent de `docs/UR.md`. Pain point quotidien pour Alex (4 tabs : frontend, backend, logs, git) et Jordan (10+ sessions SSH). Demande très upvotée sur Hyper (#311) et comportement attendu de Tabby ("Tabby remembers your tabs").
  Comportement cible : à la fermeture, sérialiser la liste des onglets (type local/SSH, titre, profil de connexion associé). Au relancement, proposer "Restaurer la session précédente ?" — opt-in, pas imposé (Sam ne voudra pas toujours).
  Note : les PTYs locaux ne sont pas restaurables (processus mort) — seules les métadonnées sont restaurées. Les connexions SSH sauvegardées peuvent être relancées automatiquement. **Ne jamais sérialiser le VT buffer** — données potentiellement sensibles + coût mémoire.
  **Architecture Tabby** (`tabby-core/src/services/tabRecovery.service.ts`, `tabby-core/src/api/tabRecovery.ts`, `tabby-core/src/services/app.service.ts`, `tabby-ssh/src/recoveryProvider.ts`) **— `TabRecoveryProvider` pattern :** enum discriminée Rust :
  ```rust
  #[derive(Serialize, Deserialize)]
  #[serde(tag = "type")]
  enum TabSnapshot {
      LocalPty { title: String, working_dir: Option<String>, shell: String },
      Ssh { connection_id: String, title: String },
  }
  ```
  Stockée dans `~/.config/tauterm/session.json`. Au démarrage : désérialiser → proposer le dialog de restauration → recréer les onglets depuis les snapshots. `working_dir` provient du OSC 7 CWD tracking (dépendance : item OSC 7 ci-dessus).
  Actions requises : spécifier dans `docs/UR.md §4.1` + `docs/fs/`, implémenter `SessionSnapshot` en Rust avec `#[serde(tag = "type")]`, ajouter dialog de restauration au démarrage.

- [ ] **Jump hosts / ProxyJump SSH dans le connection manager**
  Absent de `docs/UR.md §9`. Le cas d'usage standard de Jordan : accéder à des serveurs en réseau privé via un bastion. Sans ProxyJump dans l'UI, Jordan configure son `~/.ssh/config` manuellement et les connexions TauTerm ne correspondent pas à son infrastructure réelle.
  Tabby gère les jump hosts nativement : chaque connexion sauvegardée peut référencer un profil "jump host", avec messages d'erreur ciblés par maillon de la chaîne.
  La lib `russh` supporte le ProxyJump — c'est un problème de data model (ajouter un champ "via jump host" dans `SshConnectionConfig`) + UI form, pas un problème de transport.
  **Architecture Tabby** (`tabby-ssh/src/components/sshTab.component.ts`, fn `setupOneSession`, `tabby-ssh/src/session/ssh.ts`, data model dans `tabby-ssh/src/api/interfaces.ts`) **— `direct-tcpip` channel (RFC 4254 §7.2) :**
  1. Authentifier la session SSH sur le bastion normalement (host key + auth).
  2. Ouvrir un canal `direct-tcpip` via `session.channel_open_direct_tcpip(target_host, target_port, originator, originator_port)`.
  3. Utiliser ce canal comme transport TCP pour une seconde session `russh::client::connect_stream` vers la cible finale.
  4. Data model : `jump_host_id: Option<String>` dans `SshConnectionConfig` (référence un autre profil de connexion sauvegardé). Limiter à 1 niveau de saut pour v1.
  Actions requises : spécifier dans `docs/UR.md §9` + `docs/fs/03-remote-ssh.md`, étendre `SshConnectionConfig`, implémenter la séquence `direct-tcpip` dans `src-tauri/src/ssh/manager/connect.rs`, ajouter le champ "Via jump host" dans le formulaire de connexion.

- [ ] **Pane maximized — agrandir un pane sans détruire le split**
  Absent de `docs/uxd/03-components.md §7.2`. Workflow Alex : 3 panes ouverts, besoin de focus temporaire sur l'un sans perdre le contexte des autres. Fermer et recréer détruit l'historique VT.
  Comportement cible : raccourci `Ctrl+Shift+Enter` (configurable) bascule le pane actif en état "maximized" — il occupe toute l'aire du split, les autres sont masqués mais non détruits. Une bordure `--color-accent` + badge discret signale l'état. Même raccourci ou `Escape` restitue le layout.
  Aligné avec AD.md §1.3 "Durability Over Novelty" : aucun état perdu, aucun PTY tué.
  **Référence Tabby** (`tabby-core/src/components/splitTab.component.ts` + `splitTab.component.scss`) — état `maximizedTab: BaseTabComponent|null`, méthode `maximize(tab)` qui appelle `layout()`. Effets visuels : pane maximized en `position: absolute`, `left/top: 5%`, `width/height: 90%`, `z-index: 6`, `box-shadow`, `backdrop-filter: blur(10px)`, `border-radius: 10px`. Panes non-maximized : `opacity: 0.1`. Raccourci `pane-maximize` bascule (`null` → pane → `null`).
  **Traduction TauTerm (Svelte 5, déclarative) :**
  - État : `let maximizedPaneId = $state<PaneId | null>(null)` dans `useTerminalView.core.svelte.ts`
  - `SplitPane.svelte` : chaque leaf vérifie `node.paneId === maximizedPaneId` → CSS conditionnel (`position: absolute; inset: 0; z-index: 6` + `backdrop-filter`, `box-shadow` depuis design tokens). Pas de mutation DOM directe (contrairement à Tabby qui opère en impératif).
  - Raccourci : ajouter `pane-maximize` dans `handleGlobalKeydown()` de `useTerminalView.io-handlers.svelte.ts`.
  Actions requises : spécifier dans `docs/uxd/03-components.md §7.2` + `docs/uxd/04-interaction.md`, implémenter l'état layout dans `SplitPane.svelte`.

---

### Post-v1

- [ ] **Kitty keyboard protocol** (déféré explicitement — ADR-0003, FS-05-scope-constraints.md)
  Activé par défaut dans Alacritty ; requis pour Neovim 0.10+ (Shift+Enter, Ctrl+I vs Tab, Ctrl+M vs Enter). Extension naturelle : flag dans `ModeState`, dispatch `CSI > 4 ; flags m` (enable) / `CSI < u` (disable) dans `Perform::csi_dispatch`, encodage frontend selon le mode actif.
  Note : implémentation non triviale — Alacritty a eu 6 corrections de bugs entre v0.13 et v0.16 (Shift+number, C0/C1 dans associated text).

- [ ] **Vi mode — navigation clavier dans le scrollback**
  Alacritty killer feature. Mode modal intégré au terminal : mouvements vi (`w`, `b`, `{`, `}`), recherche (`/`), sélection par blocs, yank vers le presse-papiers. Pas un wrapper tmux — un état géré par le terminal avec curseur vi indépendant. Power user (Alex qui vit dans neovim).
  Coût : state machine VT supplémentaire + frontend. Substantiel — à ne pas sous-estimer.

- [ ] **Keyword highlighting temps réel dans le flux terminal**
  Mise en évidence de patterns (erreurs, IPs, noms de fichiers) dans l'output en temps réel, via regex configurables. Tabby a une demande très upvotée (#632). Différenciateur fort pour Jordan qui scanne des logs.
  Distinction avec la recherche existante : la recherche est ponctuelle et rétroactive ; le highlighting est continu et prospectif.

- [ ] **SFTP intégré — panneau contextuel dans la session SSH**
  Tabby's biggest differentiator pour les ops. Panneau latéral CWD-aware dans le même onglet que la session SSH active, avec filter bar, download de dossiers, upload drag-and-drop. Élimine le besoin de FileZilla ou `scp` manuel.
  Coût backend : implémentation d'un client SFTP complet côté Rust. Substantiel — v2 réaliste.

- [ ] **Mosh support**
  Demande très upvotée dans Tabby (#593, ouverte). Résout le pain point de Jordan : sessions SSH qui meurent sur reconnexion réseau (laptop en veille, wifi instable). Mosh maintient la session via UDP même après une coupure.
  Coût : intégration de la lib mosh ou spawn d'un processus `mosh-client` externe. Complexe — à investiguer.

---

### Performance — pistes à valider par mesure

Ces deux optimisations ont été explicitement mises hors scope de la campagne de performance initiale. Le benchmark `write_1mb_ascii` établit la baseline actuelle à **~19.6 MiB/s** — à comparer après chaque changement.

- [ ] **P5 — Flat buffer pour `ScreenBuffer`**
  Remplacer `Vec<Vec<Cell>>` par un unique `Vec<Cell>` de taille `rows × cols` avec accès par `row * cols + col`. Élimine un niveau d'indirection et améliore la cache locality lors des lectures séquentielles (snapshot, partial update). Estimation d'impact : 3–10× sur `write_1mb_ascii`. **Risque** : breaking change sur toutes les APIs qui exposent `&[Cell]` par ligne (`get_row`, `scroll_up`, etc.) — refactor non trivial.

- [ ] **P12a — DOM avec dirty tracking + recycling de cellules** *(étape 1 — faible risque)*
  Le vrai problème est que les 11 000 `<span>` sont recréés ou modifiés globalement à chaque update, pas que le rendu est DOM. xterm.js a validé en production (v6.0, déc. 2024) qu'un DOM renderer avec dirty tracking peut être compétitif avec Canvas — c'est d'ailleurs pourquoi ils ont abandonné le Canvas addon.
  Actions requises :
  - Ne parcourir que les lignes marquées sales (`DirtyRows`) pour les mises à jour DOM
  - Recycler les éléments `<span>` existants (modifier leurs propriétés plutôt que les recréer)
  - Mesurer via les benchmarks Criterion (axe latence perçue) avant et après
  **Condition de succès :** si P12a ramène la latence de rendu dans le budget cible, P12b est inutile.

- [ ] **P12b — WebGL2** *(étape 2 — seulement si P12a insuffisant après mesure)*
  Si les benchmarks post-P12a montrent que le DOM reste le bottleneck sur des viewports larges, envisager WebGL2 via un addon. **Canvas 2D est à exclure** : il concentre les inconvénients des deux approches (perd l'accessibilité DOM comme WebGL, sans les gains GPU) — c'est la conclusion de xterm.js qui a abandonné son Canvas addon en v6.0 précisément pour cette raison.
  Contraintes WebGL2 sur WebKitGTK/Linux :
  - Fallback obligatoire sur `onContextLoss` (crash driver GPU, tab backgrounded)
  - Transparence de fond incompatible avec WebGL (force un fond opaque)
  - Ligatures structurellement incompatibles (ne peut pas dessiner au-delà des bordures de cellule)
  - Couche DOM parallèle pour l'accessibilité AT-SPI2 obligatoire (`role="list"` + `aria-live="assertive"`, alimentée depuis le buffer, `aria-hidden="true"` sur le canvas) — ~17 KB de code chez xterm.js pour ce seul composant
  - Sélection texte à ré-implémenter entièrement (modèle logique `[col, row]`, tracking souris, extraction depuis buffer Rust, clipboard via Tauri)
  **Ne pas engager P12b sans données de benchmark post-P12a.**

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
