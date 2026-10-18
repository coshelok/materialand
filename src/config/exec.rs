//! Desktop Exec field decoding without a shell.
use super::*;
/// Parse a Desktop Entry `Exec` value without invoking a shell. This handles
/// quoting, the Exec-specific escapes, and the field codes used by launchers.
pub(super) fn split_exec(
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
