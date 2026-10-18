//! One terminal selection is shared by the dock, shortcuts and Terminal=true entries.
use super::*;
const PROGRAMS: &[&str] = &[
    "kitty",
    "alacritty",
    "wezterm",
    "konsole",
    "gnome-terminal",
    "kgx",
    "xfce4-terminal",
    "mate-terminal",
    "terminator",
    "foot",
];
pub fn select_terminal(apps: &[AppEntry]) -> AppEntry {
    select_with(apps, apps::executable_available)
}
fn select_with(apps: &[AppEntry], available: impl Fn(&str) -> bool) -> AppEntry {
    let mut candidates: Vec<_> = apps
        .iter()
        .filter(|entry| {
            if entry.terminal {
                return false;
            }
            let name = format!("{} {}", entry.name, entry.exec).to_lowercase();
            (name.contains("terminal") || PROGRAMS.iter().any(|p| name.contains(p)))
                && entry.command().first().is_some_and(|p| available(p))
        })
        .collect();
    candidates.sort_by_key(|entry| {
        let name = format!("{} {}", entry.name, entry.exec).to_lowercase();
        if name.contains("kitty") {
            0
        } else if name.contains("foot") {
            2
        } else {
            1
        }
    });
    if let Some(entry) = candidates.first() {
        return (*entry).clone();
    }
    let program = std::iter::once("foot")
        .chain(PROGRAMS.iter().copied().filter(|p| *p != "foot"))
        .find(|p| available(p))
        .unwrap_or("foot");
    AppEntry {
        name: "Terminal".into(),
        exec: program.into(),
        icon: "utilities-terminal".into(),
        terminal: false,
        desktop_file: None,
    }
}
pub(super) fn wrap(terminal: &[String], args: Vec<String>) -> Vec<String> {
    let Some(program) = terminal.first() else {
        return Vec::new();
    };
    if args.is_empty() {
        return Vec::new();
    }
    let mut command = terminal.to_vec();
    let name = Path::new(program)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or(program);
    match name {
        "foot" | "footclient" | "kitty" | "gnome-terminal" | "kgx" => command.push("--".into()),
        "wezterm" => {
            if !command.iter().skip(1).any(|s| s == "start") {
                command.push("start".into());
            }
            command.push("--".into());
        }
        "xfce4-terminal" | "mate-terminal" | "terminator" => command.push("-x".into()),
        _ => command.push("-e".into()),
    }
    command.extend(args);
    command
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn fallback_uses_an_installed_program_even_without_desktop_files() {
        assert_eq!(
            select_with(&[], |p| p == "alacritty").command(),
            ["alacritty"]
        );
        assert_eq!(
            select_with(&[], |p| p == "foot" || p == "kitty").command(),
            ["foot"]
        );
    }
    #[test]
    fn execute_flags_and_whitespace_arguments_are_preserved() {
        assert_eq!(
            wrap(
                &["/usr/bin/foot".into()],
                ["editor".into(), "a file.txt".into()].into()
            ),
            ["/usr/bin/foot", "--", "editor", "a file.txt"]
        );
        assert_eq!(
            wrap(&["wezterm".into(), "start".into()], vec!["htop".into()]),
            ["wezterm", "start", "--", "htop"]
        );
        assert_eq!(
            wrap(&["alacritty".into()], vec!["htop".into()]),
            ["alacritty", "-e", "htop"]
        );
    }
}
