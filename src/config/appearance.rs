//! Remember the last light/dark mode and Material palette chosen in the UI.
use super::storage::{atomic_write, config_path};
use crate::theme::ColorVariant;
use std::{fs, io, path::PathBuf};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Appearance {
    pub dark: bool,
    pub variant: ColorVariant,
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
        let contents = match fs::read_to_string(path) {
            Ok(contents) => contents,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(defaults),
            Err(error) => return Err(error),
        };
        let mut appearance = defaults;
        for line in contents.lines() {
            let Some((key, value)) = line.trim().split_once('=') else {
                continue;
            };
            match key.trim() {
                "theme" => match value.trim() {
                    "dark" => appearance.dark = true,
                    "light" => appearance.dark = false,
                    _ => {}
                },
                "variant" => {
                    if let Some(variant) = ColorVariant::from_name(value) {
                        appearance.variant = variant;
                    }
                }
                _ => {}
            }
        }
        Ok(appearance)
    }

    pub fn save(&self, appearance: Appearance) -> io::Result<()> {
        let Some(path) = &self.path else {
            return Ok(());
        };
        atomic_write(
            path,
            &format!(
                "# Materialand appearance\ntheme={}\nvariant={}\n",
                if appearance.dark { "dark" } else { "light" },
                appearance.variant.name(),
            ),
        )
    }
}

pub fn appearance_path() -> Option<PathBuf> {
    config_path("appearance")
}

#[cfg(test)]
mod tests;
