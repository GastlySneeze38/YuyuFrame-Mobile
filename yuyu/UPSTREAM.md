# D'où vient ce dossier

`horizon-android/` est une copie de **Starling** (portage Android d'Horizon, licence MIT),
prise sur [starling-client/starling](https://github.com/starling-client/starling).

| | |
|---|---|
| Tag d'origine | `v0.9.0` |
| Commit d'origine | `ff303502a4f7982551a0e0d3622d9268efe9e829` |
| Date | 2026-11-20 |
| Message | Merge pull request #768 from starling-client/catalog-hide-whole-game |
| Version de l'appli Starling | 0.9.0 (`packaging/android/build_apk.py`, `VERSION_NAME`) |
| Copie précédente | 0.2.3, commit `121742ad`, jusqu'au commit `f0ecd8a9^` du monorepo |

Comme `server-pvp/` pour Pumpkin, le `.git` d'origine a été retiré : le code est suivi par le
monorepo.

## Ce qui a changé par rapport à Starling (2026-11-22)

Depuis la 0.8, Starling embarque **son propre launcher** (`crates/horizon-client/src/ha_launcher/`,
celui de Horizon Android 0.7.8, ~24 000 lignes) : un écran avant le jeu, avec instances, catalogue,
contenu installé, gestionnaire de fichiers, comptes, réglages et garde-robe. Notre launcher fait
déjà ce travail, et le moteur doit devenir **générique** (fonctions sans écran, utilisables par
n'importe quel launcher, portable sur iOS). Leur launcher a donc été **retiré**, pas contourné :

| Fichier | Changement |
|---|---|
| `crates/horizon-client/src/ha_launcher/` | Supprimé |
| `crates/horizon-client/src/lib.rs` | `pub mod ha_launcher` retiré |
| `crates/horizon-client/Cargo.toml` | `horizon-port-pack`, `url`, `image` retirés des dépendances Android (seul leur launcher s'en servait) |
| `crates/horizon-client/src/accounts.rs`, `game/starling/setup/mod.rs` | Nos deux correctifs (ci-dessous) |
| `crates/horizon-client/src/android.rs` | Toujours le jeu : plus de branche launcher, `game_started`, `IN_GAME`, `acknowledge_play`, `restart_into_game` ; `finish_and_exit` termine toujours ; dossier `profiles` retiré de `hide_from_gallery` |
| `crates/horizon-client/src/android_lib.rs` | `nativeDocumentResult` retiré (sélecteur de fichiers de leur launcher) |
| `crates/horizon-client/src/game/interface/screens.rs`, `title.rs`, `title/preset_buttons.rs`, `world/world_session.rs`, `interface/startup_menu.rs` | Bouton « Launcher » de l'écran-titre et du menu pause retiré (`LAUNCHER`, `Action::Launcher`) |
| `crates/horizon-client/src/android_control_ui.rs`, `game/starling/mobile_controls/mod.rs` | Bouton « Launcher » du hub tactile retiré (`OpenLauncher`) |
| `crates/horizon-client/src/game/runtime/loading_screen.rs`, `resource_loader.rs`, `game/starling/setup/mod.rs`, `setup/onboarding.rs` (supprimé) | Écran de premier démarrage de leur launcher retiré |
| `crates/horizon-client/src/game/starling/own_skin.rs`, `own_skin/wardrobe.rs` (supprimé), `interface/options.rs`, `network/remote/mod.rs`, `starling/imports.rs`, `starling/touch/mod.rs`, `starling/mobile_controls/mod.rs` | Garde-robe Android en jeu retirée (elle reposait sur la bibliothèque de skins de leur launcher) : la page Skin est celle des autres plateformes. Le code de mise en page de la garde-robe reste dans `horizon-client-render/src/options_ui/`, éteint (`wardrobe_enabled` reste à faux) |
| `packaging/android/java/.../StarlingActivity.java` | `pickDocument`, `PICK_DOCUMENT` et `nativeDocumentResult` retirés |

Chaque endroit est marqué « YuyuFrame » dans le code quand il en reste une trace.

**Le launcher PC d'Horizon** (`launcher/`, GPUI, hors du workspace) a été retiré aussi, avec ce qui
s'en servait :

| Fichier | Changement |
|---|---|
| `launcher/` | Supprimé |
| `crates/starling-catalog/` | Supprimé : il compilait le navigateur de mods du launcher PC (`#[path = "../../../launcher/src"]`) pour le **catalogue en jeu** |
| `crates/horizon-client/src/game/starling/mod_catalog.rs` | Supprimé (l'écran du catalogue en jeu) ; ses appels retirés de `game.rs`, `runtime/window_handler.rs`, `starling/input_mode.rs`, `starling/touch/mod.rs`, `starling/touch/text_mirror.rs`, `interface/options.rs`, `interface/startup_menu.rs` (`offer_catalog` n'est plus appelé, les boutons « Catalog » des pages Mods / Packs ne s'affichent plus) |
| `crates/horizon-client-render/src/shader_ui.rs` | Ligne « Catalog » de la page des shaders retirée (`Item::Catalog`) |
| `Cargo.toml` (racine), `crates/horizon-client/Cargo.toml` | `starling-catalog` retiré du workspace et des dépendances ; `launcher` retiré de `exclude` |
| `packaging/macos/`, `packaging/linux/appimage.sh`, `packaging/linux/horizon-launcher.desktop`, `packaging/fork-launcher-release-notes.md` | Supprimés (empaquetage du launcher PC) |
| `tools/` : `configure_fork_launcher_workflow.py`, `fork_launcher_identity.py`, `generate_fork_launcher_icons.py`, `launcher_*.py`, `stage_launcher_packaging.py`, `test_launcher_*.py`, `ci/install_launcher_deps.sh`, `ci/ignored-tests/*launcher*.txt` | Supprimés |
| `tests/fixtures/launcher-upstream/`, `tests/test_ci_apt_retry.py` | Supprimés ; `tests/test_native_mod_targets.py` et `tools/ci/test_scope.py` ne parlent plus de `starling-catalog` |
| `flake.nix`, `.gitattributes`, `.github/CODEOWNERS` | Paquet Nix `horizon-launcher` et lignes sur `launcher/` retirés |

Restent des mentions de `launcher/` dans des commentaires et la doc d'Horizon (`docs/`), sans
effet. Le catalogue en jeu reviendra par l'API générique (voir ci-dessous) : son code d'origine
est au tag `v0.9.0`, dans `launcher/src/browser/` et `crates/starling-catalog/`.

**L'API générique du moteur** (2026-11-22) : nouvelle crate `crates/horizon-engine-api`, membre
du workspace (détail : [`API-MOTEUR.md`](API-MOTEUR.md)). Dans leur code :
`crates/horizon-client-render/Cargo.toml` en dépend ; `graphics.rs` (`FILE`), `own_skin.rs`
(`DIR`, `PROFILES`), `android_display.rs` (`ASPECTS`, clés des options d'affichage) prennent leurs
valeurs dans la crate, et `skins.rs` réexporte `process_legacy_skin` / `process_cape`, déplacées
dedans.

**Play Store (2026-11-22)** : plus de code téléchargé dans l'APK.

| Fichier | Changement |
|---|---|
| `crates/horizon-client/src/game/starling/native_builds.rs`, `build_mining.rs` | Supprimés (builds de mods natifs et leur mini-jeu d'attente) ; appels retirés de `game.rs`, `runtime/window_handler.rs`, `starling/input_mode.rs`, `starling/touch/mod.rs`, `modding/mods/mod.rs` |
| `crates/horizon-client/src/android_lib.rs` | Plus d'ouverture des bibliothèques de `native-libs` au démarrage |
| `crates/horizon-client/src/android.rs` | `on_create` sans relais vers un build téléchargé (`STARLING_GAME_LIBRARY`), plus de `set_native_libs_opened` ; chargeur de pilotes GPU (adrenotools, `load_custom_driver`) retiré |
| `crates/horizon-client/src/import.rs`, `game/starling/imports.rs` | `.adpkg` lu comme fichier inconnu, mod natif refusé avec un message |
| `crates/horizon-client-render/src/graphics_ui.rs` | Ligne « GPU Drivers... » jamais affichée |
| `packaging/android/java/.../StarlingActivity.java`, `NativeMods.java` (supprimé) | `loadGame` charge toujours la bibliothèque de l'APK ; demande de permissions des mods et serveur de build retirés |

**Ligne « Launcher » du menu pause** : `crates/horizon-client/src/game/interface/screens.rs`
(ligne 31, si `HORIZON_LAUNCHER_BUTTON=1`) ; `crates/horizon-client/Cargo.toml` dépend de
`horizon-engine-api`.

### Ce qu'on reprendra de leur launcher

Décidé fonction par fonction le 2026-11-22. Le code d'origine est au tag `v0.9.0` de leur dépôt,
dans `crates/horizon-client/src/ha_launcher/`.

- **Fonctions génériques du moteur** (sans écran, appelées par notre launcher) : catalogue
  (registre Horizon + Modrinth) et images des projets ; garde-robe (bibliothèque, aperçu 3D) ;
  résolution d'image globale ; réglages du téléphone (fréquence d'images, gyroscope, ADPF) ;
  proxy ; thème ; logique du gestionnaire de fichiers.
- **Refaits dans notre launcher** : contenu installé, interface du gestionnaire de fichiers.
- **Gardés dans le jeu** (ils y sont déjà) : conversion des mods Java, captures F2 vers la galerie.
- **Retirés pour de bon** : profils, écran de comptes, écran d'installation et de versions.

Pas repris pour l'instant dans notre manifeste : `GameFolderProvider` (le dossier du jeu dans
l'appli Fichiers d'Android) et les filtres d'ouverture de `LauncherActivity` (`.starling`,
`.hmod`, `.adpkg`), qui iront avec le gestionnaire de fichiers.

## Nos deux correctifs

Tous deux conditionnés à `accounts::launched_by_yuyuframe()` (variable `YUYUFRAME=1`, que le
launcher écrit dans `horizon-args.txt`) : lancé autrement, le jeu se comporte comme Starling.

| Fichier | Changement | Pourquoi |
|---|---|---|
| `crates/horizon-client/src/accounts.rs` | `launched_by_yuyuframe()` ; `state()` rend `SignedIn` avec le compte passé par le launcher (`crate::auth::current()`) quand Starling n'a pas de compte à lui | Leur écran de compte et le bouton de l'écran-titre ne lisent que le compte enregistré par Starling : ils affichaient « pas connecté » alors que le jeu jouait avec le compte du launcher |
| `crates/horizon-client/src/game/starling/setup/mod.rs` | Dans `poll_setup`, le téléchargement des fichiers du jeu lancé tout seul | Au premier démarrage, leur écran d'installation demandait de se connecter à **leur** compte (ou « Play offline ») avant de télécharger les fichiers de Minecraft. Lancé par YuyuFrame, le compte est déjà passé au jeu par `--session-file` |

## Le reste à nous

- `yuyu/` (ce dossier) : `build_engine.py`, qui compile le moteur avec leur propre script et le
  dépose dans l'appli, et ce fichier.
- Hors d'ici, dans `Launcher-Client/Backend/gen/android/starling/` : le module Gradle qui lit
  leur Java et leurs ressources, et `YuyuGameActivity` (hérite de leur `StarlingActivity`,
  change `releaseBuild` et `versionName`).

## Reporter leurs mises à jour

Plus de `git apply` direct : leur launcher a été retiré et le moteur va diverger (API générique,
iOS). Une mise à jour se reporte à la main :

1. Récupérer leur dépôt à côté : `git clone https://github.com/starling-client/starling.git <ailleurs>`.
2. Lire le diff depuis le tag d'origine : `git -C <ailleurs> diff v0.9.0..<nouveau tag>`, en
   écartant `ha_launcher/` et ce qui ne sert qu'à lui.
3. Reporter le reste, puis mettre à jour le tag, le commit et la version dans ce fichier.
4. Vérifier que le contrat dont dépend le launcher n'a pas bougé (voir
   `docs/launcher/android.md`) : `horizon-args.txt` lu par `crates/horizon-client/src/android.rs`
   (`command_line`), les options de `crates/horizon-client/src/game/runtime/launch_api.rs` et
   `launch_options.rs`, les méthodes `releaseBuild` et `versionName` de `StarlingActivity`, le
   manifeste de `packaging/android/AndroidManifest.xml` (à reporter dans celui du module
   `starling`).
