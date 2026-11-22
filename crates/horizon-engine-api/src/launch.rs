//! How a launcher starts the engine: the options of Horizon's launch contract (`docs/api/launch.md`),
//! the session file that carries the account, and on Android `horizon-args.txt`, which the
//! activity reads as its command line (`android.rs`, `command_line`): one option a line, or
//! `NAME=value` for an environment variable.
use std::path::{Path, PathBuf};

/// Who plays.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Player {
    /// A signed-in account, passed in a session file ([`write_session_file`]): `--session-file`.
    Session(PathBuf),
    /// An offline name: `--name`.
    Offline(String),
}

/// One run of the engine.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Launch {
    /// The game folder: saves, options, mods, packs (`--game-dir`).
    pub game_dir: PathBuf,
    /// The engine's own files for this folder: logs, crash reports (`--home`).
    pub home: PathBuf,
    pub player: Player,
    /// A server to join once started (`--quickPlayMultiplayer`).
    pub server: Option<String>,
    /// Environment variables for the engine (on Android, written in `horizon-args.txt`).
    pub env: Vec<(String, String)>,
}

impl Launch {
    /// The command line, one option an element: only options the launch contract keeps.
    pub fn arguments(&self) -> Vec<String> {
        let mut args = vec![
            "--game-dir".to_owned(),
            self.game_dir.display().to_string(),
            "--home".to_owned(),
            self.home.display().to_string(),
        ];
        match &self.player {
            Player::Session(path) => args.extend(["--session-file".to_owned(), path.display().to_string()]),
            Player::Offline(name) => args.extend(["--name".to_owned(), name.clone()]),
        }
        if let Some(server) = self.server.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
            args.extend(["--quickPlayMultiplayer".to_owned(), server.to_owned()]);
        }
        args
    }

    /// `horizon-args.txt` as the Android activity reads it. A value with a line break would make
    /// another option, and a variable name other than `A-Z`, `0-9` and `_` would be read as an
    /// option: both are refused.
    pub fn args_file(&self) -> Result<String, String> {
        let mut text = String::from("# The engine's command line, written at each launch.\n");
        for (name, value) in &self.env {
            let named = !name.is_empty()
                && name.bytes().all(|b| b.is_ascii_uppercase() || b.is_ascii_digit() || b == b'_');
            if !named || value.contains(['\n', '\r']) {
                return Err(format!("invalid environment variable {name:?}"));
            }
            text.push_str(&format!("{name}={value}\n"));
        }
        for arg in self.arguments() {
            if arg.contains(['\n', '\r']) {
                return Err("invalid launch option (line break)".into());
            }
            text.push_str(&arg);
            text.push('\n');
        }
        Ok(text)
    }

    /// Writes `horizon-args.txt` in `files` (the app's private files) in one step.
    pub fn write_args_file(&self, files: &Path) -> Result<PathBuf, String> {
        let path = files.join(crate::contract::ARGS);
        crate::write_atomic(&path, self.args_file()?.as_bytes()).map_err(|e| e.to_string())?;
        Ok(path)
    }
}

/// The account a session file carries (the launch contract's fields).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Session {
    pub name: String,
    /// The profile's UUID, with or without dashes.
    pub uuid: String,
    pub access_token: String,
    /// `msa` for a Microsoft account.
    pub user_type: String,
    pub xuid: Option<String>,
}

/// Writes the session file for [`Player::Session`] in `dir`: a new file under a name nobody can
/// guess, readable by its owner only (the engine refuses another), which the engine reads once and
/// deletes; it also refuses one older than two minutes. A launcher deletes it itself when the
/// engine never started.
pub fn write_session_file(dir: &Path, session: &Session) -> Result<PathBuf, String> {
    use std::io::Write;
    std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    let path = dir.join(format!("session-{}.json", unguessable()));
    let body = serde_json::json!({
        "name": session.name,
        "uuid": session.uuid.replace('-', ""),
        "access_token": session.access_token,
        "user_type": session.user_type,
        "xuid": session.xuid,
    })
    .to_string();
    let mut options = std::fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options.open(&path).map_err(|e| e.to_string())?;
    file.write_all(body.as_bytes())
        .and_then(|()| file.sync_all())
        .map_err(|e| e.to_string())?;
    Ok(path)
}

/// 128 bits from the standard library's random hash keys, the time and the process.
fn unguessable() -> String {
    use std::hash::{BuildHasher, Hasher};
    let mut text = String::new();
    for round in 0..2u64 {
        let mut hasher = std::collections::hash_map::RandomState::new().build_hasher();
        hasher.write_u64(round);
        hasher.write_u128(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or_default(),
        );
        hasher.write_u32(std::process::id());
        text.push_str(&format!("{:016x}", hasher.finish()));
    }
    text
}

#[cfg(test)]
mod tests {
    use super::*;

    fn launch(player: Player, server: Option<&str>) -> Launch {
        Launch {
            game_dir: "/data/instances/a".into(),
            home: "/data/instances/a/horizon".into(),
            player,
            server: server.map(str::to_owned),
            env: vec![("YUYUFRAME".into(), "1".into())],
        }
    }

    #[test]
    fn offline_takes_the_name_and_an_account_the_session_file() {
        let offline = launch(Player::Offline("Alex".into()), None).arguments();
        assert!(offline.windows(2).any(|w| w == ["--name", "Alex"]));
        let online = launch(Player::Session("/f/s.json".into()), Some(" play.example.net ")).arguments();
        assert!(!online.iter().any(|a| a == "--name"));
        assert!(online.windows(2).any(|w| w == ["--session-file", "/f/s.json"]));
        assert!(online.windows(2).any(|w| w == ["--quickPlayMultiplayer", "play.example.net"]));
    }

    #[test]
    fn the_args_file_refuses_what_would_make_another_line() {
        let text = launch(Player::Offline("Alex".into()), None).args_file().unwrap();
        assert!(text.lines().any(|l| l == "YUYUFRAME=1"));
        assert!(text.lines().any(|l| l == "--name"));
        assert!(launch(Player::Offline("Alex\n--op".into()), None).args_file().is_err());
        let mut bad = launch(Player::Offline("Alex".into()), None);
        bad.env.push(("--game-dir".into(), "x".into()));
        assert!(bad.args_file().is_err());
    }

    #[test]
    fn a_session_file_is_new_each_time_and_holds_the_contract_fields() {
        let dir = crate::scratch("session");
        let session = Session {
            name: "Alex".into(),
            uuid: "069a79f4-44e9-4726-a5be-fca90e38aaf5".into(),
            access_token: "t".into(),
            user_type: "msa".into(),
            xuid: None,
        };
        let a = write_session_file(&dir, &session).unwrap();
        let b = write_session_file(&dir, &session).unwrap();
        assert_ne!(a, b);
        let json: serde_json::Value = serde_json::from_slice(&std::fs::read(&a).unwrap()).unwrap();
        assert_eq!(json["uuid"], "069a79f444e94726a5befca90e38aaf5");
        std::fs::remove_dir_all(dir).unwrap();
    }
}
