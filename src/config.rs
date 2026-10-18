use std::collections::HashSet;
use std::path::{Path, PathBuf};

/// Wayland socket name advertised by the compositor.
pub const SOCKET_NAME: &str = "materialand-0";

/// Nominal physical size of the output in millimeters.
pub const OUTPUT_PHYSICAL_MM: (i32, i32) = (370, 240);

/// Default logical size of a new toplevel.
pub const DEFAULT_WINDOW_SIZE: (i32, i32) = (900, 600);

/// Cascade offset when placing successive windows.
pub const CASCADE_STEP: i32 = 36;

/// Keyboard key repeat as announced to clients through
/// `wl_keyboard.repeat_info`: how long a key must be held before the first
/// repeat, in milliseconds.
pub const KEY_REPEAT_DELAY_MS: i32 = 500;

/// Repeats per second after `KEY_REPEAT_DELAY_MS`. Clients implement the
/// repeat themselves; the compositor only advertises the setting.
pub const KEY_REPEAT_RATE_HZ: i32 = 25;

// Layout metrics, in logical (dp) units.

pub const TOPBAR_HEIGHT: f32 = 32.0;
pub const TOPBAR_MARGIN: f32 = 0.0;

pub const DOCK_HEIGHT: f32 = 80.0;
pub const DOCK_MARGIN: f32 = 0.0;
pub const DOCK_ICON: f32 = 56.0;
pub const DOCK_GAP: f32 = 12.0;

pub const FRAME_TITLEBAR: f32 = 32.0;
pub const FRAME_BORDER: f32 = 1.0;
pub const FRAME_RADIUS: f32 = crate::style::shape::EXTRA_LARGE;

pub const LAUNCHER_WIDTH: f32 = 800.0;
pub const LAUNCHER_MAX_HEIGHT: f32 = 640.0;
pub const LAUNCHER_RADIUS: f32 = crate::style::shape::EXTRA_EXTRA_LARGE;
pub const LAUNCHER_TILE: f32 = 112.0;

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
        let args = split_exec(
            &self.exec,
            &self.name,
            &self.icon,
            self.desktop_file.as_deref(),
        );
        if self.terminal {
            // kitty takes the program to run as its first positional argument.
            let mut cmd = vec!["kitty".to_string()];
            cmd.extend(args);
            cmd
        } else {
            args
        }
    }
}

/// Parse a Desktop Entry `Exec` value without invoking a shell. This handles
/// quoting, the Exec-specific escapes, and the field codes used by launchers.
fn split_exec(
    exec: &str,
    app_name: &str,
    icon: &str,
    desktop_file: Option<&Path>,
) -> Vec<String> {
    let mut raw: Vec<String> = Vec::new();
    let mut cur = String::new();
    let mut has_token = false;
    let mut in_quotes = false;
    let mut chars = exec.chars().peekable();

    while let Some(c) = chars.next() {
        match c {
            '"' => {
                in_quotes = !in_quotes;
                has_token = true;
            }
            '\\' => match chars.peek() {
                Some(&n) if matches!(n, '"' | '\\' | '$' | '`') => {
                    cur.push(n);
                    chars.next();
                    has_token = true;
                }
                _ => {
                    // Keep a literal backslash when it is not an Exec escape.
                    cur.push('\\');
                    has_token = true;
                }
            },
            c if c.is_whitespace() && !in_quotes => {
                if has_token {
                    raw.push(std::mem::take(&mut cur));
                    has_token = false;
                }
            }
            c => {
                cur.push(c);
                has_token = true;
            }
        }
    }
    if in_quotes {
        tracing::warn!(exec, "ignoring desktop entry with unclosed Exec quote");
        return Vec::new();
    }
    if has_token {
        raw.push(cur);
    }

    let mut args = Vec::new();
    for arg in raw {
        // File/URL field codes have no value when launched from the app menu.
        if matches!(arg.as_str(), "%f" | "%F" | "%u" | "%U") {
            continue;
        }
        if arg == "%i" {
            if !icon.is_empty() {
                args.push("--icon".to_string());
                args.push(icon.to_string());
            }
            continue;
        }

        let mut expanded = String::with_capacity(arg.len());
        let mut it = arg.chars();
        while let Some(c) = it.next() {
            if c != '%' {
                expanded.push(c);
                continue;
            }
            match it.next() {
                Some('%') => expanded.push('%'),
                Some('c') => expanded.push_str(app_name),
                Some('k') => {
                    if let Some(path) = desktop_file {
                        expanded.push_str(&path.to_string_lossy());
                    }
                }
                // These codes are valid only as standalone arguments. If a
                // malformed entry embeds one, omit the incomplete argument.
                Some('f' | 'F' | 'u' | 'U' | 'i') => {
                    expanded.clear();
                    break;
                }
                Some(other) => {
                    tracing::debug!(code = %other, "unknown desktop-entry field code");
                }
                None => expanded.push('%'),
            }
        }
        if !expanded.is_empty() {
            args.push(expanded);
        }
    }
    args
}

fn desktop_dirs() -> Vec<PathBuf> {
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
fn executable_available(program: &str) -> bool {
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
        .map(|path| {
            std::env::split_paths(&path).any(|dir| is_executable(&dir.join(program)))
        })
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
            let id = path.file_name().unwrap_or_default().to_string_lossy().into_owned();
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

fn parse_desktop(path: &Path) -> Option<AppEntry> {
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
    if try_exec.as_deref().is_some_and(|program| !executable_available(program)) {
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
    if has("terminal") || has("foot") || has("kitty") || has("alacritty") || has("wezterm") || has("konsole") {
        "terminal"
    } else if has("firefox") || has("chrome") || has("chromium") || has("browser") || has("webkit") {
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

/// Which key a chord is compared against.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BindingKey {
    /// A printable key, matched on its unmodified character (e.g. `p`).
    Char(char),
    /// The main Return/Enter key.
    Return,
}

/// The action a shell chord performs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BindingAction {
    Launcher,
    Terminal,
    ToggleTheme,
    CloseWindow,
}

/// A keyboard chord the shell owns; every other key goes to the focused
/// client. `logo` is the Super/Windows key.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct KeyBinding {
    pub logo: bool,
    pub key: BindingKey,
    pub action: BindingAction,
}

/// The compositor's chords, checked in order. Keeping them in one table makes
/// the set testable and the README list easy to audit.
pub const KEY_BINDINGS: &[KeyBinding] = &[
    KeyBinding { logo: true, key: BindingKey::Char('p'), action: BindingAction::Launcher },
    KeyBinding { logo: true, key: BindingKey::Return, action: BindingAction::Terminal },
    KeyBinding { logo: true, key: BindingKey::Char('d'), action: BindingAction::ToggleTheme },
    KeyBinding { logo: true, key: BindingKey::Char('q'), action: BindingAction::CloseWindow },
];

/// The action `key` triggers with the given Super state, if the shell owns it.
pub fn binding_for(logo: bool, key: BindingKey) -> Option<BindingAction> {
    KEY_BINDINGS
        .iter()
        .find(|binding| binding.logo == logo && binding.key == key)
        .map(|binding| binding.action)
}

/// Keyboard layouts and options for the seat, resolved once at start-up.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct KeyboardSettings {
    pub rules: String,
    pub model: String,
    pub layout: String,
    pub variant: String,
    pub options: String,
}

/// The system keyboard fragment written by `localectl(1)` and read by Xorg —
/// layouts configured system-wide apply to this compositor too.
const X11_KEYBOARD_CONF: &str = "/etc/X11/xorg.conf.d/00-keyboard.conf";

/// Resolve the seat keyboard configuration, most specific source first:
///
/// 1. the `MATERIALAND_XKB_RULES`/`_MODEL`/`_LAYOUT`/`_VARIANT`/`_OPTIONS`
///    environment variables,
/// 2. the standard xkbcommon `XKB_DEFAULT_*` variables,
/// 3. the system `X11_KEYBOARD_CONF` fragment,
/// 4. the evdev/pc105/us defaults.
///
/// When more than one layout is configured and no `grp:` option picks a
/// switch chord, `grp:alt_shift_toggle` is appended so the extra layouts are
/// actually reachable.
pub fn keyboard_settings() -> KeyboardSettings {
    let system = std::fs::read_to_string(X11_KEYBOARD_CONF)
        .map(|text| parse_x11_keyboard_conf(&text))
        .unwrap_or_default();
    let mut settings = KeyboardSettings {
        rules: keyboard_setting(
            "MATERIALAND_XKB_RULES",
            "XKB_DEFAULT_RULES",
            &system.rules,
            "evdev",
        ),
        model: keyboard_setting(
            "MATERIALAND_XKB_MODEL",
            "XKB_DEFAULT_MODEL",
            &system.model,
            "pc105",
        ),
        layout: keyboard_setting(
            "MATERIALAND_XKB_LAYOUT",
            "XKB_DEFAULT_LAYOUT",
            &system.layout,
            "us",
        ),
        variant: keyboard_setting(
            "MATERIALAND_XKB_VARIANT",
            "XKB_DEFAULT_VARIANT",
            &system.variant,
            "",
        ),
        options: keyboard_setting(
            "MATERIALAND_XKB_OPTIONS",
            "XKB_DEFAULT_OPTIONS",
            &system.options,
            "",
        ),
    };
    let options = group_switch(&settings.layout, &settings.options);
    if options != settings.options {
        tracing::info!(
            "several keyboard layouts are configured; switching them with Alt+Shift \
             (set MATERIALAND_XKB_OPTIONS to choose another chord)"
        );
    }
    settings.options = options;
    settings
}

/// First non-empty value of the `app`/standard-xkb environment variables,
/// then the system fragment's value, then the fallback.
fn keyboard_setting(app_var: &str, xkb_var: &str, system: &str, fallback: &str) -> String {
    std::env::var(app_var)
        .ok()
        .filter(|value| !value.is_empty())
        .or_else(|| std::env::var(xkb_var).ok().filter(|value| !value.is_empty()))
        .unwrap_or_else(|| {
            let value = if system.is_empty() { fallback } else { system };
            value.to_owned()
        })
}

/// Extract the `Xkb*` options from an Xorg keyboard fragment, for example
/// `Option "XkbLayout" "us,ru"` lines written by `localectl(1)`.
fn parse_x11_keyboard_conf(text: &str) -> KeyboardSettings {
    let mut settings = KeyboardSettings::default();
    for line in text.lines() {
        // `\tOption "XkbLayout" "us,ru"` splits into
        // ["\tOption ", "XkbLayout", " ", "us,ru", ""].
        let parts: Vec<&str> = line.split('"').collect();
        if parts.len() < 4 {
            continue;
        }
        let value = parts[3].trim().to_owned();
        match parts[1] {
            "XkbRules" => settings.rules = value,
            "XkbModel" => settings.model = value,
            "XkbLayout" => settings.layout = value,
            "XkbVariant" => settings.variant = value,
            "XkbOptions" => settings.options = value,
            _ => {}
        }
    }
    settings
}

/// Append `grp:alt_shift_toggle` when several layouts are configured but no
/// `grp:` option selects a switch chord — without one the second layout could
/// never be reached.
fn group_switch(layout: &str, options: &str) -> String {
    let has_switch = options
        .split(',')
        .any(|option| option.trim().starts_with("grp:"));
    if !layout.contains(',') || has_switch {
        return options.to_owned();
    }
    if options.is_empty() {
        "grp:alt_shift_toggle".to_owned()
    } else {
        format!("{options},grp:alt_shift_toggle")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn app(exec: &str, terminal: bool) -> AppEntry {
        AppEntry {
            name: "x".into(),
            exec: exec.into(),
            icon: "sample-icon".into(),
            terminal,
            desktop_file: None,
        }
    }

    #[test]
    fn exec_plain() {
        assert_eq!(app("foo bar baz", false).command(), ["foo", "bar", "baz"]);
    }

    #[test]
    fn exec_quotes_and_spaces() {
        assert_eq!(
            app("sh -c \"echo hi there\"", false).command(),
            ["sh", "-c", "echo hi there"]
        );
    }

    #[test]
    fn exec_field_codes_expanded_or_stripped() {
        assert_eq!(app("foo %U --bar %f %%", false).command(), ["foo", "--bar", "%"]);
        assert_eq!(app("foo %c %i", false).command(), ["foo", "x", "--icon", "sample-icon"]);
    }

    #[test]
    fn exec_escaped_quote() {
        assert_eq!(app("foo a\\\"b c", false).command(), ["foo", "a\"b", "c"]);
    }

    #[test]
    fn terminal_backend_wraps() {
        assert_eq!(app("htop", true).command(), ["kitty", "htop"]);
    }

    #[test]
    fn parse_desktop_entry() {
        let path = std::env::temp_dir().join("materialand_cfg_test.desktop");
        std::fs::write(
            &path,
            "[Desktop Entry]\nName=Test App\nExec=foo %U\nIcon=foo\nTerminal=false\n",
        )
        .unwrap();
        let parsed = parse_desktop(&path).unwrap();
        assert_eq!(parsed.name, "Test App");
        assert!(!parsed.terminal);
        assert_eq!(parsed.command(), ["foo"]);
        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn parse_desktop_hidden_rejected() {
        let path = std::env::temp_dir().join("materialand_cfg_hidden.desktop");
        std::fs::write(
            &path,
            "[Desktop Entry]\nName=Hidden\nExec=foo\nNoDisplay=true\n",
        )
        .unwrap();
        assert!(parse_desktop(&path).is_none());
        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn key_bindings_match_the_documented_controls() {
        assert_eq!(binding_for(true, BindingKey::Char('p')), Some(BindingAction::Launcher));
        assert_eq!(binding_for(true, BindingKey::Return), Some(BindingAction::Terminal));
        assert_eq!(binding_for(true, BindingKey::Char('d')), Some(BindingAction::ToggleTheme));
        assert_eq!(binding_for(true, BindingKey::Char('q')), Some(BindingAction::CloseWindow));
    }

    #[test]
    fn key_bindings_need_super_and_an_exact_key() {
        assert_eq!(binding_for(false, BindingKey::Char('p')), None);
        assert_eq!(binding_for(false, BindingKey::Return), None);
        assert_eq!(binding_for(true, BindingKey::Char('z')), None);
    }

    #[test]
    fn parses_localectl_keyboard_fragment() {
        let conf = "# Written by systemd-localed(8), read by systemd-localed and Xorg.\n\
                    Section \"InputClass\"\n\
                    \tIdentifier \"system-keyboard\"\n\
                    \tMatchIsKeyboard \"on\"\n\
                    \tOption \"XkbLayout\" \"us,ru\"\n\
                    \tOption \"XkbModel\" \"pc105+inet\"\n\
                    \tOption \"XkbOptions\" \"terminate:ctrl_alt_bksp\"\n\
                    EndSection\n";
        let parsed = parse_x11_keyboard_conf(conf);
        assert_eq!(parsed.layout, "us,ru");
        assert_eq!(parsed.model, "pc105+inet");
        assert_eq!(parsed.options, "terminate:ctrl_alt_bksp");
        assert_eq!(parsed.variant, "");
        assert_eq!(parsed.rules, "");
    }

    #[test]
    fn group_switch_appended_for_multiple_layouts() {
        assert_eq!(group_switch("us,ru", ""), "grp:alt_shift_toggle");
        assert_eq!(
            group_switch("us,ru", "terminate:ctrl_alt_bksp"),
            "terminate:ctrl_alt_bksp,grp:alt_shift_toggle"
        );
        assert_eq!(group_switch("us,ru", "grp:caps_toggle"), "grp:caps_toggle");
        assert_eq!(
            group_switch("us,ru", "compose:ralt, grp:caps_toggle"),
            "compose:ralt, grp:caps_toggle"
        );
    }

    #[test]
    fn group_switch_left_alone_for_a_single_layout() {
        assert_eq!(group_switch("us", ""), "");
        assert_eq!(
            group_switch("us", "terminate:ctrl_alt_bksp"),
            "terminate:ctrl_alt_bksp"
        );
    }
}
