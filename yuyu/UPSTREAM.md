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

**Rien dans leur code.** Tout ce qui est à nous :

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
