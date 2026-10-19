//! Persistent launcher identities. Writes are atomic and never change commands.
use std::{
    fs::{self, OpenOptions},
    io::{self, Write},
    os::unix::fs::OpenOptionsExt,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

#[derive(Default)]
pub struct Pins {
    path: Option<PathBuf>,
    ids: Vec<String>,
}
impl Pins {
    pub fn initial(ids: Vec<String>) -> Self {
        Self { path: None, ids }
    }
    pub fn load(path: PathBuf, defaults: Vec<String>) -> io::Result<Self> {
        let ids = match fs::read_to_string(&path) {
            Ok(text) => {
                let mut ids = Vec::new();
                for line in text
                    .lines()
                    .filter(|line| !line.is_empty() && !line.starts_with('#'))
                {
                    let id = decode(line)?;
                    if !ids.contains(&id) {
                        ids.push(id);
                    }
                }
                ids
            }
            Err(error) if error.kind() == io::ErrorKind::NotFound => defaults,
            Err(error) => return Err(error),
        };
        Ok(Self {
            path: Some(path),
            ids,
        })
    }
    pub fn set_path(&mut self, path: PathBuf) {
        self.path = Some(path);
    }
    pub fn ids(&self) -> &[String] {
        &self.ids
    }
    pub fn contains(&self, id: &str) -> bool {
        self.ids.iter().any(|key| key == id)
    }
    pub fn toggle(&mut self, id: String) -> io::Result<()> {
        let mut next = self.ids.clone();
        if next.contains(&id) {
            next.retain(|key| key != &id);
        } else {
            next.push(id);
        }
        if let Some(path) = &self.path {
            save(path, &next)?;
        }
        self.ids = next;
        Ok(())
    }
}

pub fn pins_path() -> Option<PathBuf> {
    std::env::var_os("XDG_CONFIG_HOME")
        .filter(|p| Path::new(p).is_absolute())
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|p| PathBuf::from(p).join(".config")))
        .map(|p| p.join("materialand/pinned-apps"))
}
fn save(path: &Path, ids: &[String]) -> io::Result<()> {
    let parent = path
        .parent()
        .ok_or_else(|| io::Error::other("missing config directory"))?;
    fs::create_dir_all(parent)?;
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let temporary = parent.join(format!(".pinned-apps-{}-{nonce}", std::process::id()));
    let result = (|| {
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(&temporary)?;
        writeln!(file, "# Materialand pinned application identities")?;
        for id in ids {
            writeln!(file, "{}", encode(id))?;
        }
        file.sync_all()?;
        fs::rename(&temporary, path)
    })();
    if result.is_err() {
        let _ = fs::remove_file(temporary);
    }
    result
}
fn encode(id: &str) -> String {
    id.replace('\\', "\\\\")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
}
fn decode(line: &str) -> io::Result<String> {
    let mut out = String::new();
    let mut chars = line.chars();
    while let Some(c) = chars.next() {
        if c != '\\' {
            out.push(c);
            continue;
        }
        out.push(match chars.next() {
            Some('n') => '\n',
            Some('r') => '\r',
            Some('\\') => '\\',
            _ => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "invalid pinned app identity",
                ))
            }
        });
    }
    Ok(out)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn pins_survive_restart_including_an_intentionally_empty_dock() {
        let dir = std::env::temp_dir().join(format!("materialand-pins-{}", std::process::id()));
        let path = dir.join("pinned-apps");
        let _ = fs::remove_dir_all(&dir);
        let mut pins = Pins::load(path.clone(), vec!["desktop:terminal.desktop".into()]).unwrap();
        pins.toggle("desktop:browser.desktop".into()).unwrap();
        let mut pins = Pins::load(path.clone(), vec![]).unwrap();
        assert!(
            pins.contains("desktop:terminal.desktop") && pins.contains("desktop:browser.desktop")
        );
        pins.toggle("desktop:terminal.desktop".into()).unwrap();
        pins.toggle("desktop:browser.desktop".into()).unwrap();
        assert!(Pins::load(path, vec!["default".into()])
            .unwrap()
            .ids()
            .is_empty());
        fs::remove_dir_all(dir).unwrap();
    }
    #[test]
    fn failed_save_keeps_the_previous_pin_state() {
        let mut pins = Pins {
            path: Some(PathBuf::from("/dev/null/pinned-apps")),
            ids: vec!["first".into()],
        };
        assert!(pins.toggle("second".into()).is_err());
        assert_eq!(pins.ids(), ["first"]);
        assert_eq!(
            decode(&encode("exec:hello\\world\nargument\r")).unwrap(),
            "exec:hello\\world\nargument\r"
        );
    }
}
