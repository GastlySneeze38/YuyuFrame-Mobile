//! The settings a launcher may change for a game folder, in the files the engine reads them from.
//!
//! Each value is `None` when the file doesn't say (the engine then uses its own default, which can
//! depend on the phone), and a launcher writes only what it changes ([`Settings::apply`]): every
//! other line of `options.txt` and `horizon-graphics.txt`, and every other field of
//! `horizon-touch.json`, stays as the engine or the player left it.
//!
//! | Setting | File | Key | Range |
//! |---|---|---|---|
//! | [`resolution_scale`](Settings::resolution_scale) | `horizon-graphics.txt` | `resolutionScale` | 25 to 200 (%), also `resolution_scale_percent` in `horizon-touch.json`, 25 to 100 |
//! | [`max_fps`](Settings::max_fps) | `options.txt` | `maxFps` | 10 to 260, by tens; 260 is unlimited. Android's frame rate and ADPF hint follow it |
//! | [`battery_mode`](Settings::battery_mode) | `horizon-graphics.txt` | `batteryMode` | `true`, `false` |
//! | [`cutout`](Settings::cutout) | `options.txt` | `starlingDisplayCutout` | [`Cutout`] |
//! | [`orientation`](Settings::orientation) | `options.txt` | `starlingDisplayOrientation` | [`Orientation`] |
//! | [`rotation`](Settings::rotation) | `options.txt` | `starlingDisplayRotation` | [`Rotation`] |
//! | [`aspect`](Settings::aspect) | `options.txt` | `starlingDisplayAspect` | an index of [`ASPECTS`] |
//! | [`gyroscope`](Settings::gyroscope) | `horizon-touch.json` | `gyroscope` | `true`, `false` |
//! | [`gyro_sensitivity`](Settings::gyro_sensitivity) | `horizon-touch.json` | `gyro_sensitivity` | 0.1 to 3.0 |
use crate::contract;
use crate::text::KeyValues;
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Android's aspect ratio choices (`starlingDisplayAspect`): width, height, name; 0 is the screen's own.
pub const ASPECTS: [(i32, i32, &str); 11] = [
    (0, 0, "Native"),
    (1, 1, "1:1"),
    (5, 4, "5:4"),
    (4, 3, "4:3"),
    (3, 2, "3:2"),
    (16, 10, "16:10"),
    (16, 9, "16:9"),
    (18, 9, "18:9"),
    (39, 18, "19.5:9"),
    (20, 9, "20:9"),
    (21, 9, "21:9"),
];

/// The display options' keys in `options.txt`.
pub mod keys {
    pub const CUTOUT: &str = "starlingDisplayCutout";
    pub const ORIENTATION: &str = "starlingDisplayOrientation";
    pub const ROTATION: &str = "starlingDisplayRotation";
    pub const ASPECT: &str = "starlingDisplayAspect";
    pub const MAX_FPS: &str = "maxFps";
    pub const RESOLUTION_SCALE: &str = "resolutionScale";
    pub const BATTERY_MODE: &str = "batteryMode";
    pub const TOUCH_RESOLUTION: &str = "resolution_scale_percent";
    pub const GYROSCOPE: &str = "gyroscope";
    pub const GYRO_SENSITIVITY: &str = "gyro_sensitivity";
}

/// Whether the game draws under the camera's cutout.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Cutout {
    Full = 0,
    Avoid = 1,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Orientation {
    Landscape = 0,
    Portrait = 1,
    Both = 2,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Rotation {
    Automatic = 0,
    InvertedAutomatic = 1,
    Fixed = 2,
    FixedReversed = 3,
}

/// The settings of a game folder a launcher may read and change. `None`: not set, the engine's
/// default applies (reading), or left as it is (writing).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub resolution_scale: Option<u32>,
    pub max_fps: Option<u32>,
    pub battery_mode: Option<bool>,
    pub cutout: Option<Cutout>,
    pub orientation: Option<Orientation>,
    pub rotation: Option<Rotation>,
    pub aspect: Option<u32>,
    pub gyroscope: Option<bool>,
    pub gyro_sensitivity: Option<f32>,
}

impl Settings {
    /// What the game folder `game` holds now, read as the engine reads it (out-of-range values
    /// brought back in range).
    pub fn read(game: &Path) -> std::io::Result<Self> {
        let options = KeyValues::load(&game.join(contract::OPTIONS))?;
        let graphics = KeyValues::load(&game.join(contract::GRAPHICS))?;
        let touch = read_json(&game.join(contract::TOUCH))?;
        let int = |file: &KeyValues, key| file.int(key).map(|v| v.clamp(0, i64::from(u32::MAX)) as u32);
        Ok(Self {
            resolution_scale: int(&graphics, keys::RESOLUTION_SCALE).map(|v| v.clamp(25, 200)),
            max_fps: int(&options, keys::MAX_FPS).map(frame_limit),
            battery_mode: graphics.bool(keys::BATTERY_MODE),
            cutout: int(&options, keys::CUTOUT).map(|v| if v == 1 { Cutout::Avoid } else { Cutout::Full }),
            orientation: int(&options, keys::ORIENTATION).map(|v| match v.min(2) {
                0 => Orientation::Landscape,
                1 => Orientation::Portrait,
                _ => Orientation::Both,
            }),
            rotation: int(&options, keys::ROTATION).map(|v| match v.min(3) {
                0 => Rotation::Automatic,
                1 => Rotation::InvertedAutomatic,
                2 => Rotation::Fixed,
                _ => Rotation::FixedReversed,
            }),
            aspect: int(&options, keys::ASPECT).map(|v| v.min(ASPECTS.len() as u32 - 1)),
            gyroscope: touch.get(keys::GYROSCOPE).and_then(|v| v.as_bool()),
            gyro_sensitivity: touch
                .get(keys::GYRO_SENSITIVITY)
                .and_then(|v| v.as_f64())
                .map(|v| (v as f32).clamp(0.1, 3.0)),
        })
    }

    /// Writes the values that are `Some` into `game`'s files, in range, and nothing else. Not while
    /// the game runs in that folder: it writes these files too, and would undo the change.
    pub fn apply(&self, game: &Path) -> Result<(), String> {
        let io = |e: std::io::Error| e.to_string();
        let options_path = game.join(contract::OPTIONS);
        let mut options = KeyValues::load(&options_path).map_err(io)?;
        let mut options_changed = false;
        let mut set = |key: &str, value: String| -> Result<(), String> {
            options_changed = true;
            options.set(key, &value)
        };
        if let Some(fps) = self.max_fps {
            set(keys::MAX_FPS, frame_limit(fps).to_string())?;
        }
        if let Some(cutout) = self.cutout {
            set(keys::CUTOUT, (cutout as i32).to_string())?;
        }
        if let Some(orientation) = self.orientation {
            set(keys::ORIENTATION, (orientation as i32).to_string())?;
        }
        if let Some(rotation) = self.rotation {
            set(keys::ROTATION, (rotation as i32).to_string())?;
        }
        if let Some(aspect) = self.aspect {
            set(keys::ASPECT, aspect.min(ASPECTS.len() as u32 - 1).to_string())?;
        }
        if options_changed {
            options.save(&options_path).map_err(io)?;
        }

        if self.resolution_scale.is_some() || self.battery_mode.is_some() {
            let path = game.join(contract::GRAPHICS);
            let mut graphics = KeyValues::load(&path).map_err(io)?;
            if let Some(scale) = self.resolution_scale {
                graphics.set(keys::RESOLUTION_SCALE, &scale.clamp(25, 200).to_string())?;
            }
            if let Some(on) = self.battery_mode {
                graphics.set(keys::BATTERY_MODE, if on { "true" } else { "false" })?;
            }
            graphics.save(&path).map_err(io)?;
        }

        if self.resolution_scale.is_some() || self.gyroscope.is_some() || self.gyro_sensitivity.is_some() {
            let path = game.join(contract::TOUCH);
            let mut touch = read_json(&path).map_err(io)?;
            if let Some(scale) = self.resolution_scale {
                // The touch menu's copy of the same setting (it never goes above 100).
                touch.insert(keys::TOUCH_RESOLUTION.into(), scale.clamp(25, 100).into());
            }
            if let Some(on) = self.gyroscope {
                touch.insert(keys::GYROSCOPE.into(), on.into());
            }
            if let Some(sensitivity) = self.gyro_sensitivity {
                let value = if sensitivity.is_finite() { sensitivity.clamp(0.1, 3.0) } else { 1.0 };
                touch.insert(keys::GYRO_SENSITIVITY.into(), f64::from(value).into());
            }
            let bytes = serde_json::to_vec_pretty(&touch).map_err(|e| e.to_string())?;
            crate::write_atomic(&path, &bytes).map_err(io)?;
        }
        Ok(())
    }
}

/// `maxFps` as vanilla keeps it: 10 to 260, by tens.
fn frame_limit(fps: u32) -> u32 {
    (fps.clamp(10, 260) / 10) * 10
}

/// `horizon-touch.json` as an object; none (or not an object) reads as an empty one. The engine
/// fills what is missing with its defaults.
fn read_json(path: &Path) -> std::io::Result<serde_json::Map<String, serde_json::Value>> {
    match std::fs::read(path) {
        Ok(bytes) => Ok(serde_json::from_slice::<serde_json::Value>(&bytes)
            .ok()
            .and_then(|v| match v {
                serde_json::Value::Object(map) => Some(map),
                _ => None,
            })
            .unwrap_or_default()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Default::default()),
        Err(e) => Err(e),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_change_reaches_the_engines_files_and_leaves_the_rest() {
        let game = crate::scratch("settings");
        std::fs::write(game.join(contract::OPTIONS), "version:4671\nmaxFps:120\nfov:0.5\n").unwrap();
        std::fs::write(game.join(contract::GRAPHICS), "preset:2\nresolutionScale:100\n").unwrap();
        std::fs::write(game.join(contract::TOUCH), r#"{"vibrate":false,"gyroscope":false}"#).unwrap();
        Settings {
            resolution_scale: Some(150),
            max_fps: Some(65),
            gyroscope: Some(true),
            orientation: Some(Orientation::Both),
            ..Default::default()
        }
        .apply(&game)
        .unwrap();
        let options = std::fs::read_to_string(game.join(contract::OPTIONS)).unwrap();
        assert_eq!(options, "version:4671\nmaxFps:60\nfov:0.5\nstarlingDisplayOrientation:2\n");
        let graphics = std::fs::read_to_string(game.join(contract::GRAPHICS)).unwrap();
        assert_eq!(graphics, "preset:2\nresolutionScale:150\n");
        let touch: serde_json::Value =
            serde_json::from_slice(&std::fs::read(game.join(contract::TOUCH)).unwrap()).unwrap();
        assert_eq!(touch["vibrate"], false);
        assert_eq!(touch["gyroscope"], true);
        assert_eq!(touch["resolution_scale_percent"], 100);

        let read = Settings::read(&game).unwrap();
        assert_eq!(read.resolution_scale, Some(150));
        assert_eq!(read.max_fps, Some(60));
        assert_eq!(read.orientation, Some(Orientation::Both));
        assert_eq!(read.battery_mode, None);
        std::fs::remove_dir_all(game).unwrap();
    }
}
