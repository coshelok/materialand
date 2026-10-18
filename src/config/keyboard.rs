//! Keyboard settings resolution.
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
pub(super) fn keyboard_setting(
    app_var: &str,
    xkb_var: &str,
    system: &str,
    fallback: &str,
) -> String {
    std::env::var(app_var)
        .ok()
        .filter(|value| !value.is_empty())
        .or_else(|| {
            std::env::var(xkb_var)
                .ok()
                .filter(|value| !value.is_empty())
        })
        .unwrap_or_else(|| {
            let value = if system.is_empty() { fallback } else { system };
            value.to_owned()
        })
}

/// Extract the `Xkb*` options from an Xorg keyboard fragment, for example
/// `Option "XkbLayout" "us,ru"` lines written by `localectl(1)`.
pub(super) fn parse_x11_keyboard_conf(text: &str) -> KeyboardSettings {
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
pub(super) fn group_switch(layout: &str, options: &str) -> String {
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
