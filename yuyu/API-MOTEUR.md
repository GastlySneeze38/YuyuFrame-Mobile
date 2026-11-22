# L'API du moteur pour un launcher

Le moteur (Horizon pour Android, venu de Starling) n'a **aucun écran de launcher**. Un launcher
le pilote par **une ligne de lancement et le dossier de jeu**, jamais par un appel dans le jeu en
cours : sur Android le jeu tourne dans son propre processus, sur iOS il partagera celui de
l'appli, et un launcher peut être écrit dans n'importe quel langage.

Ce contrat est écrit une fois, en Rust, dans la crate **`horizon-engine-api`**
(`crates/horizon-engine-api/`). Elle est légère (serde, sha2, image en PNG seulement) : ni rendu,
ni monde, ni fenêtre. Un launcher en Rust l'utilise directement ; un autre suit ce document.
Le moteur prend lui aussi ses noms de fichiers et ses formats de skin dans la crate
(`horizon-client-render` en dépend), donc les deux côtés ne peuvent pas diverger.

Rien dans la crate ne démarre le jeu ni ne dessine : le launcher montre ses propres écrans et
démarre le jeu à sa façon (écran Android, processus, appel sur iOS).

## Lancement (`launch`)

Les options du contrat de lancement d'Horizon (`docs/api/launch.md`) :

| Option | Valeur |
|---|---|
| `--game-dir` | le dossier de jeu : mondes, options, mods, packs |
| `--home` | les fichiers du moteur pour ce dossier : journaux, rapports de plantage |
| `--session-file` | le compte connecté (fichier ci-dessous) |
| `--name` | un pseudo hors ligne, à la place du fichier de session |
| `--quickPlayMultiplayer` | un serveur à rejoindre au démarrage |

- **Fichier de session** (`write_session_file`) : JSON `name`, `uuid` (sans tirets),
  `access_token`, `user_type` (`msa`), `xuid`. Nom imprévisible, créé neuf, lisible par son seul
  propriétaire ; le moteur le lit une fois puis l'efface, et refuse un fichier de plus de deux
  minutes. Le launcher l'efface lui-même si le jeu n'a pas démarré.
- **Android** : `horizon-args.txt` dans les fichiers privés de l'appli (`Launch::write_args_file`),
  lu par l'écran du jeu comme sa ligne de commande : une option par ligne, ou `NOM=valeur` pour une
  variable d'environnement (`A-Z`, `0-9`, `_`). Écrit d'un coup ; un saut de ligne dans une valeur
  est refusé.

## Réglages (`settings`)

Lus et écrits dans les fichiers que le jeu lit déjà. `None` = non réglé (le jeu prend son défaut,
qui peut dépendre du téléphone). `Settings::apply` n'écrit que ce qui est `Some` et garde toutes
les autres lignes et clés. **Jamais pendant que le jeu tourne dans ce dossier** : il réécrit ces
fichiers en quittant.

| Réglage | Fichier | Clé | Plage |
|---|---|---|---|
| Résolution | `horizon-graphics.txt` | `resolutionScale` | 25 à 200 % (copie dans `horizon-touch.json`, `resolution_scale_percent`, 25 à 100) |
| Images/s max | `options.txt` | `maxFps` | 10 à 260 par dizaines, 260 = illimité ; la fréquence de l'écran et l'indice de performance d'Android (ADPF) suivent |
| Mode batterie | `horizon-graphics.txt` | `batteryMode` | `true` / `false` |
| Encoche | `options.txt` | `starlingDisplayCutout` | 0 plein écran, 1 éviter l'encoche |
| Orientation | `options.txt` | `starlingDisplayOrientation` | 0 paysage, 1 portrait, 2 les deux |
| Rotation | `options.txt` | `starlingDisplayRotation` | 0 auto, 1 auto inversée, 2 fixe, 3 fixe inversée |
| Format | `options.txt` | `starlingDisplayAspect` | index de `ASPECTS` (0 = celui de l'écran) |
| Gyroscope | `horizon-touch.json` | `gyroscope` | `true` / `false` |
| Sensibilité du gyroscope | `horizon-touch.json` | `gyro_sensitivity` | 0,1 à 3,0 |

## Skin (`skin`)

Le jeu (page Skin) porte un PNG de `skins/` choisi par profil dans `horizon-skins.json` :
`{"<clé du profil>": {"wardrobe": {"skin": "a.png", "model": "slim"}}}`, `skin` à `null` pour le
skin du compte (ou celui par défaut). Clé : celle de Starling (UUID sans tirets d'un profil
Microsoft, `ely<uuid>` pour Ely.by) ; **`""` quand le jeu est lancé avec `--session-file` ou
`--name`** sans profil Starling, ce qui est le cas de YuyuFrame. `wear` vérifie le PNG (64×64 ou
64×32, 1 Mio au plus), le réécrit sans métadonnées sous un nom tiré de son empreinte, et le
choisit sans toucher aux autres profils.

La crate porte aussi les conversions communes au jeu et à un aperçu : `process_legacy_skin`
(64×32 → 64×64, comme vanilla) et `process_cape`.

## Garde-robe et aperçu (`wardrobe`, `preview`)

- **Bibliothèque** d'un launcher, dans un dossier à lui : `wardrobe.json` + `images/<sha-256>.png`.
  Importer (vérifié, réécrit), retirer, choisir par compte (skin, cape, modèle), puis
  `prepare_game` avant un lancement, qui passe le skin choisi à `wear`. Locale : n'appelle aucun
  service de compte.
- **Aperçu 3D** sans carte graphique : `preview::render(skin, cape, modèle, caméra, taille)` rend
  une image RGBA (les deux couches du skin et la cape en vrai volume), `Camera::drag` / `zoom`
  pour la faire tourner au doigt.

Les capes de la bibliothèque ne sont **pas encore portées en jeu** : le moteur ne lit pas de cape
locale (la garde-robe en jeu de Starling a été retirée).

## Fichiers (`files`)

Les dossiers qu'un joueur peut voir et modifier, sans jamais en sortir : mods (`horizon-mods`),
mods natifs, packs de ressources, shaders, mondes (`saves`), captures, skins, et la configuration
(`options.txt`, `horizon-graphics.txt`, `horizon-touch.json`, `horizon-keypad.json`, en lecture
de liste seulement). Un nom est un seul élément de chemin (ni `..`, ni séparateur, ni point en
tête) et un lien symbolique est refusé. Opérations : `list`, `remove`, `rename` (jamais par-dessus
un fichier), `create_dir`, `import` (copie sans écraser : `nom (2).zip`…).

## Pas encore dans l'API

- **Catalogue** (registre Horizon + Modrinth, images des projets) : le registre Horizon est déjà
  fait par notre launcher (`Launcher-Client/Backend/src/minecraft/launcher/horizon/content.rs`,
  aussi sur Android) ; à déplacer ici, avec Modrinth, pour n'en garder qu'une implémentation.
- **Thème** : le jeu n'a aujourd'hui que celui des commandes tactiles ; ce que les couleurs d'un
  launcher doivent changer en jeu reste à décider.
- **Interface C** pour un launcher non Rust (Swift sur iOS) : quand il en faudra une.

## Qui s'en sert

- Le moteur : `horizon-client-render` (`graphics::FILE`, `own_skin::DIR` / `PROFILES`,
  `android_display` : `ASPECTS` et clés, `skins::process_legacy_skin` / `process_cape`).
- YuyuFrame : `Launcher-Client/Backend` (Android seulement), dépendance Git sur le dépôt public
  `YuyuFrame-Mobile`, remplacée en local par `horizon-android/` (`.cargo/config.toml` à la racine du
  monorepo, hors du dépôt public du launcher). `minecraft/launcher/horizon/android.rs` lance par
  `launch`. **Publier `YuyuFrame-Mobile` avant le launcher**, sinon sa CI ne trouve pas la crate.
