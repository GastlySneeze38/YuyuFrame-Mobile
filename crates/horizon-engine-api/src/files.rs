//! The folders of a game folder a player may see and change from a launcher's file manager, and
//! the few operations on them, with no way out: a name is one plain path component (no `..`, no
//! separator, no leading dot), and a symbolic link anywhere on the way is refused.
//!
//! The engine's own files (its caches, prepared resources, logs and native builds) are never shown:
//! only [`Folder`]s are. Adapted from Starling's launcher (`ha_launcher/document_store.rs`, v0.9.0).
use crate::contract;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::{self, ErrorKind};
use std::path::{Path, PathBuf};

/// A folder of the game folder a player may open.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Folder {
    /// Horizon's content and WebAssembly mods (`horizon-mods/`).
    Mods,
    /// Native mods (`horizon-native-mods/`).
    NativeMods,
    ResourcePacks,
    ShaderPacks,
    /// Worlds (`saves/`).
    Worlds,
    Screenshots,
    /// The skins Skin Customization offers (`skins/`).
    Skins,
    /// The settings files a launcher shows: [`CONFIGURATION`] only, no folder.
    Configuration,
}

impl Folder {
    pub const ALL: [Folder; 8] = [
        Folder::Mods,
        Folder::NativeMods,
        Folder::ResourcePacks,
        Folder::ShaderPacks,
        Folder::Worlds,
        Folder::Screenshots,
        Folder::Skins,
        Folder::Configuration,
    ];

    /// Its path in the game folder (`""` for the configuration, which is the game folder's own files).
    pub fn dir(self) -> &'static str {
        match self {
            Folder::Mods => "horizon-mods",
            Folder::NativeMods => "horizon-native-mods",
            Folder::ResourcePacks => "resourcepacks",
            Folder::ShaderPacks => "shaderpacks",
            Folder::Worlds => "saves",
            Folder::Screenshots => "screenshots",
            Folder::Skins => contract::SKINS,
            Folder::Configuration => "",
        }
    }
}

/// The settings files [`Folder::Configuration`] shows.
pub const CONFIGURATION: [&str; 4] = [contract::OPTIONS, contract::GRAPHICS, contract::TOUCH, "horizon-keypad.json"];

/// A file or folder of a listing.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Item {
    pub name: String,
    pub dir: bool,
    pub size: u64,
    /// Milliseconds since 1970, 0 when unknown.
    pub modified: u64,
}

const MAX_ITEMS: usize = 32768;

fn invalid() -> io::Error {
    io::Error::new(ErrorKind::InvalidInput, "invalid file name")
}

/// Whether `name` is one plain path component a player may give.
pub fn valid_name(name: &str) -> bool {
    !(name.is_empty()
        || name == "."
        || name == ".."
        || name.starts_with('.')
        || name.len() > 240
        || name.chars().any(|c| c.is_control() || "/\\:*?\"<>|".contains(c)))
}

/// The path of `parts` (names, one folder deep each) in `folder` of the game folder `game`.
pub fn resolve(game: &Path, folder: Folder, parts: &[&str]) -> io::Result<PathBuf> {
    if parts.iter().any(|p| !valid_name(p)) {
        return Err(invalid());
    }
    if folder == Folder::Configuration && (parts.len() > 1 || parts.first().is_some_and(|p| !CONFIGURATION.contains(p))) {
        return Err(invalid());
    }
    let mut path = game.join(folder.dir());
    let mut checked = game.to_owned();
    for part in std::iter::once(folder.dir()).filter(|d| !d.is_empty()).chain(parts.iter().copied()) {
        checked.push(part);
        match fs::symlink_metadata(&checked) {
            Ok(meta) if meta.file_type().is_symlink() => return Err(invalid()),
            Ok(_) => {}
            Err(e) if e.kind() == ErrorKind::NotFound => {}
            Err(e) => return Err(e),
        }
    }
    for part in parts {
        path.push(part);
    }
    Ok(path)
}

/// What `parts` of `folder` holds, folders first, then by name. The folder itself is made if
/// missing; the configuration lists only its files that exist.
pub fn list(game: &Path, folder: Folder, parts: &[&str]) -> io::Result<Vec<Item>> {
    let dir = resolve(game, folder, parts)?;
    if folder == Folder::Configuration {
        if !parts.is_empty() {
            return Err(invalid());
        }
        return Ok(CONFIGURATION.iter().filter_map(|name| item(&dir.join(name), name)).collect());
    }
    if parts.is_empty() {
        fs::create_dir_all(&dir)?;
    }
    let mut items = Vec::new();
    for entry in fs::read_dir(&dir)?.take(MAX_ITEMS) {
        let entry = entry?;
        let Some(name) = entry.file_name().to_str().map(str::to_owned) else { continue };
        if !valid_name(&name) {
            continue;
        }
        if let Some(item) = item(&entry.path(), &name) {
            items.push(item);
        }
    }
    items.sort_by(|a, b| b.dir.cmp(&a.dir).then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase())));
    Ok(items)
}

fn item(path: &Path, name: &str) -> Option<Item> {
    let meta = fs::symlink_metadata(path).ok()?;
    if !meta.is_dir() && !meta.is_file() {
        return None;
    }
    Some(Item {
        name: name.to_owned(),
        dir: meta.is_dir(),
        size: if meta.is_file() { meta.len() } else { 0 },
        modified: meta
            .modified()
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0),
    })
}

/// Deletes a file or a whole folder of `folder`; never the folder itself, nor a configuration file.
pub fn remove(game: &Path, folder: Folder, parts: &[&str]) -> io::Result<()> {
    if parts.is_empty() || folder == Folder::Configuration {
        return Err(invalid());
    }
    let path = resolve(game, folder, parts)?;
    if fs::symlink_metadata(&path)?.is_dir() {
        fs::remove_dir_all(path)
    } else {
        fs::remove_file(path)
    }
}

/// Renames the last of `parts` to `name`, in the same folder; never over something that exists.
pub fn rename(game: &Path, folder: Folder, parts: &[&str], name: &str) -> io::Result<()> {
    if parts.is_empty() || folder == Folder::Configuration || !valid_name(name) {
        return Err(invalid());
    }
    let from = resolve(game, folder, parts)?;
    let to = from.with_file_name(name);
    if fs::symlink_metadata(&to).is_ok() {
        return Err(ErrorKind::AlreadyExists.into());
    }
    fs::rename(from, to)
}

/// Makes the folder `parts` in `folder`.
pub fn create_dir(game: &Path, folder: Folder, parts: &[&str]) -> io::Result<()> {
    if parts.is_empty() || folder == Folder::Configuration {
        return Err(invalid());
    }
    fs::create_dir_all(resolve(game, folder, parts)?)
}

/// Copies the file `source` into `parts` of `folder` as `name`, never over a file that exists:
/// the name gets ` (2)`, ` (3)`... before its extension. Returns the name it took.
pub fn import(game: &Path, folder: Folder, parts: &[&str], source: &Path, name: &str) -> io::Result<String> {
    import_from(game, folder, parts, &mut fs::File::open(source)?, name)
}

/// [`import`] from what `input` reads (a file the system's picker opened, bytes a launcher got).
pub fn import_from(
    game: &Path,
    folder: Folder,
    parts: &[&str],
    input: &mut impl io::Read,
    name: &str,
) -> io::Result<String> {
    if folder == Folder::Configuration || !valid_name(name) {
        return Err(invalid());
    }
    let dir = resolve(game, folder, parts)?;
    fs::create_dir_all(&dir)?;
    let (stem, ext) = match name.rsplit_once('.') {
        Some((stem, ext)) if !stem.is_empty() => (stem, format!(".{ext}")),
        _ => (name, String::new()),
    };
    for n in 1..1000 {
        let candidate = if n == 1 { name.to_owned() } else { format!("{stem} ({n}){ext}") };
        let target = dir.join(&candidate);
        match fs::OpenOptions::new().write(true).create_new(true).open(&target) {
            Ok(mut out) => {
                let copied = io::copy(input, &mut out);
                if let Err(e) = copied {
                    drop(out);
                    let _ = fs::remove_file(&target);
                    return Err(e);
                }
                return Ok(candidate);
            }
            Err(e) if e.kind() == ErrorKind::AlreadyExists => continue,
            Err(e) => return Err(e),
        }
    }
    Err(ErrorKind::AlreadyExists.into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_the_players_folders_and_never_out_of_them() {
        let game = crate::scratch("files");
        fs::write(game.join(contract::OPTIONS), "maxFps:60\n").unwrap();
        fs::create_dir_all(game.join("cache")).unwrap();
        let source = game.join("cache/pack.zip");
        fs::write(&source, b"zip").unwrap();

        assert_eq!(import(&game, Folder::ResourcePacks, &[], &source, "pack.zip").unwrap(), "pack.zip");
        assert_eq!(import(&game, Folder::ResourcePacks, &[], &source, "pack.zip").unwrap(), "pack (2).zip");
        let names: Vec<_> = list(&game, Folder::ResourcePacks, &[]).unwrap().into_iter().map(|i| i.name).collect();
        assert_eq!(names, ["pack (2).zip", "pack.zip"]);
        rename(&game, Folder::ResourcePacks, &["pack (2).zip"], "other.zip").unwrap();
        assert!(rename(&game, Folder::ResourcePacks, &["other.zip"], "pack.zip").is_err());
        remove(&game, Folder::ResourcePacks, &["other.zip"]).unwrap();

        let config: Vec<_> = list(&game, Folder::Configuration, &[]).unwrap().into_iter().map(|i| i.name).collect();
        assert_eq!(config, [contract::OPTIONS]);
        assert!(remove(&game, Folder::Configuration, &[contract::OPTIONS]).is_err());
        assert!(resolve(&game, Folder::Configuration, &["cache"]).is_err());
        for bad in ["..", ".hidden", "a/b", "a\\b", ""] {
            assert!(resolve(&game, Folder::Worlds, &[bad]).is_err(), "{bad:?}");
        }
        assert!(remove(&game, Folder::Worlds, &[]).is_err());
        fs::remove_dir_all(game).unwrap();
    }
}
