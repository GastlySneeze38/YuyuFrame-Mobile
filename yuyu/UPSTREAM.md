# D'où vient ce dossier

`horizon-android/` est une copie de **Starling** (portage Android d'Horizon, licence MIT),
prise sur la branche `starling` de [starling-client/starling](https://github.com/starling-client/starling).

| | |
|---|---|
| Commit d'origine | `121742ad8937455fb8718b526147b269f7be3b4c` |
| Date | 2026-11-08 |
| Message | Android: integrate diagnostics, early-crash recovery and preparation fixes (#272) |
| Version de l'appli Starling | 0.2.3 (`packaging/android/build_apk.py`, `VERSION_NAME`) |

Comme `server-pvp/` pour Pumpkin, le `.git` d'origine a été retiré : le code est suivi par le
monorepo.

## Ce qui est à nous

**Deux patchs dans leur code**, marqués « YuyuFrame », tous deux conditionnés à
`accounts::launched_by_yuyuframe()` (variable `YUYUFRAME=1`, que le launcher écrit dans
`horizon-args.txt`) : lancé autrement, le jeu se comporte exactement comme Starling.

| Fichier | Changement | Pourquoi |
|---|---|---|
| `crates/horizon-client/src/accounts.rs` | `launched_by_yuyuframe()` ; `state()` rend `SignedIn` avec le compte passé par le launcher (`crate::auth::current()`) quand Starling n'a pas de compte à lui | Leur écran de compte et le bouton de l'écran-titre ne lisent que le compte enregistré par Starling : ils affichaient « pas connecté » alors que le jeu jouait avec le compte du launcher |
| `crates/horizon-client/src/game/setup.rs` | Dans `poll_setup`, le téléchargement des fichiers du jeu lancé tout seul | Au premier démarrage, leur écran d'installation demandait de se connecter à **leur** compte (ou « Play offline ») avant de télécharger les fichiers de Minecraft. Lancé par YuyuFrame, le compte est déjà passé au jeu par `--session-file` : on lance directement le téléchargement, comme leur bouton « Play offline », qui ne touche pas au compte |

Le reste à nous :

- `yuyu/` (ce dossier) : `build_engine.py`, qui compile le moteur avec leur propre script et le
  dépose dans l'appli, et ce fichier.
- Hors d'ici, dans `Launcher-Client/Backend/gen/android/starling/` : le module Gradle qui lit
  leur Java et leurs ressources, et `YuyuGameActivity` (hérite de leur `StarlingActivity`,
  change `releaseBuild` et `versionName`).

Un fichier de leur code modifié un jour devra être listé ici, avec la raison.

## Reporter leurs mises à jour

1. Récupérer leur branche à côté : `git clone --branch starling https://github.com/starling-client/starling.git <ailleurs>`.
2. Comparer depuis le commit d'origine ci-dessus : `git -C <ailleurs> diff 121742ad..HEAD`.
3. Appliquer le diff ici (`git apply --directory=horizon-android`), puis mettre à jour le
   commit et la version dans ce fichier.
4. Vérifier que le contrat dont dépend le launcher n'a pas bougé (voir
   `docs/launcher/android.md`) : `horizon-args.txt` lu par `crates/horizon-client/src/android.rs`
   (`command_line`), les options de `crates/horizon-client/src/game/launch_api.rs`, les
   méthodes `releaseBuild` et `versionName` de `StarlingActivity`, le manifeste de
   `packaging/android/AndroidManifest.xml` (à reporter dans celui du module `starling`).
