//! Optional explicit compaction, retaining every non-default override.
use super::*;
use std::{
    fs::OpenOptions,
    io::Write,
    os::unix::fs::OpenOptionsExt,
    time::{SystemTime, UNIX_EPOCH},
};
use toml_edit::{DocumentMut, TableLike, Value};

pub(super) fn compact_document(document: &DocumentMut) -> DocumentMut {
    let defaults = document::startup_defaults();
    let mut compact = document.clone();
    prune(compact.as_table_mut(), defaults.as_table(), "");
    if let Some(mut key) = compact.as_table_mut().key_mut("schema_version") {
        key.leaf_decor_mut().set_prefix("# Materialand settings. Restart the session after editing.\n# Omitted settings use defaults; --print-default-config lists advanced options.\n");
    }
    compact
}

fn keep(path: &str) -> bool {
    matches!(
        path,
        "schema_version"
            | "appearance.theme"
            | "appearance.variant"
            | "appearance.seed"
            | "appearance.reduce_motion"
            | "input.touchpad.invert_scroll"
    )
}

fn prune(table: &mut dyn TableLike, defaults: &dyn TableLike, prefix: &str) {
    let keys: Vec<_> = table.iter().map(|(key, _)| key.to_owned()).collect();
    for key in keys {
        let path = if prefix.is_empty() {
            key.clone()
        } else {
            format!("{prefix}.{key}")
        };
        if keep(&path) {
            continue;
        }
        let Some(default) = defaults.get(&key) else {
            continue;
        };
        let item = table.get_mut(&key).expect("existing key");
        let remove = match (item.as_table_like_mut(), default.as_table_like()) {
            (Some(child), Some(base)) => {
                prune(child, base, &path);
                child.is_empty()
            }
            _ => match (item.as_value(), default.as_value()) {
                (Some(value), Some(base)) => equal_value(value, base),
                _ => false,
            },
        };
        if remove {
            table.remove(&key);
        }
    }
}

fn equal_value(a: &Value, b: &Value) -> bool {
    let number = |value: &Value| {
        value
            .as_float()
            .or_else(|| value.as_integer().map(|n| n as f64))
    };
    if let (Some(a), Some(b)) = (number(a), number(b)) {
        return a == b;
    }
    match (a, b) {
        (Value::String(a), Value::String(b)) => a.value() == b.value(),
        (Value::Boolean(a), Value::Boolean(b)) => a.value() == b.value(),
        (Value::Array(a), Value::Array(b)) => {
            a.len() == b.len() && a.iter().zip(b).all(|(a, b)| equal_value(a, b))
        }
        _ => false,
    }
}

/// Explicit migration of an existing full file; keep an exact backup first.
pub fn compact_file() -> io::Result<()> {
    let path = appearance_path().ok_or_else(|| io::Error::other("no configuration directory"))?;
    let original = fs::read_to_string(&path)?;
    let resolved = load_document(&path, document::startup_defaults())?;
    parse_document(&resolved)?;
    let doc = original.parse::<DocumentMut>().map_err(invalid)?;
    let compact = compact_document(&doc).to_string();
    if compact == original {
        println!("Configuration is already compact: {}", path.display());
        return Ok(());
    }
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let backup = path.with_file_name(format!("config.toml.full-{nonce}.bak"));
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(&backup)?;
    file.write_all(original.as_bytes())?;
    file.sync_all()?;
    if fs::read_to_string(&path)? != original {
        return Err(io::Error::other(
            "configuration changed during compaction; retry",
        ));
    }
    atomic_write(&path, &compact)?;
    println!(
        "Compact configuration: {}\nPrevious configuration: {}",
        path.display(),
        backup.display()
    );
    Ok(())
}
