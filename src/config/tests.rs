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
    assert_eq!(
        app("foo %U --bar %f %%", false).command(),
        ["foo", "--bar", "%"]
    );
    assert_eq!(
        app("foo %c %i", false).command(),
        ["foo", "x", "--icon", "sample-icon"]
    );
}

#[test]
fn exec_escaped_quote() {
    assert_eq!(app("foo a\\\"b c", false).command(), ["foo", "a\"b", "c"]);
}

#[test]
fn terminal_backend_wraps() {
    assert_eq!(
        app("htop", true).command_in_terminal(&["foot".into()]),
        ["foot", "--", "htop"]
    );
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
    assert_eq!(
        binding_for(true, BindingKey::Char('p')),
        Some(BindingAction::Launcher)
    );
    assert_eq!(
        binding_for(true, BindingKey::Return),
        Some(BindingAction::Terminal)
    );
    assert_eq!(
        binding_for(true, BindingKey::Char('d')),
        Some(BindingAction::ToggleTheme)
    );
    assert_eq!(
        binding_for(true, BindingKey::Char('q')),
        Some(BindingAction::CloseWindow)
    );
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
