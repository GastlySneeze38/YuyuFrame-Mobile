//! The engine's `key:value` files (`options.txt`, `horizon-graphics.txt`): read, and changed a key
//! at a time with every other line kept as it was, unknown keys and their order included.
use std::path::Path;

/// A `key:value` file as lines. A line without `:` is kept and never matched.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct KeyValues {
    lines: Vec<String>,
}

impl KeyValues {
    /// The file at `path`; empty when there is none yet.
    pub fn load(path: &Path) -> std::io::Result<Self> {
        match std::fs::read_to_string(path) {
            Ok(text) => Ok(Self::parse(&text)),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Self::default()),
            Err(e) => Err(e),
        }
    }

    pub fn parse(text: &str) -> Self {
        Self {
            lines: text
                .lines()
                .map(|l| l.trim_end_matches('\r').to_owned())
                .collect(),
        }
    }

    /// The value of `key`: the last line naming it, as the engine reads it.
    pub fn get(&self, key: &str) -> Option<&str> {
        self.lines
            .iter()
            .rev()
            .find_map(|line| line.split_once(':').filter(|(k, _)| *k == key))
            .map(|(_, v)| v)
    }

    pub fn int(&self, key: &str) -> Option<i64> {
        self.get(key)?.trim().parse().ok()
    }

    pub fn bool(&self, key: &str) -> Option<bool> {
        match self.get(key)?.trim() {
            "true" => Some(true),
            "false" => Some(false),
            _ => None,
        }
    }

    /// Sets `key` where it is (every line naming it), else at the end. A key or value holding a
    /// line break would make another line, so it is refused.
    pub fn set(&mut self, key: &str, value: &str) -> Result<(), String> {
        if key.is_empty() || key.contains([':', '\n', '\r']) || value.contains(['\n', '\r']) {
            return Err(format!("invalid setting {key:?}"));
        }
        let line = format!("{key}:{value}");
        let mut found = false;
        for existing in &mut self.lines {
            if existing.split_once(':').is_some_and(|(k, _)| k == key) {
                *existing = line.clone();
                found = true;
            }
        }
        if !found {
            self.lines.push(line);
        }
        Ok(())
    }

    pub fn text(&self) -> String {
        let mut text = self.lines.join("\n");
        if !text.is_empty() {
            text.push('\n');
        }
        text
    }

    pub fn save(&self, path: &Path) -> std::io::Result<()> {
        crate::write_atomic(path, self.text().as_bytes())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_change_keeps_every_other_line_and_their_order() {
        let mut file = KeyValues::parse("version:4671\r\nunknownMod:x:y\n# note\nmaxFps:120\n");
        file.set("maxFps", "60").unwrap();
        file.set("newKey", "1").unwrap();
        assert_eq!(
            file.text(),
            "version:4671\nunknownMod:x:y\n# note\nmaxFps:60\nnewKey:1\n"
        );
        assert_eq!(file.get("unknownMod"), Some("x:y"));
        assert!(file.set("maxFps", "1\nop:true").is_err());
    }
}
