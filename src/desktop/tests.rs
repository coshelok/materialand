use super::*;
use smithay::reexports::wayland_server::Display;

fn fresh() -> (Display<State>, State) {
    let mut display = Display::<State>::new().expect("display");
    let state = State::new(
        &mut display,
        Fonts::load().expect("test system font"),
        vec![],
        vec![],
        ThemeSpec::default(),
    );
    (display, state)
}

#[test]
fn opening_overlay_consumes_old_release_and_seat_reset_clears_the_latch() {
    let (_display, mut state) = fresh();
    // A compositor-owned press is still held when a keyboard shortcut opens
    // the launcher. Its release is neither a launcher click nor a dismissal.
    state.swallowed_buttons.insert(0x110);
    crate::input::prepare_shell_overlay(&mut state);
    state.shell.launcher_open = true;
    state.swallowed_buttons.remove(&0x110);
    crate::input::finish_compositor_press(&mut state, (0.0, 0.0).into());
    assert!(state.shell.launcher_open);
    assert!(!state.shell.consume_menu_release, "consumed exactly once");

    state.swallowed_buttons.insert(0x110);
    crate::input::prepare_shell_overlay(&mut state);
    assert!(state.shell.consume_menu_release);
    crate::input::cancel(&mut state);
    assert!(
        !state.shell.consume_menu_release,
        "VT reset must not swallow the next click"
    );
    assert!(state.swallowed_buttons.is_empty());

    crate::input::prepare_shell_overlay(&mut state);
    assert!(
        !state.shell.consume_menu_release,
        "opening with no held button needs no release"
    );
}

#[test]
fn cascade_stays_within_output() {
    let (_display, mut s) = fresh();
    let (ww, wh) = (
        crate::config::design().window.width as i32,
        crate::config::design().window.height as i32,
    );
    let reserved_bottom = crate::shell::bottom_inset() as i32;
    for _ in 0..30 {
        let p = s.next_location();
        assert!(p.x >= 0 && p.y >= 0);
        assert!(p.x + ww <= s.display.size.w, "window spills right: {p:?}");
        assert!(
            p.y + wh <= s.display.size.h - reserved_bottom,
            "window hits navigation: {p:?}"
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
#[ignore = "explicit CPU timing of theme input handling, not a display FPS benchmark"]
fn measure_theme_input_preparation() {
    let (_display, mut state) = fresh();
    state.display.size = (1280, 800).into();
    for shade_open in [false, true] {
        state.shell.set_quick_settings_open(shade_open);
        for sample in 0..3 {
            let start = std::time::Instant::now();
            state.toggle_theme((100.0, 100.0).into());
            println!(
                "theme input: shade={shade_open}, sample={sample}, elapsed={:?}",
                start.elapsed()
            );
        }
    }
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
fn appearance_controls_save_in_background_and_restore_before_rendering() {
    let dir = std::env::temp_dir().join(format!(
        "materialand-desktop-appearance-{}",
        std::process::id()
    ));
    let path = dir.join("config.toml");
    let _ = std::fs::remove_dir_all(&dir);
    let (_display, mut state) = fresh();
    state.restore_appearance(config::AppearanceStore::at(Some(path.clone())));
    if !state.theme.is_dark {
        state.toggle_theme((500.0, 16.0).into());
    } else {
        state.toggle_theme((500.0, 16.0).into());
        state.toggle_theme((500.0, 16.0).into());
    }
    state.set_variant(ColorVariant::Vibrant);
    state.appearance_writer.flush();
    let chosen = config::Appearance {
        dark: true,
        variant: ColorVariant::Vibrant,
        reduce_motion: false,
    };
    let defaults = config::Appearance {
        dark: false,
        variant: ColorVariant::Neutral,
        reduce_motion: false,
    };
    assert_eq!(
        config::AppearanceStore::at(Some(path.clone()))
            .load(defaults)
            .unwrap(),
        chosen
    );

    // Read while the first session is still alive: shutdown is not needed to save.
    let (_display, mut restarted) = fresh();
    restarted.restore_appearance(config::AppearanceStore::at(Some(path.clone())));
    assert!(restarted.theme.is_dark);
    assert_eq!(restarted.theme_spec.variant, ColorVariant::Vibrant);
    assert_eq!(restarted.theme, restarted.theme_spec.dark());
    assert!(restarted.shell.reveal_frame().is_none());
    assert!(restarted.shell.theme_fade().is_none());

    restarted.toggle_theme((500.0, 16.0).into());
    restarted.set_variant(ColorVariant::TonalSpot);
    restarted.appearance_writer.flush();
    let (_display, mut third) = fresh();
    third.restore_appearance(config::AppearanceStore::at(Some(path)));
    assert!(!third.theme.is_dark);
    assert_eq!(third.theme_spec.variant, ColorVariant::TonalSpot);
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn failed_appearance_save_preserves_invalid_config_and_retry_clears_error() {
    let dir = std::env::temp_dir().join(format!(
        "materialand-appearance-retry-{}",
        std::process::id()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("config.toml");
    let invalid = "# user edits must survive\n[appearance\n";
    std::fs::write(&path, invalid).unwrap();
    let (_display, mut state) = fresh();
    state.restore_appearance(config::AppearanceStore::at(Some(path.clone())));
    assert_eq!(
        state.shell.appearance_error.as_deref(),
        Some("Config needs repair")
    );
    state.toggle_reduce_motion();
    state.appearance_writer.flush();
    state.poll_appearance();
    assert_eq!(std::fs::read_to_string(&path).unwrap(), invalid);
    assert!(state.shell.appearance_error.is_some());

    std::fs::write(&path, "# repaired by user\n[appearance]\n").unwrap();
    state.save_appearance();
    assert!(
        state.shell.appearance_error.is_some(),
        "a queued retry is not yet a successful save"
    );
    state.appearance_writer.flush();
    assert!(state.poll_appearance());
    assert!(state.shell.appearance_error.is_none());
    let saved = std::fs::read_to_string(&path).unwrap();
    assert!(saved.contains("# repaired by user"));
    assert!(saved.contains("reduce_motion = true"));
    std::fs::remove_dir_all(dir).unwrap();
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
    // The bottom now belongs to the full-height drawer; only the area above it is outside.
    let pos = Point::from((12.0, 12.0));
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
    let layer = &frame.pieces[2].0;
    let source = layer.source.expect("cropped shade body");
    let size = layer.size.unwrap();
    let card = crate::shell::quick_settings_rect(w, h);
    let tile = crate::shell::quick_setting_tile_rect(w, h, 0);
    let pad = crate::config::design().shadow.padding;
    let point = Point::from((
        layer.loc.x
            + f64::from(
                (tile.x - card.x + pad + tile.w / 2.0 - source.x) * size.w as f32 / source.w,
            ),
        layer.loc.y
            + f64::from(
                (tile.y - card.y + pad + tile.h / 2.0 - source.y) * size.h as f32 / source.h,
            ),
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

#[test]
fn home_menu_adds_and_removes_a_shortcut_without_touching_the_dock() {
    let (_display, mut state) = fresh();
    let app = state.shell.pinned[0].clone();
    let point = (400.0, 400.0).into();
    let (w, h) = (state.display.size.w as f32, state.display.size.h as f32);
    state.shell.open_app_menu(app.clone(), point);
    state.shell.app_menu.as_mut().unwrap().press_home = true;
    let button = crate::shell::app_menu_home_button(crate::shell::app_menu_rect(w, h, point));
    crate::input::finish_compositor_press(
        &mut state,
        (f64::from(button.x + 30.0), f64::from(button.y + 24.0)).into(),
    );
    assert!(state.shell.is_on_home(&app) && state.shell.is_pinned(&app));
    assert!(state.shell.app_menu.is_none());
    state.shell.open_app_menu(app.clone(), point);
    state.shell.activate_home_menu();
    assert!(!state.shell.is_on_home(&app) && state.shell.is_pinned(&app));
}

#[test]
fn returning_home_closes_panels_and_ends_compositor_grabs() {
    let (_display, mut state) = fresh();
    state.shell.launcher_open = true;
    state.shell.set_launcher_open(true);
    state.shell.set_brightness_open(true);
    state.shell.set_quick_settings_open(true);
    state.shell.set_calendar_open(true);
    state.shell.set_power_open(true);
    state.shell.home.pressed = Some(crate::shell::HomeHit::Next);
    state.interaction = Interaction::Brightness;
    crate::input::show_home(&mut state);
    assert!(matches!(state.interaction, Interaction::None));
    assert!(!state.shell.launcher_open && !state.shell.calendar.open && !state.shell.power_open);
    assert!(!state.shell.brightness_open && !state.shell.quick_settings_open);
    assert!(state.shell.home.pressed.is_none());
}

#[test]
fn three_finger_swipes_open_and_close_the_launcher_without_stale_toggles() {
    let (_display, mut state) = fresh();
    state.shell.set_reduce_motion(true);
    crate::input::swipe_begin(&mut state, "pad".into(), 3);
    state.launcher_swipe.update("pad", 0.0, -100.0);
    crate::input::swipe_end(&mut state, "pad", false);
    assert!(state.shell.launcher_open);
    crate::input::swipe_begin(&mut state, "pad".into(), 3);
    state.launcher_swipe.update("pad", 0.0, 100.0);
    crate::input::swipe_end(&mut state, "pad", false);
    assert!(!state.shell.launcher_open);
    crate::input::swipe_begin(&mut state, "pad".into(), 3);
    state.launcher_swipe.update("pad", 0.0, -100.0);
    state.shell.launcher_open = true;
    state.shell.set_launcher_open(true);
    crate::input::swipe_end(&mut state, "pad", false);
    assert!(
        state.shell.launcher_open,
        "a concurrent keyboard action is not undone by an old gesture"
    );
}
