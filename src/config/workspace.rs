//! Sparse desktop positions, committed together so a failed drop changes nothing.
use super::{storage::atomic_write, Pins};
use serde::{Deserialize, Serialize};
use std::{io, path::PathBuf};

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Anchor {
    pub page: usize,
    pub x: f32,
    pub y: f32,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Shortcut {
    pub id: String,
    pub anchor: Option<Anchor>,
}
#[derive(Default)]
pub struct Workspace {
    path: Option<PathBuf>,
    pub entries: Vec<Shortcut>,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Document {
    version: u32,
    shortcuts: Vec<Shortcut>,
}
impl Workspace {
    pub fn load(legacy: PathBuf) -> io::Result<Self> {
        let path = legacy.with_file_name("workspace.json");
        let entries = match std::fs::read_to_string(&path) {
            Ok(text) => {
                let doc: Document = serde_json::from_str(&text).map_err(io::Error::other)?;
                if doc.version != 1
                    || doc.shortcuts.iter().any(|s| {
                        s.anchor.is_some_and(|a| {
                            a.page > 4096
                                || !a.x.is_finite()
                                || !a.y.is_finite()
                                || !(0.0..=1.0).contains(&a.x)
                                || !(0.0..=1.0).contains(&a.y)
                        })
                    })
                {
                    return Err(io::Error::other("invalid home workspace"));
                }
                let mut ids = std::collections::HashSet::new();
                if doc.shortcuts.iter().any(|s| !ids.insert(&s.id)) {
                    return Err(io::Error::other("duplicate home shortcut"));
                }
                doc.shortcuts
            }
            Err(e) if e.kind() == io::ErrorKind::NotFound => Pins::load(legacy, vec![])?
                .ids()
                .iter()
                .map(|id| Shortcut {
                    id: id.clone(),
                    anchor: None,
                })
                .collect(),
            Err(e) => return Err(e),
        };
        Ok(Self {
            path: Some(path),
            entries,
        })
    }
    pub fn set_path(&mut self, path: PathBuf) {
        self.path = Some(path);
    }
    pub fn contains(&self, id: &str) -> bool {
        self.entries.iter().any(|s| s.id == id)
    }
    pub fn commit(&mut self, entries: Vec<Shortcut>) -> io::Result<()> {
        if let Some(path) = &self.path {
            let contents = serde_json::to_string_pretty(&Document {
                version: 1,
                shortcuts: entries.clone(),
            })
            .map_err(io::Error::other)?;
            atomic_write(path, &contents)?;
        }
        self.entries = entries;
        Ok(())
    }
}
