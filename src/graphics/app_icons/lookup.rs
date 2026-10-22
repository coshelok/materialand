//! freedesktop icon-theme lookup: current theme, inherited themes, hicolor, pixmaps.
use std::{
    collections::{HashMap, HashSet},
    path::{Path, PathBuf},
};
#[derive(Default)]
struct Theme {
    directories: Vec<(String, u32, u32)>,
    parents: Vec<String>,
}
pub(super) struct Resolver {
    roots: Vec<PathBuf>,
    fallback: Vec<PathBuf>,
    theme: String,
    themes: HashMap<String, Theme>,
}
impl Resolver {
    pub fn system() -> Self {
        let home = std::env::var_os("HOME")
            .map(PathBuf::from)
            .unwrap_or_default();
        let data = std::env::var_os("XDG_DATA_HOME")
            .map(PathBuf::from)
            .filter(|p| p.is_absolute())
            .unwrap_or_else(|| home.join(".local/share"));
        let mut data_dirs = vec![data];
        data_dirs.extend(
            std::env::split_paths(
                &std::env::var_os("XDG_DATA_DIRS")
                    .unwrap_or_else(|| "/usr/local/share:/usr/share".into()),
            )
            .filter(|p| p.is_absolute()),
        );
        data_dirs.extend([
            home.join(".local/share/flatpak/exports/share"),
            PathBuf::from("/var/lib/flatpak/exports/share"),
        ]);
        let mut roots = vec![home.join(".icons")];
        roots.extend(data_dirs.iter().map(|p| p.join("icons")));
        let fallback = data_dirs.iter().map(|p| p.join("pixmaps")).collect();
        let cfg = std::env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .filter(|p| p.is_absolute())
            .unwrap_or_else(|| home.join(".config"));
        let theme = [
            ("gtk-4.0/settings.ini", "Settings", "gtk-icon-theme-name"),
            ("gtk-3.0/settings.ini", "Settings", "gtk-icon-theme-name"),
            ("kdeglobals", "Icons", "Theme"),
        ]
        .into_iter()
        .find_map(|(file, section, key)| {
            let text = std::fs::read_to_string(cfg.join(file)).ok()?;
            ini(&text).get(section)?.get(key).cloned()
        })
        .unwrap_or_else(|| "Adwaita".into());
        Self {
            roots,
            fallback,
            theme,
            themes: HashMap::new(),
        }
    }
    pub fn find(&mut self, name: &str, size: u32) -> Option<PathBuf> {
        let path = Path::new(name);
        if path.is_absolute() {
            return path.is_file().then(|| path.to_owned());
        }
        if name.is_empty() || name.contains('/') || name.contains('\\') {
            return None;
        }
        let mut visited = HashSet::new();
        let theme = self.theme.clone();
        self.in_theme(name, size, &theme, &mut visited)
            .or_else(|| self.in_theme(name, size, "hicolor", &mut visited))
            .or_else(|| {
                self.roots
                    .iter()
                    .chain(&self.fallback)
                    .find_map(|dir| candidate(dir, name))
            })
    }
    fn in_theme(
        &mut self,
        name: &str,
        size: u32,
        theme: &str,
        visited: &mut HashSet<String>,
    ) -> Option<PathBuf> {
        if theme.contains('/')
            || theme == ".."
            || visited.len() >= 32
            || !visited.insert(theme.to_owned())
        {
            return None;
        }
        if !self.themes.contains_key(theme) {
            let parsed = self
                .roots
                .iter()
                .find_map(|r| std::fs::read_to_string(r.join(theme).join("index.theme")).ok())
                .map(|text| {
                    let data = ini(&text);
                    let mut out = Theme::default();
                    if let Some(index) = data.get("Icon Theme") {
                        out.parents = index
                            .get("Inherits")
                            .map(|s| {
                                s.split(',')
                                    .map(|v| v.trim().to_owned())
                                    .filter(|v| !v.is_empty())
                                    .collect()
                            })
                            .unwrap_or_default();
                        for dir in index
                            .get("Directories")
                            .into_iter()
                            .chain(index.get("ScaledDirectories"))
                            .flat_map(|s| s.split(','))
                        {
                            let dir = dir.trim();
                            if dir.is_empty()
                                || Path::new(dir)
                                    .components()
                                    .any(|c| !matches!(c, std::path::Component::Normal(_)))
                            {
                                continue;
                            }
                            let Some(attrs) = data.get(dir) else {
                                continue;
                            };
                            let value = |key: &str, default: u32| {
                                attrs
                                    .get(key)
                                    .and_then(|v| v.parse::<u32>().ok())
                                    .unwrap_or(default)
                            };
                            let nominal = value("Size", 48);
                            let scale = value("Scale", 1).clamp(1, 16);
                            let (lo, hi) = match attrs.get("Type").map(String::as_str) {
                                Some("Scalable") => {
                                    (value("MinSize", nominal), value("MaxSize", nominal))
                                }
                                Some("Fixed") => (nominal, nominal),
                                _ => (
                                    nominal.saturating_sub(value("Threshold", 2)),
                                    nominal.saturating_add(value("Threshold", 2)),
                                ),
                            };
                            out.directories.push((
                                dir.into(),
                                lo.saturating_mul(scale),
                                hi.saturating_mul(scale),
                            ));
                        }
                    }
                    out
                })
                .unwrap_or_default();
            self.themes.insert(theme.into(), parsed);
        }
        let t = &self.themes[theme];
        let mut dirs = t.directories.clone();
        dirs.sort_by_key(|(_, lo, hi)| lo.saturating_sub(size).max(size.saturating_sub(*hi)));
        for (sub, _, _) in dirs {
            for root in &self.roots {
                if let Some(path) = candidate(&root.join(theme).join(&sub), name) {
                    return Some(path);
                }
            }
        }
        let parents = t.parents.clone();
        parents
            .into_iter()
            .find_map(|p| self.in_theme(name, size, &p, visited))
    }
    #[cfg(test)]
    pub fn test(root: PathBuf, theme: &str) -> Self {
        Self {
            roots: vec![root.clone()],
            fallback: vec![root.join("pixmaps")],
            theme: theme.into(),
            themes: HashMap::new(),
        }
    }
}
fn candidate(dir: &Path, name: &str) -> Option<PathBuf> {
    if ["png", "svg", "xpm"]
        .iter()
        .any(|ext| name.ends_with(&format!(".{ext}")))
    {
        let p = dir.join(name);
        if p.is_file() {
            return Some(p);
        }
    }
    ["png", "svg", "xpm"]
        .iter()
        .map(|ext| dir.join(format!("{name}.{ext}")))
        .find(|p| p.is_file())
}
fn ini(text: &str) -> HashMap<String, HashMap<String, String>> {
    let mut groups: HashMap<String, HashMap<String, String>> = HashMap::new();
    let mut section = String::new();
    for line in text
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
    {
        if line.starts_with('[') && line.ends_with(']') {
            section = line[1..line.len() - 1].into();
        } else if let Some((k, v)) = line.split_once('=') {
            groups
                .entry(section.clone())
                .or_default()
                .insert(k.trim().into(), v.trim().trim_matches('"').into());
        }
    }
    groups
}
