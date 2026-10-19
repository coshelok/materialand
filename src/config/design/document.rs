//! Merge partial TOML, migrate legacy appearance and preserve user formatting.
use super::*;
use crate::config::Appearance;
use crate::theme::{parse_seed, ColorVariant};
use std::path::Path;
use toml_edit::{value, DocumentMut, Item, Table, TableLike};

fn replace_value(target: &mut Item, mut replacement: Item) {
    if let (Some(old), Some(new)) = (target.as_value(), replacement.as_value_mut()) {
        *new.decor_mut() = old.decor().clone();
    }
    *target = replacement;
}

pub(super) fn startup_defaults() -> DocumentMut {
    let mut doc = DEFAULT_CONFIG
        .parse::<DocumentMut>()
        .expect("built-in TOML");
    if let Ok(directory) = std::env::var("MATERIALAND_FONT_DIR") {
        replace_value(&mut doc["fonts"]["directory"], value(directory));
    }
    if let Ok(mode) = std::env::var("MATERIALAND_THEME") {
        if matches!(mode.as_str(), "light" | "dark") {
            replace_value(&mut doc["appearance"]["theme"], value(mode));
        }
    }
    if let Ok(variant) = std::env::var("MATERIALAND_VARIANT") {
        if let Some(variant) = ColorVariant::from_name(&variant) {
            replace_value(&mut doc["appearance"]["variant"], value(variant.name()));
        }
    }
    if let Ok(seed) = std::env::var("MATERIALAND_SEED") {
        if parse_seed(&seed).is_some() {
            replace_value(&mut doc["appearance"]["seed"], value(seed));
            return doc;
        }
    }
    if let Ok(wallpaper) = std::env::var("MATERIALAND_WALLPAPER") {
        replace_value(&mut doc["appearance"]["seed"], value(""));
        replace_value(&mut doc["appearance"]["wallpaper"], value(wallpaper));
    }
    doc
}

pub(in crate::config) fn load_document(
    path: &Path,
    mut defaults: DocumentMut,
) -> io::Result<DocumentMut> {
    match fs::read_to_string(path) {
        Ok(text) => {
            // Validate before merging, so errors identify the user's own input.
            let doc = text.parse::<DocumentMut>().map_err(invalid)?;
            merge(defaults.as_table_mut(), doc.as_table());
        }
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            // Migration never deletes or rewrites the old preference file.
            match fs::read_to_string(path.with_file_name("appearance")) {
                Ok(text) => {
                    for line in text.lines() {
                        let Some((key, val)) = line.split_once('=') else {
                            continue;
                        };
                        match key.trim() {
                            "theme" if matches!(val.trim(), "dark" | "light") => replace_value(
                                &mut defaults["appearance"]["theme"],
                                value(val.trim()),
                            ),
                            "variant" => {
                                if let Some(variant) = ColorVariant::from_name(val) {
                                    replace_value(
                                        &mut defaults["appearance"]["variant"],
                                        value(variant.name()),
                                    );
                                }
                            }
                            _ => {}
                        }
                    }
                }
                Err(error) if error.kind() == io::ErrorKind::NotFound => {}
                Err(error) => return Err(error),
            }
        }
        Err(error) => return Err(error),
    }
    Ok(defaults)
}

fn merge(base: &mut dyn TableLike, overrides: &dyn TableLike) {
    for (key, replacement) in overrides.iter() {
        match (
            base.get_mut(key).and_then(Item::as_table_like_mut),
            replacement.as_table_like(),
        ) {
            (Some(target), Some(table)) => merge(target, table),
            _ => {
                base.insert(key, replacement.clone());
            }
        }
    }
}

pub(in crate::config) fn parse_document(doc: &DocumentMut) -> io::Result<Design> {
    let design: Design = toml_edit::de::from_str(&doc.to_string()).map_err(invalid)?;
    validation::validate(&design, doc)?;
    Ok(design)
}

pub(in crate::config) fn write_preferences(path: &Path, appearance: Appearance) -> io::Result<()> {
    let mut doc = match fs::read_to_string(path) {
        Ok(text) => text.parse::<DocumentMut>().map_err(invalid)?,
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            load_document(path, startup_defaults())?
        }
        Err(error) => return Err(error),
    };
    // Validate a merged copy, retaining all unrelated comments and partial tables.
    let mut merged = DEFAULT_CONFIG
        .parse::<DocumentMut>()
        .expect("built-in TOML");
    merge(merged.as_table_mut(), doc.as_table());
    parse_document(&merged)?;
    if !doc.as_table().contains_key("appearance") {
        doc.as_table_mut()
            .insert("appearance", Item::Table(Table::new()));
    }
    for (key, val) in [
        (
            "theme",
            value(if appearance.dark { "dark" } else { "light" }),
        ),
        ("variant", value(appearance.variant.name())),
        ("reduce_motion", value(appearance.reduce_motion)),
    ] {
        replace_value(&mut doc["appearance"][key], val);
    }
    atomic_write(path, &doc.to_string())
}
