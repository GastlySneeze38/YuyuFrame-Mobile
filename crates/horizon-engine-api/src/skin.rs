//! The player's skin as the engine wears it, and the skin and cape layouts.
//!
//! The engine (`own_skin`, Options > Skin Customization) wears a PNG of the game folder's `skins/`
//! for each profile, kept in `horizon-skins.json`:
//! `{"<profile key>": {"wardrobe": {"skin": "a.png", "model": "slim"}}}`, `skin` null for the
//! profile's own (its account's, else the default one). The key is Starling's profile key (a
//! Microsoft profile's UUID without dashes, `ely<uuid>` for Ely.by); with no Starling profile (a
//! game started by a launcher with `--session-file` or `--name`) it is `""`. [`wear`] writes there.
use crate::contract;
use serde::{Deserialize, Serialize};
use std::path::Path;

/// The largest PNG taken for a skin or a cape.
pub const MAX_BYTES: usize = 1024 * 1024;

/// The arms.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Model {
    /// Four-pixel arms ("Steve").
    #[default]
    Classic,
    /// Three-pixel arms ("Alex").
    Slim,
}

impl Model {
    pub fn name(self) -> &'static str {
        match self {
            Model::Classic => "classic",
            Model::Slim => "slim",
        }
    }
}

fn dimensions(bytes: &[u8]) -> Result<(u32, u32), String> {
    if bytes.len() > MAX_BYTES {
        return Err("The PNG must be at most 1 MiB.".into());
    }
    image::ImageReader::with_format(std::io::Cursor::new(bytes), image::ImageFormat::Png)
        .into_dimensions()
        .map_err(|_| "That isn't a PNG picture.".to_owned())
}

fn decode(bytes: &[u8]) -> Result<image::RgbaImage, String> {
    image::load_from_memory_with_format(bytes, image::ImageFormat::Png)
        .map(|i| i.to_rgba8())
        .map_err(|_| "That isn't a PNG picture.".to_owned())
}

/// A skin's picture, if it is one: 64×64 or the older 64×32.
pub fn read_skin(bytes: &[u8]) -> Result<image::RgbaImage, String> {
    match dimensions(bytes)? {
        (64, 64) | (64, 32) => decode(bytes),
        (w, h) => Err(format!(
            "A skin is 64×64 (or the older 64×32) pixels; this one is {w}×{h}."
        )),
    }
}

/// A cape's picture, if it is one: 64×32 or the older 22×17.
pub fn read_cape(bytes: &[u8]) -> Result<image::RgbaImage, String> {
    match dimensions(bytes)? {
        (64, 32) | (22, 17) => decode(bytes),
        (w, h) => Err(format!(
            "A cape is 64×32 (or the older 22×17) pixels; this one is {w}×{h}."
        )),
    }
}

/// Encodes `image` as a PNG with nothing else in it (no metadata, no trailing data).
pub fn encode_png(image: &image::RgbaImage) -> Result<Vec<u8>, String> {
    let mut out = std::io::Cursor::new(Vec::new());
    image
        .write_to(&mut out, image::ImageFormat::Png)
        .map_err(|_| "Could not encode PNG.".to_owned())?;
    Ok(out.into_inner())
}

/// Makes `png` the skin the profile `key` wears in the game folder `game` (`None`: the profile's
/// own, its account's or the default one). The picture is checked, written again without metadata
/// into `skins/` under a name made of its hash, and chosen in `horizon-skins.json`; every other
/// profile's entry stays. Not while the game runs in that folder: it writes the file too.
pub fn wear(game: &Path, key: &str, png: Option<&[u8]>, model: Model) -> Result<(), String> {
    use sha2::{Digest, Sha256};
    let file = match png {
        Some(bytes) => {
            let png = encode_png(&read_skin(bytes)?)?;
            let hash = format!("{:x}", Sha256::digest(&png));
            let name = format!("wardrobe-{}.png", &hash[..16]);
            let path = game.join(contract::SKINS).join(&name);
            crate::write_atomic(&path, &png).map_err(|e| e.to_string())?;
            Some(name)
        }
        None => None,
    };
    let path = game.join(contract::SKIN_PROFILES);
    let mut all = match std::fs::read(&path) {
        Ok(bytes) => match serde_json::from_slice(&bytes) {
            Ok(serde_json::Value::Object(all)) => all,
            _ => serde_json::Map::new(),
        },
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => serde_json::Map::new(),
        Err(e) => return Err(e.to_string()),
    };
    let entry = all.entry(key).or_insert_with(|| serde_json::json!({}));
    if !entry.is_object() {
        *entry = serde_json::json!({});
    }
    entry.as_object_mut().unwrap().insert(
        "wardrobe".into(),
        serde_json::json!({ "skin": file, "model": model.name() }),
    );
    let text = serde_json::to_string_pretty(&all).map_err(|e| e.to_string())? + "\n";
    crate::write_atomic(&path, text.as_bytes()).map_err(|e| e.to_string())
}

/// `SkinTextureDownloader.processLegacySkin` on RGBA pixels: 64x32 skins get the 64x64 layout,
/// and the opaque parts lose their alpha. `None` for any other size (the download fails).
pub fn process_legacy_skin(width: u32, height: u32, rgba: &[u8]) -> Option<Vec<u8>> {
    if width != 64 || height != 32 && height != 64 {
        return None;
    }
    let legacy = height == 32;
    let mut img = vec![0u8; 64 * 64 * 4];
    let n = rgba.len().min(img.len());
    img[..n].copy_from_slice(&rgba[..n]);
    let at = |x: i32, y: i32| ((y * 64 + x) * 4) as usize;
    if legacy {
        let rects = [
            (4, 16, 16, 32, 4, 4),
            (8, 16, 16, 32, 4, 4),
            (0, 20, 24, 32, 4, 12),
            (4, 20, 16, 32, 4, 12),
            (8, 20, 8, 32, 4, 12),
            (12, 20, 16, 32, 4, 12),
            (44, 16, -8, 32, 4, 4),
            (48, 16, -8, 32, 4, 4),
            (40, 20, 0, 32, 4, 12),
            (44, 20, -8, 32, 4, 12),
            (48, 20, -16, 32, 4, 12),
            (52, 20, -8, 32, 4, 12),
        ];
        for (sx, sy, ox, oy, w, h) in rects {
            for y in 0..h {
                for x in 0..w {
                    let (s, d) = (at(sx + x, sy + y), at(sx + ox + w - 1 - x, sy + oy + y));
                    let p: [u8; 4] = img[s..s + 4].try_into().unwrap();
                    img[d..d + 4].copy_from_slice(&p);
                }
            }
        }
    }
    let no_alpha = |img: &mut Vec<u8>, x0: i32, y0: i32, x1: i32, y1: i32| {
        for x in x0..x1 {
            for y in y0..y1 {
                img[at(x, y) + 3] = 255;
            }
        }
    };
    no_alpha(&mut img, 0, 0, 32, 16);
    if legacy {
        // doNotchTransparencyHack: a fully opaque hat layer is cleared.
        let opaque = (32..64).all(|x| (0..32).all(|y| img[at(x, y) + 3] >= 128));
        if opaque {
            for x in 32..64 {
                for y in 0..32 {
                    img[at(x, y) + 3] = 0;
                }
            }
        }
    }
    no_alpha(&mut img, 0, 16, 64, 32);
    no_alpha(&mut img, 16, 48, 48, 64);
    Some(img)
}

/// A cape in a 64x64 skin slot: the 64x32 picture (or the older 22x17, which has the same layout)
/// in its top half. Vanilla draws a cape of any size; a larger one is scaled down to 64x32 here.
pub fn process_cape(width: u32, height: u32, rgba: &[u8]) -> Option<Vec<u8>> {
    let (w, h) = (width as usize, height as usize);
    if w == 0 || h == 0 || rgba.len() < w * h * 4 {
        return None;
    }
    let mut img = vec![0u8; 64 * 64 * 4];
    let (fit_w, fit_h) = if w <= 64 && h <= 32 { (w, h) } else { (64, 32) };
    for y in 0..fit_h {
        for x in 0..fit_w {
            let (sx, sy) = (x * w / fit_w, y * h / fit_h);
            let (s, d) = ((sy * w + sx) * 4, (y * 64 + x) * 4);
            img[d..d + 4].copy_from_slice(&rgba[s..s + 4]);
        }
    }
    Some(img)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_worn_skin_is_where_the_engine_reads_it_and_others_stay() {
        let game = crate::scratch("wear");
        std::fs::write(
            game.join(contract::SKIN_PROFILES),
            r#"{"abc":{"skin":"mine.png","model":"classic"}}"#,
        )
        .unwrap();
        let png = encode_png(&image::RgbaImage::from_pixel(64, 64, image::Rgba([9, 9, 9, 255]))).unwrap();
        wear(&game, "", Some(&png), Model::Slim).unwrap();
        let all: serde_json::Value =
            serde_json::from_slice(&std::fs::read(game.join(contract::SKIN_PROFILES)).unwrap()).unwrap();
        assert_eq!(all["abc"]["skin"], "mine.png");
        let file = all[""]["wardrobe"]["skin"].as_str().unwrap();
        assert_eq!(all[""]["wardrobe"]["model"], "slim");
        assert!(game.join(contract::SKINS).join(file).is_file());
        assert!(wear(&game, "", Some(b"not a png"), Model::Slim).is_err());
        wear(&game, "", None, Model::Classic).unwrap();
        let all: serde_json::Value =
            serde_json::from_slice(&std::fs::read(game.join(contract::SKIN_PROFILES)).unwrap()).unwrap();
        assert!(all[""]["wardrobe"]["skin"].is_null());
        std::fs::remove_dir_all(game).unwrap();
    }
}
