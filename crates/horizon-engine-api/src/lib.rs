//! What a launcher needs to drive the engine, with no screen of its own.
//!
//! The engine and a launcher talk through **a launch line and the game folder**, never through a
//! call into the running game: on Android the game runs in a process of its own, on iOS it will
//! share the app's, and a launcher may be written in any language. This crate is that contract,
//! written once in Rust:
//!
//! - [`launch`]: the options the engine starts with (`--game-dir`, `--session-file`...), the
//!   session file, and `horizon-args.txt`, the file Android's activity reads them from;
//! - [`settings`]: the settings a launcher may change, in the files the engine reads them from
//!   (`options.txt`, `horizon-graphics.txt`, `horizon-touch.json`), every other line kept;
//! - [`skin`]: the player's skin as the engine wears it (`skins/` and `horizon-skins.json`), and
//!   the skin and cape layouts the engine and a preview both use;
//! - [`wardrobe`]: a library of skins and capes a launcher keeps for its players, and
//!   [`preview`], a 3D picture of a skin with no GPU;
//! - [`files`]: the folders of a game folder a player may see and change, with no way out of them.
//!
//! Nothing here starts the engine or draws: a launcher shows its own screens and starts the game
//! its own way (an Android activity, a process, a call on iOS) with [`launch::Launch::arguments`].
//! The names of the files are in [`contract`]; the engine takes them from here, so the two sides
//! cannot drift apart.

pub mod files;
pub mod launch;
pub mod preview;
pub mod settings;
pub mod skin;
pub mod text;
pub mod wardrobe;

/// The files of a game folder the engine reads, relative to it.
pub mod contract {
    /// Vanilla's options, `key:value` lines; the engine's Android display options live here too.
    pub const OPTIONS: &str = "options.txt";
    /// Horizon's own graphics settings, `key:value` lines.
    pub const GRAPHICS: &str = "horizon-graphics.txt";
    /// The touch controls and their settings (the gyroscope among them), one JSON object.
    pub const TOUCH: &str = "horizon-touch.json";
    /// The skins to choose from, PNG files.
    pub const SKINS: &str = "skins";
    /// Each profile's skin choice, one JSON object by profile key.
    pub const SKIN_PROFILES: &str = "horizon-skins.json";
    /// Android: the launch line, beside the app's private files (not in a game folder).
    pub const ARGS: &str = "horizon-args.txt";
}

/// Writes `bytes` to `path` in one step: a file beside it, then a rename, so the engine never reads
/// half of it.
pub(crate) fn write_atomic(path: &std::path::Path, bytes: &[u8]) -> std::io::Result<()> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let mut partial = path.as_os_str().to_owned();
    partial.push(".partial");
    std::fs::write(&partial, bytes)?;
    std::fs::rename(&partial, path)
}

#[cfg(test)]
pub(crate) fn scratch(name: &str) -> std::path::PathBuf {
    use std::sync::atomic::{AtomicU32, Ordering};
    static NEXT: AtomicU32 = AtomicU32::new(0);
    let dir = std::env::temp_dir().join(format!(
        "horizon-engine-api-{name}-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}
