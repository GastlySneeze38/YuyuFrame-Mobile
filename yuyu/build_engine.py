#!/usr/bin/env python3
"""Compile le moteur Horizon pour Android et le dépose dans l'appli YuyuFrame.

Le seul fichier à nous dans horizon-android/ avec UPSTREAM.md : tout le reste est le code de
Starling, inchangé. Ce script réutilise les fonctions de leur propre construction
(packaging/android/build_apk.py), sans en recopier aucune, et range le résultat dans le module
Gradle `starling` du launcher (Launcher-Client/Backend/gen/android/starling/src/main/) :

- jniLibs/arm64-v8a/ : libhorizon_android.so (le jeu), libstarling_java.so et
  libstarling_javac.so (les lanceurs Java qui préparent les fichiers du jeu sur le téléphone),
  libc++_shared.so ;
- assets/ : prepare-kit.zip, lowbudget.zip, StarlingVisuals.zip, diagnostic-fonts/ ;
- res/values/ : starling-colors.xml (couleurs de leur thème, que leur code Java attend) et
  yuyu-engine.xml (la version du moteur, lue par YuyuGameActivity et GamePlugin).

    python horizon-android/yuyu/build_engine.py                  # compile puis dépose
    python horizon-android/yuyu/build_engine.py --profile release
    python horizon-android/yuyu/build_engine.py --no-build       # dépose le dernier build

Ensuite : `npm --prefix Launcher-Client/Frontend run tauri android build` (ou `dev`).

Il faut le SDK Android (ANDROID_HOME, ici D:\\Android\\Sdk) avec le NDK et les build-tools que
fixe build_apk.py, la cible Rust aarch64-linux-android et cargo-ndk.
"""

import argparse
import importlib.util
import re
import shutil
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
ENGINE = HERE.parent
MODULE = ENGINE.parent / "Launcher-Client" / "Backend" / "gen" / "android" / "starling" / "src" / "main"


def load_build_apk():
    """Leur script, chargé comme un module : ses fonctions, ses versions épinglées."""
    path = ENGINE / "packaging" / "android" / "build_apk.py"
    spec = importlib.util.spec_from_file_location("starling_build_apk", path)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def theme_colors():
    """Les couleurs de leur thème sombre, comme les écrit link_resources de build_apk.py."""
    theme = (ENGINE / "crates/horizon-theme/src/lib.rs").read_text(encoding="utf-8")
    dark = theme.split("pub const DARK: Palette = Palette {", 1)[1].split("};", 1)[0]
    colors = dict(re.findall(r"(\w+): 0x([0-9a-fA-F]{6})", dark))
    entries = "\n".join(
        f'<color name="starling_{name}">#{colors[name]}</color>'
        for name in ("page", "rule", "ink", "ink_soft", "accent", "on_accent")
    )
    return f"<resources>\n{entries}\n</resources>\n"


def main():
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--profile", default="quick", help="profil Cargo (quick pour essayer, release pour publier)")
    parser.add_argument("--no-build", action="store_true", help="déposer le dernier build sans recompiler")
    parser.add_argument("--keep-symbols", action="store_true", help="garder les symboles (traces de plantage)")
    args = parser.parse_args()

    apk = load_build_apk()
    sdk = apk.sdk_dir()
    ndk = sdk / "ndk" / apk.NDK
    if not ndk.exists():
        sys.exit(f"build_engine: NDK {apk.NDK} absent de {sdk} (ANDROID_HOME)")

    if not args.no_build:
        apk.build_library(args.profile, ndk, sdk)
    profile_dir = "debug" if args.profile == "dev" else args.profile
    built = apk.ROOT / "target" / apk.TRIPLE / profile_dir / f"lib{apk.LIBRARY}.so"
    if not built.is_file():
        sys.exit(f"build_engine: {built} n'a pas été compilé")

    out = apk.ROOT / "target" / "android"
    out.mkdir(parents=True, exist_ok=True)
    library = out / f"lib{apk.LIBRARY}.so"
    if args.keep_symbols:
        shutil.copyfile(built, library)
    else:
        apk.run(apk.ndk_tool(ndk, "llvm-strip"), "--strip-unneeded", "-o", library, built)

    libraries = apk.build_launchers(ndk, out)
    libraries[library.name] = library
    jni = MODULE / "jniLibs" / apk.ABI
    shutil.rmtree(jni, ignore_errors=True)
    jni.mkdir(parents=True)
    for name, path in sorted(libraries.items()):
        shutil.copyfile(path, jni / name)

    assets = apk.assets(out)
    target_assets = MODULE / "assets"
    shutil.rmtree(target_assets, ignore_errors=True)
    shutil.copytree(assets, target_assets)

    values = MODULE / "res" / "values"
    values.mkdir(parents=True, exist_ok=True)
    (values / "starling-colors.xml").write_text(theme_colors(), encoding="utf-8")
    (values / "yuyu-engine.xml").write_text(
        "<resources>\n"
        f'    <string name="yuyu_engine_version" translatable="false">{apk.VERSION_NAME}</string>\n'
        "</resources>\n",
        encoding="utf-8",
    )
    print(f"build_engine: moteur Starling {apk.VERSION_NAME} déposé dans {MODULE}")


if __name__ == "__main__":
    main()
