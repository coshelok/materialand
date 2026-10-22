use super::*;
#[test]
fn inertia_distance_is_independent_of_refresh_rate() {
    let expected = coast(1000.0, 0.5, 9.0);
    for hz in [30, 60, 144, 240, 360] {
        let (mut distance, mut speed) = (0.0, 1000.0);
        for _ in 0..hz / 2 {
            let step = coast(speed, 1.0 / f64::from(hz), 9.0);
            distance += step.0;
            speed = step.1;
        }
        assert!((distance - expected.0).abs() < 0.00001);
        assert!((speed - expected.1).abs() < 0.00001);
    }
}

fn state() -> (smithay::reexports::wayland_server::Display<State>, State) {
    let mut display = smithay::reexports::wayland_server::Display::new().unwrap();
    let apps = (0..96)
        .map(|i| config::AppEntry {
            name: format!("App {i}"),
            exec: "/bin/true".into(),
            icon: String::new(),
            terminal: false,
            desktop_file: Some(format!("/apps/{i}.desktop").into()),
        })
        .collect();
    let mut state = State::new(
        &mut display,
        crate::graphics::text::Fonts::load().unwrap(),
        apps,
        vec![],
        crate::theme::ThemeSpec::default(),
    );
    state.shell.set_reduce_motion(true);
    state.shell.launcher_open = true;
    state.shell.set_launcher_open(true);
    state.shell.set_reduce_motion(false);
    let r = shell::launcher_viewport(state.display.size.w as f32, state.display.size.h as f32);
    state.cursor = (f64::from(r.x + r.w / 2.0), f64::from(r.y + r.h / 2.0)).into();
    (display, state)
}
fn flick(state: &mut State) {
    assert!(input(state, "pad".into(), None, Some(20.0), 1000));
    assert!(input(state, "pad".into(), None, Some(20.0), 1016));
    assert!(input(state, "pad".into(), None, Some(0.0), 1020));
}
#[test]
fn finger_motion_is_direct_release_coasts_and_touch_stops_it() {
    let (_display, mut state) = state();
    flick(&mut state);
    assert_eq!(state.shell.launcher_scroll(), 40.0);
    assert!(state.touchpad_scroll.coasting);
    input(&mut state, "pad".into(), Some(0.0), None, 1021);
    assert!(
        state.touchpad_scroll.coasting,
        "a second axis stop must not cancel the flick"
    );
    assert!(tick(&mut state, 1.0 / 144.0));
    assert!(state.shell.launcher_scroll() > 40.0);
    stop(&mut state);
    let stopped = state.shell.launcher_scroll();
    assert!(!tick(&mut state, 0.1));
    state.shell.tick(0.1);
    assert_eq!(state.shell.launcher_scroll(), stopped);
}
#[test]
fn pauses_boundaries_and_reduced_motion_do_not_leave_inertia_running() {
    let (_display, mut state) = state();
    input(&mut state, "pad".into(), None, Some(20.0), 1000);
    input(&mut state, "pad".into(), None, Some(20.0), 1016);
    input(&mut state, "pad".into(), None, Some(0.0), 1500);
    assert!(!tick(&mut state, 0.1));
    state.shell.set_reduce_motion(true);
    flick(&mut state);
    assert!(!tick(&mut state, 0.1));
    state.shell.set_reduce_motion(false);
    flick(&mut state);
    for _ in 0..300 {
        tick(&mut state, 1.0 / 60.0);
    }
    assert!(!state.touchpad_scroll.coasting);
    state.shell.scroll_launcher(1280.0, 800.0, 100000.0);
    state.shell.follow_touchpad_scroll();
    flick(&mut state);
    assert!(!tick(&mut state, 1.0 / 60.0));
}
#[test]
fn changed_layers_consume_the_old_sequence_without_scrolling_the_new_panel() {
    let (_display, mut state) = state();
    input(&mut state, "pad".into(), None, Some(20.0), 1000);
    super::super::navigation::open_shade(&mut state, true);
    input(&mut state, "pad".into(), None, Some(200.0), 1016);
    input(&mut state, "pad".into(), None, Some(0.0), 1020);
    assert_eq!(state.shell.shade_scroll, 0.0);
    assert!(!tick(&mut state, 0.1));
}
#[test]
fn device_and_column_ownership_survive_cursor_motion() {
    let (_display, mut state) = state();
    state.shell.set_reduce_motion(true);
    state.shell.launcher_open = false;
    state.shell.set_launcher_open(false);
    state.shell.set_quick_settings_open(true);
    state.shell.notifications = crate::notifications::Snapshot {
        available: true,
        items: (0..40)
            .map(|id| crate::notifications::Notice {
                id,
                app: "App".into(),
                summary: "Message".into(),
                body: "body".into(),
            })
            .collect(),
        ..Default::default()
    };
    let rect = shell::notification_panel(1280.0, 800.0);
    state.cursor = (
        f64::from(rect.x + rect.w / 2.0),
        f64::from(rect.y + rect.h / 2.0),
    )
        .into();
    input(&mut state, "pad".into(), None, Some(20.0), 1000);
    let before = state.shell.touchpad_scroll_positions();
    input(&mut state, "other".into(), None, Some(100.0), 1010);
    assert_eq!(before, state.shell.touchpad_scroll_positions());
    state.cursor = (100.0, 400.0).into();
    input(&mut state, "pad".into(), None, Some(20.0), 1016);
    assert_eq!(state.shell.shade_scroll, 0.0);
    assert!(state.shell.touchpad_scroll_positions().2 > before.2);
}
#[test]
fn home_turns_one_existing_page_per_horizontal_sequence_and_ignores_vertical_motion() {
    let (_display, mut state) = state();
    state.shell.set_reduce_motion(true);
    state.shell.launcher_open = false;
    state.shell.set_launcher_open(false);
    // State::new uses an in-memory workspace until load_home is requested.
    let per_page = shell::home_layout(1280.0, 800.0).per_page;
    state
        .shell
        .place_home_app(state.shell.apps[0].clone(), per_page)
        .unwrap();
    state.cursor = (500.0, 300.0).into();
    input(&mut state, "pad".into(), Some(1.0), Some(100.0), 1000);
    assert_eq!(
        state
            .shell
            .home_drop_index(1280.0, 800.0, state.cursor)
            .unwrap()
            / per_page,
        0
    );
    input(&mut state, "pad".into(), Some(80.0), Some(1.0), 1016);
    assert_eq!(
        state
            .shell
            .home_drop_index(1280.0, 800.0, state.cursor)
            .unwrap()
            / per_page,
        1
    );
    input(&mut state, "pad".into(), Some(180.0), None, 1032);
    assert_eq!(
        state
            .shell
            .home_drop_index(1280.0, 800.0, state.cursor)
            .unwrap()
            / per_page,
        1
    );
    input(&mut state, "pad".into(), Some(0.0), None, 1048);
    input(&mut state, "pad".into(), Some(-80.0), None, 1064);
    assert_eq!(
        state
            .shell
            .home_drop_index(1280.0, 800.0, state.cursor)
            .unwrap()
            / per_page,
        0
    );
}
