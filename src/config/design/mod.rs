//! One immutable design snapshot per session; UI preferences share its TOML file.
mod compact;
mod document;
mod schema;
mod validation;
use super::{appearance_path, storage::atomic_write};
pub use compact::compact_file;
pub(super) use document::{load_document, parse_document, write_preferences};
pub use schema::Design;
use std::{fs, io, path::Path, sync::OnceLock};

pub const DEFAULT_CONFIG: &str = include_str!("../../../config/default.toml");
static DESIGN: OnceLock<Design> = OnceLock::new();
static BUILT_IN: OnceLock<Design> = OnceLock::new();

pub fn built_in_design() -> &'static Design {
    BUILT_IN.get_or_init(|| toml_edit::de::from_str(DEFAULT_CONFIG).expect("valid built-in design"))
}

pub fn design() -> &'static Design {
    // Tests and standalone algorithms use built-in values without touching HOME.
    DESIGN.get_or_init(Design::default)
}

impl Default for Design {
    fn default() -> Self {
        built_in_design().clone()
    }
}

/// Called before fonts, windows, devices or animations are created.
pub fn initialize_design() {
    let Some(path) = appearance_path() else {
        return;
    };
    let loaded = load_or_create(&path);
    let design = match loaded {
        Ok(design) => design,
        Err(error) => {
            tracing::warn!(path=%path.display(), %error, "could not load design; using built-in defaults");
            Design::default()
        }
    };
    if DESIGN.set(design).is_err() {
        tracing::warn!("design was already initialized");
    }
}

fn load_or_create(path: &Path) -> io::Result<Design> {
    let doc = load_document(path, document::startup_defaults())?;
    let design = parse_document(&doc)?;
    if !path.exists() {
        atomic_write(path, &doc.to_string())?;
    }
    Ok(design)
}

/// Create a complete commented file without acquiring a display or a seat.
pub fn initialize_file() -> io::Result<()> {
    let path = appearance_path().ok_or_else(|| io::Error::other("no configuration directory"))?;
    load_or_create(&path)?;
    println!("Design configuration: {}", path.display());
    Ok(())
}

pub fn check_file() -> io::Result<()> {
    let Some(path) = appearance_path() else {
        return Err(io::Error::other("no configuration directory"));
    };
    let doc = load_document(&path, document::startup_defaults())?;
    parse_document(&doc)?;
    println!("Valid design configuration: {}", path.display());
    Ok(())
}

fn invalid(error: impl std::fmt::Display) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, error.to_string())
}

#[cfg(test)]
mod tests;
