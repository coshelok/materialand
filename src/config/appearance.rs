//! UI preferences live in the same TOML document as layout and animation tokens.
use super::{
    design::{load_document, parse_document, write_preferences},
    storage::config_path,
    DEFAULT_CONFIG,
};
use crate::theme::ColorVariant;
use std::{io, path::PathBuf};
use toml_edit::{value, DocumentMut};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Appearance {
    pub dark: bool,
    pub variant: ColorVariant,
    pub reduce_motion: bool,
}
#[derive(Default)]
pub struct AppearanceStore {
    path: Option<PathBuf>,
}
impl AppearanceStore {
    pub fn at(path: Option<PathBuf>) -> Self {
        Self { path }
    }
    pub fn load(&self, defaults: Appearance) -> io::Result<Appearance> {
        let Some(path) = &self.path else {
            return Ok(defaults);
        };
        let mut doc = DEFAULT_CONFIG
            .parse::<DocumentMut>()
            .expect("built-in TOML");
        doc["appearance"]["theme"] = value(if defaults.dark { "dark" } else { "light" });
        doc["appearance"]["variant"] = value(defaults.variant.name());
        doc["appearance"]["reduce_motion"] = value(defaults.reduce_motion);
        let design = parse_document(&load_document(path, doc)?)?;
        Ok(Appearance {
            dark: design.appearance.theme == "dark",
            variant: ColorVariant::from_name(&design.appearance.variant)
                .expect("validated variant"),
            reduce_motion: design.appearance.reduce_motion,
        })
    }
    pub fn save(&self, appearance: Appearance) -> io::Result<()> {
        if let Some(path) = &self.path {
            write_preferences(path, appearance)?;
        }
        Ok(())
    }
}
pub fn appearance_path() -> Option<PathBuf> {
    config_path("config.toml")
}
#[cfg(test)]
mod tests;
