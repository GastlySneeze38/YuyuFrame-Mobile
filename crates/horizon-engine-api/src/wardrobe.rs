//! A launcher's own library of skins and capes, kept in a folder of its choice (`root`):
//! `wardrobe.json` and `images/<sha-256>.png`. Local only: nothing here calls an account service or
//! changes a profile. What a game wears comes from [`Library::prepare_game`], which hands the
//! chosen skin to [`crate::skin::wear`].
//!
//! Adapted from Starling's launcher (`ha_launcher/skins/library.rs`, v0.9.0), without its
//! built-in capes and without `horizon-appearance.json`, which no engine reads.
use crate::skin::{self, Model};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Kind {
    Skin,
    Cape,
}

/// A picture of the library.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Entry {
    /// The SHA-256 of its PNG, also its file name.
    pub id: String,
    pub title: String,
    pub kind: Kind,
}

/// What an account wears, by the launcher's own account key.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Selection {
    pub skin: Option<String>,
    pub cape: Option<String>,
    pub model: Model,
    /// Hide even the account's own cape, locally only.
    #[serde(default)]
    pub hide_cape: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Library {
    pub schema: u32,
    pub entries: Vec<Entry>,
    pub selections: BTreeMap<String, Selection>,
}

impl Default for Library {
    fn default() -> Self {
        Self {
            schema: 1,
            entries: vec![],
            selections: Default::default(),
        }
    }
}

const INDEX: &str = "wardrobe.json";

impl Library {
    pub fn load(root: &Path) -> Result<Self, String> {
        let data = match std::fs::read(root.join(INDEX)) {
            Ok(data) => data,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Self::default()),
            Err(_) => return Err("Could not read the wardrobe.".into()),
        };
        if data.len() > 2 * 1024 * 1024 {
            return Err("The wardrobe index is too large.".into());
        }
        let value: Self = serde_json::from_slice(&data).map_err(|_| "The wardrobe index is damaged.")?;
        if value.schema != 1 {
            return Err("This wardrobe needs a newer launcher.".into());
        }
        if value.entries.iter().any(|e| !valid_id(&e.id)) {
            return Err("The wardrobe holds an invalid picture name.".into());
        }
        Ok(value)
    }

    pub fn save(&self, root: &Path) -> Result<(), String> {
        let bytes = serde_json::to_vec_pretty(self).map_err(|_| "Could not encode the wardrobe.")?;
        crate::write_atomic(&root.join(INDEX), &bytes).map_err(|_| "Could not save the wardrobe.".into())
    }

    /// Where the picture `id` is; an `id` that is not a SHA-256 is refused, so no path leaves
    /// `images/`.
    pub fn image(root: &Path, id: &str) -> Result<PathBuf, String> {
        if !valid_id(id) {
            return Err("Invalid wardrobe picture name.".into());
        }
        Ok(root.join("images").join(format!("{id}.png")))
    }

    /// Adds a skin or cape (checked, then written again without metadata) and returns its id. The
    /// same picture twice is one entry.
    pub fn import(&mut self, root: &Path, title: &str, kind: Kind, bytes: &[u8]) -> Result<String, String> {
        let image = match kind {
            Kind::Skin => skin::read_skin(bytes)?,
            Kind::Cape => skin::read_cape(bytes)?,
        };
        let png = skin::encode_png(&image)?;
        let id = format!("{:x}", Sha256::digest(&png));
        crate::write_atomic(&Self::image(root, &id)?, &png).map_err(|_| "Could not keep this picture.")?;
        if !self.entries.iter().any(|e| e.id == id && e.kind == kind) {
            self.entries.push(Entry {
                id: id.clone(),
                title: title.chars().filter(|c| !c.is_control()).take(48).collect(),
                kind,
            });
        }
        self.save(root)?;
        Ok(id)
    }

    /// Removes a picture, and from every selection that wore it.
    pub fn remove(&mut self, root: &Path, id: &str) -> Result<(), String> {
        let path = Self::image(root, id)?;
        self.entries.retain(|e| e.id != id);
        for selection in self.selections.values_mut() {
            if selection.skin.as_deref() == Some(id) {
                selection.skin = None;
            }
            if selection.cape.as_deref() == Some(id) {
                selection.cape = None;
            }
        }
        self.save(root)?;
        match std::fs::remove_file(path) {
            Err(e) if e.kind() != std::io::ErrorKind::NotFound => Err(e.to_string()),
            _ => Ok(()),
        }
    }

    /// Chooses the picture `id` (`None`: the account's own) for `account`.
    pub fn select(&mut self, root: &Path, account: &str, kind: Kind, id: Option<&str>) -> Result<(), String> {
        if let Some(id) = id
            && !self.entries.iter().any(|e| e.id == id && e.kind == kind)
        {
            return Err("Choose a picture of the wardrobe.".into());
        }
        let selection = self.selections.entry(account.into()).or_default();
        match kind {
            Kind::Skin => selection.skin = id.map(str::to_owned),
            Kind::Cape => {
                selection.cape = id.map(str::to_owned);
                selection.hide_cape = false;
            }
        }
        self.save(root)
    }

    pub fn set_model(&mut self, root: &Path, account: &str, model: Model) -> Result<(), String> {
        self.selections.entry(account.into()).or_default().model = model;
        self.save(root)
    }

    /// Before a launch: the skin `account` chose here becomes the one the profile `key` wears in
    /// the game folder `game` ([`skin::wear`]). With no skin chosen, the profile wears its own.
    pub fn prepare_game(&self, root: &Path, account: &str, game: &Path, key: &str) -> Result<(), String> {
        let selection = self.selections.get(account).cloned().unwrap_or_default();
        let png = match &selection.skin {
            Some(id) => Some(std::fs::read(Self::image(root, id)?).map_err(|e| e.to_string())?),
            None => None,
        };
        skin::wear(game, key, png.as_deref(), selection.model)
    }
}

fn valid_id(id: &str) -> bool {
    id.len() == 64 && id.bytes().all(|b| b.is_ascii_hexdigit())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_picture_is_kept_chosen_and_worn_and_cannot_leave_its_folder() {
        let root = crate::scratch("wardrobe");
        let mut library = Library::default();
        let png = skin::encode_png(&image::RgbaImage::from_pixel(64, 64, image::Rgba([1, 2, 3, 255]))).unwrap();
        let id = library.import(&root, "Mine", Kind::Skin, &png).unwrap();
        assert_eq!(library.import(&root, "Again", Kind::Skin, &png).unwrap(), id);
        assert_eq!(library.entries.len(), 1);
        assert!(library.import(&root, "Cape?", Kind::Cape, &png).is_err());
        library.select(&root, "alex", Kind::Skin, Some(&id)).unwrap();
        assert!(library.select(&root, "alex", Kind::Skin, Some("missing")).is_err());
        assert!(Library::image(&root, "../private").is_err());

        let loaded = Library::load(&root).unwrap();
        assert_eq!(loaded.selections["alex"].skin.as_deref(), Some(id.as_str()));
        let game = root.join("game");
        loaded.prepare_game(&root, "alex", &game, "").unwrap();
        assert!(game.join(crate::contract::SKIN_PROFILES).is_file());

        library.remove(&root, &id).unwrap();
        assert!(library.selections["alex"].skin.is_none());
        assert!(!Library::image(&root, &id).unwrap().exists());
        std::fs::remove_dir_all(root).unwrap();
    }
}
