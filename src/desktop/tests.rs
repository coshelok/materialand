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
    let initial = s.theme.is_dark;
    let origin = Point::from((500.0, 16.0));
    s.toggle_theme(origin);
    assert_ne!(s.theme.is_dark, initial);
    s.toggle_theme(origin);
    assert_eq!(s.theme.is_dark, initial);
}

#[test]
fn set_variant_keeps_brightness_and_changes_palette() {
    let (_display, mut s) = fresh();
    let dark = s.theme.is_dark;
    let before = s.theme.primary;
    s.set_variant(ColorVariant::Vibrant);
    assert_eq!(s.theme.is_dark, dark, "brightness must not change");
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
        crate::shell::launcher_hit(w, h, 0, 0.0, pos),
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

#[test]
fn animated_quick_setting_click_follows_the_rendered_tile() {
    let (_display, mut state) = fresh();
    let (w, h) = (state.display.size.w as f32, state.display.size.h as f32);
    state.shell.set_quick_settings_open(true);
    for _ in 0..3 {
        state.shell.tick(1.0 / 60.0);
    }
    let progress = state.shell.quick_settings_progress();
    let frame = state.shell.render_quick_settings(
        &state.theme,
        &mut state.fonts,
        w,
        h,
        progress,
        state.brightness,
        state.theme_spec.variant,
        None,
    );
    let layer = &frame.pieces[0].0;
    let size = layer.size.unwrap();
    let card = crate::shell::quick_settings_rect(w, h);
    let tile = crate::shell::quick_setting_tile_rect(w, h, 0);
    let x = tile.x - card.x + crate::shell::SHADOW_PAD + tile.w / 2.0;
    let y = tile.y - card.y + crate::shell::SHADOW_PAD + tile.h / 2.0;
    let point = Point::from((
        layer.loc.x
            + f64::from(x * size.w as f32 / (card.w + 2.0 * crate::shell::SHADOW_PAD).ceil()),
        layer.loc.y
            + f64::from(y * size.h as f32 / (card.h + 2.0 * crate::shell::SHADOW_PAD).ceil()),
    ));
    let initial = state.theme.is_dark;
    state.shell.press_tile = Some(crate::shell::QuickAction::Theme);
    crate::input::finish_compositor_press(&mut state, point);
    assert_ne!(
        state.theme.is_dark, initial,
        "clicking the visible moving tile toggles the theme"
    );
}

#[test]
fn real_output_properties_replace_virtual_size_and_survive_resume() {
    let (_display, mut state) = fresh();
    assert_eq!(
        state.display.handle.physical_properties().size,
        (0, 0).into()
    );
    let virtual_output = state.display.handle.clone();
    let mode = state.display.handle.current_mode();
    let properties = PhysicalProperties {
        size: (344, 194).into(),
        subpixel: Subpixel::HorizontalRgb,
        make: "Unknown".into(),
        model: "eDP-1".into(),
    };
    state.set_output_identity("eDP-1".into(), properties.clone());
    let real_output = state.display.handle.clone();
    assert_ne!(real_output, virtual_output);
    assert_eq!(real_output.physical_properties().size, (344, 194).into());
    assert_eq!(real_output.current_mode(), mode);
    assert_eq!(state.windows.space.outputs().count(), 1);
    assert_eq!(state.windows.space.outputs().next(), Some(&real_output));
    state.set_output_identity("eDP-1".into(), properties);
    assert_eq!(
        state.display.handle, real_output,
        "resuming the same monitor keeps its wl_output"
    );
    assert_eq!(state.retired_output_globals.len(), 1);
    state.set_output_identity(
        "eDP-1".into(),
        PhysicalProperties {
            size: (0, 0).into(),
            subpixel: Subpixel::Unknown,
            make: "Unknown".into(),
            model: "eDP-1".into(),
        },
    );
    assert_ne!(
        state.display.handle, real_output,
        "changed monitor metadata must not stay stale"
    );
    assert_eq!(
        state.display.handle.physical_properties().size,
        (0, 0).into()
    );
}

#[test]
fn pin_menu_action_toggles_and_dismissal_never_launches_the_underlying_tile() {
    let (_display, mut state) = fresh();
    let app = state.shell.pinned[0].clone();
    let (w, h) = (state.display.size.w as f32, state.display.size.h as f32);
    state.shell.launcher_open = true;
    state
        .shell
        .open_app_menu(app.clone(), (400.0, 400.0).into());
    let rect = crate::shell::app_menu_rect(w, h, (400.0, 400.0).into());
    // Clicking the menu header must not dismiss the launcher under it.
    crate::input::finish_compositor_press(
        &mut state,
        (f64::from(rect.x + 20.0), f64::from(rect.y + 20.0)).into(),
    );
    assert!(state.shell.app_menu.is_some() && state.shell.launcher_open);
    state.shell.press_pin = true;
    let button = crate::shell::app_menu_button(rect);
    crate::input::finish_compositor_press(
        &mut state,
        (f64::from(button.x + 30.0), f64::from(button.y + 24.0)).into(),
    );
    assert!(!state.shell.is_pinned(&app));
    assert!(state.shell.app_menu.is_none() && state.shell.launcher_open);
    state.shell.consume_menu_release = true;
    crate::input::finish_compositor_press(&mut state, (0.0, 0.0).into());
    assert!(
        state.shell.launcher_open,
        "dismissing the context menu only consumes that click"
    );
}
