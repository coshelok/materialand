//! Font and icon discovery.
use super::*;
pub(super) fn font_search_roots() -> Vec<PathBuf> {
    let mut roots = Vec::new();
    let directory = &crate::config::design().fonts.directory;
    if !directory.is_empty() {
        roots.push(PathBuf::from(directory));
    }
    roots.extend(installed_font_roots(
        std::env::current_exe().ok().as_deref(),
        std::env::var_os("XDG_DATA_HOME").as_deref().map(Path::new),
        std::env::var_os("HOME").as_deref().map(Path::new),
        std::env::var_os("XDG_DATA_DIRS").as_deref(),
    ));
    roots.push(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets/fonts"));
    roots.push(PathBuf::from("assets/fonts"));
    if let Some(home) = std::env::var_os("HOME") {
        let home = PathBuf::from(home);
        roots.push(home.join(".local/share/fonts"));
        roots.push(home.join(".fonts"));
    }
    roots.extend([
        PathBuf::from("/usr/share/fonts/truetype/roboto"),
        PathBuf::from("/usr/share/fonts/truetype/google-roboto"),
        PathBuf::from("/usr/share/fonts/opentype/roboto"),
        PathBuf::from("/usr/share/fonts/truetype/noto"),
        PathBuf::from("/usr/share/fonts/truetype/dejavu"),
        PathBuf::from("/usr/share/fonts/truetype/liberation"),
        PathBuf::from("/usr/share/fonts/fonts-go"),
        PathBuf::from("/usr/share/fonts"),
        PathBuf::from("/usr/local/share/fonts"),
    ]);
    roots.dedup();
    roots
}

fn installed_font_roots(
    executable: Option<&Path>,
    data_home: Option<&Path>,
    home: Option<&Path>,
    data_dirs: Option<&std::ffi::OsStr>,
) -> Vec<PathBuf> {
    let mut roots = Vec::new();
    if let Some(data) = data_home
        .filter(|p| p.is_absolute())
        .map(Path::to_path_buf)
        .or_else(|| home.map(|p| p.join(".local/share")))
    {
        roots.push(data.join("materialand/fonts"));
    }
    // Support staged/custom prefixes and moving a complete installation.
    if let Some(prefix) = executable.and_then(Path::parent).and_then(Path::parent) {
        roots.push(prefix.join("share/materialand/fonts"));
    }
    for dir in std::env::split_paths(
        data_dirs
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| std::ffi::OsStr::new("/usr/local/share:/usr/share")),
    ) {
        if dir.is_absolute() {
            roots.push(dir.join("materialand/fonts"));
        }
    }
    roots
}

pub(super) fn find_file(roots: &[PathBuf], names: &[&str]) -> Option<PathBuf> {
    // Fast path for the customary font directory layout.
    for root in roots {
        for name in names {
            let candidate = root.join(name);
            if candidate.is_file() {
                return Some(candidate);
            }
        }
    }

    // Distros commonly install Roboto in a versioned or nested subdirectory.
    // Search only font directories and cap depth so startup remains predictable.
    pub(super) fn walk(dir: &Path, wanted: &[&str], depth: usize) -> Option<PathBuf> {
        if depth > 5 {
            return None;
        }
        let entries = std::fs::read_dir(dir).ok()?;
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() {
                let file_name = path.file_name()?.to_string_lossy();
                if wanted.iter().any(|n| file_name.eq_ignore_ascii_case(n)) {
                    return Some(path);
                }
            } else if path.is_dir() {
                if let Some(found) = walk(&path, wanted, depth + 1) {
                    return Some(found);
                }
            }
        }
        None
    }
    for root in roots {
        if root.is_dir() {
            if let Some(found) = walk(root, names, 0) {
                return Some(found);
            }
        }
    }
    None
}

pub(super) fn load_font(path: &Path) -> Option<Font> {
    let bytes = std::fs::read(path).ok()?;
    Font::from_bytes(bytes, FontSettings::default()).ok()
}

pub(super) fn fallback_icons() -> HashMap<String, char> {
    // Readable Unicode glyph fallbacks for systems without Google's optional
    // Material Icons font. Installing MaterialIcons-Regular.ttf plus its
    // `codepoints` map switches to the canonical Material glyphs automatically.
    [
        ("apps", '▦'),
        ("terminal", '›'),
        ("public", '◎'),
        ("code", '〈'),
        ("music_note", '♫'),
        ("photo", '▧'),
        ("movie", '▶'),
        ("settings", '⚙'),
        ("folder", '▱'),
        ("mail", '✉'),
        ("calculate", '∑'),
        ("sports_esports", '♧'),
        ("monitor", '▣'),
        ("description", '▤'),
        ("calendar_month", '▦'),
        ("chat", '▰'),
        ("search", '⌕'),
        ("account_circle", '☺'),
        ("power_settings_new", '⏻'),
        ("restart_alt", '↻'),
        ("logout", '→'),
        ("light_mode", '☼'),
        ("dark_mode", '☾'),
        ("brightness_6", '☼'),
        ("nightlight", '☾'),
        ("reduce_motion", '≋'),
        ("remove", '−'),
        ("crop_square", '□'),
        ("filter_none", '▢'),
        ("close", '×'),
    ]
    .into_iter()
    .map(|(name, glyph)| (name.to_string(), glyph))
    .collect()
}

pub(super) fn parse_codepoints(path: &Path) -> HashMap<String, char> {
    let mut map = HashMap::new();
    let Ok(text) = std::fs::read_to_string(path) else {
        return map;
    };
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let mut parts = line.split_whitespace();
        if let (Some(name), Some(cp)) = (parts.next(), parts.next()) {
            let cp = cp.trim_start_matches("U+");
            if let Ok(cp) = u32::from_str_radix(cp, 16) {
                if let Some(ch) = char::from_u32(cp) {
                    map.insert(name.to_string(), ch);
                }
            }
        }
    }
    map
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn installed_resources_follow_xdg_and_relocated_binary_without_checkout() {
        let stage =
            std::env::temp_dir().join(format!("materialand-font-install-{}", std::process::id()));
        let executable = stage.join("opt/materialand/bin/materialand");
        let font_dir = stage.join("opt/materialand/share/materialand/fonts");
        std::fs::create_dir_all(&font_dir).unwrap();
        for file in [
            "Roboto-Regular.ttf",
            "Roboto-Medium.ttf",
            "Roboto-Bold.ttf",
            "MaterialIcons-Regular.ttf",
            "codepoints",
        ] {
            std::fs::copy(
                Path::new(env!("CARGO_MANIFEST_DIR"))
                    .join("assets/fonts")
                    .join(file),
                font_dir.join(file),
            )
            .unwrap();
        }
        // No source tree or system-font paths are supplied to the loader.
        let roots = installed_font_roots(
            Some(&executable),
            None,
            None,
            Some(std::ffi::OsStr::new("relative-ignored")),
        );
        assert_eq!(roots, vec![font_dir]);
        let fonts = Fonts::load_from_roots(&roots).unwrap();
        assert_eq!(fonts.icon_codepoints.get("wifi"), Some(&'\u{e63e}'));
        assert_eq!(fonts.icon_codepoints.get("splitscreen"), Some(&'\u{f06d}'));
        std::fs::remove_dir_all(stage).unwrap();

        let roots = installed_font_roots(
            None,
            Some(Path::new("/custom/data")),
            Some(Path::new("/user")),
            Some(std::ffi::OsStr::new("/opt/data:relative:/usr/share")),
        );
        assert_eq!(
            roots,
            vec![
                PathBuf::from("/custom/data/materialand/fonts"),
                PathBuf::from("/opt/data/materialand/fonts"),
                PathBuf::from("/usr/share/materialand/fonts")
            ]
        );
        let roots = installed_font_roots(
            None,
            Some(Path::new("relative")),
            Some(Path::new("/user")),
            None,
        );
        assert_eq!(
            roots[0],
            PathBuf::from("/user/.local/share/materialand/fonts")
        );
    }
}
