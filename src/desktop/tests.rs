use super::*;
use smithay::reexports::wayland_server::Display;

fn fresh() -> (Display<State>, State) {
    let mut display = Display::<State>::new().expect("display");
    let state = State::new(
        &mut display,
        Fonts::load().expect("test system font"),
        vec![],
        vec![],
        ThemeSpec::DEFAULT,
    );
    (display, state)
}

#[test]
fn cascade_stays_within_output() {
    let (_display, mut s) = fresh();
    let (ww, wh) = config::DEFAULT_WINDOW_SIZE;
    let dock = config::DOCK_HEIGHT as i32 + config::DOCK_MARGIN as i32;
    for _ in 0..30 {
        let p = s.next_location();
        assert!(p.x >= 0 && p.y >= 0);
        assert!(p.x + ww <= s.display.size.w, "window spills right: {p:?}");
        assert!(
            p.y + wh <= s.display.size.h - dock,
            "window hits dock: {p:?}"
        );
    }
}

#[test]
fn dock_item_count_matches_the_built_list() {
    let (_display, s) = fresh();
    assert_eq!(
        crate::input::dock_item_count(&s),
        crate::input::dock_items(&s).len()
    );
}

#[test]
fn theme_toggle_switches_scheme() {
    let (_display, mut s) = fresh();
    assert!(s.theme.is_dark);
    let origin = Point::from((500.0, 16.0));
    s.toggle_theme(origin);
    assert!(!s.theme.is_dark);
    s.toggle_theme(origin);
    assert!(s.theme.is_dark);
}

#[test]
fn set_variant_keeps_brightness_and_changes_palette() {
    let (_display, mut s) = fresh();
    assert!(s.theme.is_dark);
    let before = s.theme.primary;
    s.set_variant(ColorVariant::Vibrant);
    assert!(s.theme.is_dark, "brightness must not change");
    assert_eq!(s.theme_spec.variant, ColorVariant::Vibrant);
    assert_ne!(s.theme.primary, before, "palette must change");
}

#[test]
fn clicking_the_dock_launcher_keeps_the_launcher_open() {
    let (_display, mut s) = fresh();
    let (w, h) = (s.display.size.w as f32, s.display.size.h as f32);
    let items = crate::input::dock_items(&s);
    let rect = crate::shell::dock_item_rect(w, h, items.len(), 0);
    let pos = Point::from((
        (rect.x + rect.w / 2.0) as f64,
        (rect.y + rect.h / 2.0) as f64,
    ));
    // The press half of the click: the dock button swallows it. The
    // release must open the launcher and leave it open.
    s.shell.press_dock = Some(0);
    crate::input::finish_compositor_press(&mut s, pos);
    assert!(
        s.shell.launcher_open,
        "the launcher must stay open after the click that opened it"
    );
}

#[test]
fn clicking_the_launcher_scrim_dismisses_it() {
    let (_display, mut s) = fresh();
    s.shell.launcher_open = true;
    let (w, h) = (s.display.size.w as f32, s.display.size.h as f32);
    // Inside the overlay, outside the centred card: a bare scrim click.
    let pos = Point::from((12.0, h as f64 - 12.0));
    assert_eq!(
        crate::shell::launcher_hit(w, h, 0, pos),
        crate::shell::LauncherHit::Scrim
    );
    crate::input::finish_compositor_press(&mut s, pos);
    assert!(
        !s.shell.launcher_open,
        "a scrim click must dismiss the launcher"
    );
}

#[test]
fn user_button_opens_the_session_menu_without_quitting() {
    let (_display, mut s) = fresh();
    crate::input::perform_top_action(&mut s, crate::shell::TopAction::Power);
    assert!(s.shell.power_open, "the user button opens the session menu");
    assert!(s.running, "opening the menu must not stop the compositor");
    crate::input::perform_top_action(&mut s, crate::shell::TopAction::Power);
    assert!(!s.shell.power_open, "a second press closes the menu");
    assert!(s.running);
}

#[test]
fn session_menu_logout_stops_the_compositor() {
    let (_display, mut s) = fresh();
    s.shell.power_open = true;
    crate::input::perform_power_action(&mut s, crate::shell::PowerAction::Logout);
    assert!(!s.running, "log out must stop the event loop");
    assert!(!s.shell.power_open, "picking an action closes the menu");
}

#[test]
fn serial_advances_monotonically() {
    let (_display, mut s) = fresh();
    let a: u32 = s.next_serial().into();
    let b: u32 = s.next_serial().into();
    assert!(b > a);
}

#[test]
fn output_changes_update_cursor_and_protocol_mode_together() {
    let (_display, mut state) = fresh();
    state.cursor = (1200.0, 700.0).into();
    let mode = Mode {
        size: (640, 480).into(),
        refresh: 59_940,
    };
    state.set_output_mode(mode);
    assert_eq!(state.display.size, (640, 480).into());
    assert_eq!(state.display.handle.current_mode(), Some(mode));
    assert_eq!(state.cursor, (639.0, 479.0).into());
    state.set_output_mode(Mode {
        size: (0, 0).into(),
        refresh: 60_000,
    });
    assert_eq!(state.display.handle.current_mode(), Some(mode));
}

#[test]
fn seat_pause_releases_keys_buttons_and_shell_grabs() {
    let (_display, mut state) = fresh();
    let keyboard = state.keyboard.clone();
    let serial = state.next_serial();
    keyboard.input::<(), _>(
        &mut state,
        38u32.into(),
        smithay::backend::input::KeyState::Pressed,
        serial,
        10,
        |_, _, _| smithay::input::keyboard::FilterResult::Forward,
    );
    state.interaction = Interaction::Brightness;
    state.swallowed_buttons.insert(0x110);
    state.shell.press_top = Some(crate::shell::TopAction::Power);
    crate::input::cancel(&mut state);
    assert!(keyboard.pressed_keys().is_empty());
    assert!(state.swallowed_buttons.is_empty());
    assert!(state.forwarded_buttons.is_empty());
    assert!(matches!(state.interaction, Interaction::None));
    assert!(state.shell.press_top.is_none());
}
