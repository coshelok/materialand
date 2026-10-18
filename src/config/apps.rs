//! Application discovery and desktop-entry parsing.
use super::*;
/// A launchable application parsed from a `.desktop` file.
#[derive(Clone, Debug)]
pub struct AppEntry {
    pub name: String,
    pub exec: String,
    pub icon: String,
    pub terminal: bool,
    /// Originating desktop-entry file, used by the `%k` Exec field code.
    pub desktop_file: Option<PathBuf>,
}

impl AppEntry {
    /// Command line to spawn: desktop-entry split (quotes/escapes aware),
    /// field codes (`%U`, `%f`, `%%`, ...) stripped.
    pub fn command(&self) -> Vec<String> {
        split_exec(
            &self.exec,
            &self.name,
            &self.icon,
            self.desktop_file.as_deref(),
        )
    }
    pub fn command_in_terminal(&self, terminal: &[String]) -> Vec<String> {
        let args = self.command();
        if self.terminal {
            super::terminal::wrap(terminal, args)
        } else {
            args
        }
    }
}

pub(super) fn desktop_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    let data_home = std::env::var_os("XDG_DATA_HOME").filter(|value| !value.is_empty());
    if let Some(data_home) = data_home {
        dirs.push(PathBuf::from(data_home).join("applications"));
    } else if let Ok(home) = std::env::var("HOME") {
        dirs.push(PathBuf::from(home).join(".local/share/applications"));
    }

    let data_dirs = std::env::var("XDG_DATA_DIRS")
        .unwrap_or_else(|_| "/usr/local/share:/usr/share".to_string());
    for dir in data_dirs.split(':').filter(|s| !s.is_empty()) {
        dirs.push(PathBuf::from(dir).join("applications"));
    }
    dirs
}

/// Check whether a `TryExec` binary is visible in PATH or is an executable path.
pub(super) fn executable_available(program: &str) -> bool {
    use std::os::unix::fs::PermissionsExt;

    let is_executable = |path: &Path| {
        std::fs::metadata(path)
            .map(|meta| meta.is_file() && meta.permissions().mode() & 0o111 != 0)
            .unwrap_or(false)
    };
    if program.contains('/') {
        return is_executable(Path::new(program));
    }
    std::env::var_os("PATH")
        .map(|path| std::env::split_paths(&path).any(|dir| is_executable(&dir.join(program))))
        .unwrap_or(false)
}

/// Scan applications using XDG precedence: user entries override system entries
/// with the same desktop-file ID.
pub fn scan_apps() -> Vec<AppEntry> {
    let mut apps = Vec::new();
    let mut seen_ids: HashSet<String> = HashSet::new();

    for dir in desktop_dirs() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("desktop") {
                continue;
            }
            let id = path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .into_owned();
            if !seen_ids.insert(id) {
                continue;
            }
            if let Some(app) = parse_desktop(&path) {
                apps.push(app);
            }
        }
    }

    apps.sort_by_key(|a| a.name.to_lowercase());
    apps
}

pub(super) fn parse_desktop(path: &Path) -> Option<AppEntry> {
    let text = std::fs::read_to_string(path).ok()?;
    let mut in_entry = false;
    let mut name = None;
    let mut exec = None;
    let mut icon = String::new();
    let mut terminal = false;
    let mut hidden = false;
    let mut try_exec: Option<String> = None;

    for line in text.lines() {
        let line = line.trim();
        if line.starts_with('[') {
            in_entry = line == "[Desktop Entry]";
            continue;
        }
        if !in_entry {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let (key, value) = (key.trim(), value.trim());
        match key {
            "Name" if name.is_none() => name = Some(value.to_string()),
            "Exec" => exec = Some(value.to_string()),
            "Icon" => icon = value.to_string(),
            "Terminal" => terminal = value.eq_ignore_ascii_case("true"),
            "TryExec" => try_exec = Some(value.to_string()),
            "NoDisplay" | "Hidden" if value.eq_ignore_ascii_case("true") => {
                hidden = true;
            }
            _ => {}
        }
    }

    if hidden {
        return None;
    }
    if try_exec
        .as_deref()
        .is_some_and(|program| !executable_available(program))
    {
        return None;
    }
    let name = name?;
    let exec = exec?;
    if split_exec(&exec, &name, &icon, Some(path)).is_empty() {
        return None;
    }
    Some(AppEntry {
        name,
        exec,
        icon,
        terminal,
        desktop_file: Some(path.to_path_buf()),
    })
}

/// Map an application id / title / icon name to a Material Symbols icon name.
pub fn icon_for_key(key: &str) -> &'static str {
    let k = key.to_lowercase();
    let has = |s: &str| k.contains(s);
    if has("terminal")
        || has("foot")
        || has("kitty")
        || has("alacritty")
        || has("wezterm")
        || has("konsole")
    {
        "terminal"
    } else if has("firefox") || has("chrome") || has("chromium") || has("browser") || has("webkit")
    {
        "public"
    } else if has("code") || has("editor") || has("nvim") || has("vim") || has("emacs") {
        "code"
    } else if has("music") || has("spotify") || has("audio") || has("player") {
        "music_note"
    } else if has("photo") || has("image") || has("gimp") || has("inkscape") || has("viewer") {
        "photo"
    } else if has("video") || has("vlc") || has("mpv") {
        "movie"
    } else if has("settings") || has("control") || has("config") || has("tweaks") {
        "settings"
    } else if has("files") || has("nautilus") || has("thunar") || has("dolphin") || has("folder") {
        "folder"
    } else if has("mail") || has("thunderbird") || has("evolution") {
        "mail"
    } else if has("calc") {
        "calculate"
    } else if has("game") || has("steam") {
        "sports_esports"
    } else if has("disk") || has("system") || has("monitor") {
        "monitor"
    } else if has("doc") || has("writer") || has("office") || has("word") {
        "description"
    } else if has("calendar") {
        "calendar_month"
    } else if has("chat") || has("discord") || has("telegram") || has("slack") {
        "chat"
    } else {
        "apps"
    }
}
