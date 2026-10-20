use super::*;

fn geom(x: i32, y: i32, w: i32, h: i32) -> Rectangle<i32, Logical> {
    Rectangle::new((x, y).into(), (w, h).into())
}

fn edges(left: bool, right: bool, top: bool, bottom: bool) -> ResizeEdges {
    ResizeEdges {
        left,
        right,
        top,
        bottom,
    }
}

#[test]
fn resize_right_grows_width_keeps_origin() {
    let (g, size) = compute_resize(
        geom(100, 100, 400, 300),
        edges(false, true, false, false),
        50,
        0,
    );
    assert_eq!(size, (450, 300));
    assert_eq!(g.loc, (100, 100).into());
}

#[test]
fn resize_left_keeps_right_edge_fixed() {
    let (g, size) = compute_resize(
        geom(100, 100, 400, 300),
        edges(true, false, false, false),
        50,
        0,
    );
    assert_eq!(size, (350, 300));
    assert_eq!(g.loc.x, 150);
    assert_eq!(g.loc.x + g.size.w, 500, "right edge must not move");
}

#[test]
fn resize_top_keeps_bottom_edge_fixed() {
    let (g, size) = compute_resize(
        geom(100, 100, 400, 300),
        edges(false, false, true, false),
        0,
        30,
    );
    assert_eq!(size, (400, 270));
    assert_eq!(g.loc.y, 130);
    assert_eq!(g.loc.y + g.size.h, 400, "bottom edge must not move");
}

#[test]
fn resize_clamps_to_minimum_size() {
    let (g, size) = compute_resize(
        geom(100, 100, 400, 300),
        edges(true, false, false, false),
        1000,
        0,
    );
    assert_eq!(size.0, 320);
    assert_eq!(
        g.loc.x + g.size.w,
        500,
        "right edge stays fixed when clamped"
    );

    let (_, size) = compute_resize(
        geom(100, 100, 400, 300),
        edges(false, false, false, true),
        0,
        -1000,
    );
    assert_eq!(size.1, 200);
}

#[test]
fn spawn_ignores_empty_command() {
    // No program name: must be a no-op rather than a panic/index error.
    spawn_program(&[]);
}

#[test]
fn animated_clock_navigation_uses_drawn_geometry_and_matching_release() {
    use crate::{graphics::text::Fonts, theme::ThemeSpec};
    use smithay::reexports::wayland_server::Display;
    let mut display = Display::new().unwrap();
    let mut state = State::new(
        &mut display,
        Fonts::load().unwrap(),
        vec![],
        vec![],
        ThemeSpec::default(),
    );
    state.shell.set_calendar_open(true);
    state.shell.calendar.year = 2026;
    state.shell.calendar.month = 12;
    let (w, h) = (state.display.size.w as f32, state.display.size.h as f32);
    let placement = state.shell.calendar_placement(w, h);
    let source = placement.source;
    let position = Point::from((
        f64::from(placement.target.x + (source.w - 36.0) * placement.target.w / source.w),
        f64::from(placement.target.y + 168.0 * placement.target.h / source.h),
    ));
    assert!(matches!(
        target::target(&state, position),
        target::Target::Shell(target::Control::Calendar(shell::CalendarAction::Next))
    ));
    target::press(
        &mut state,
        target::Control::Calendar(shell::CalendarAction::Next),
        position,
    );
    actions::finish_compositor_press(&mut state, position);
    assert_eq!(
        (state.shell.calendar.year, state.shell.calendar.month),
        (2027, 1)
    );
    target::press(
        &mut state,
        target::Control::Calendar(shell::CalendarAction::Next),
        position,
    );
    actions::finish_compositor_press(&mut state, (900.0, 650.0).into());
    assert_eq!(
        (state.shell.calendar.year, state.shell.calendar.month),
        (2027, 1)
    );
}

fn tablet_state() -> (smithay::reexports::wayland_server::Display<State>, State) {
    let mut display = smithay::reexports::wayland_server::Display::new().unwrap();
    let state = State::new(
        &mut display,
        crate::graphics::text::Fonts::load().unwrap(),
        vec![],
        vec![],
        crate::theme::ThemeSpec::default(),
    );
    (display, state)
}
#[test]
fn navigation_press_release_keeps_overview_open_and_gestures_select_distinct_layers() {
    let (_display, mut state) = tablet_state();
    state.shell.set_reduce_motion(true);
    target::press(
        &mut state,
        target::Control::Nav(shell::NavAction::Overview),
        (640.0, 760.0).into(),
    );
    actions::finish_compositor_press(&mut state, (640.0, 760.0).into());
    assert!(state.overview.open);
    let swipe = |state: &mut State, fingers, dy| {
        swipe_begin(state, "pad".into(), fingers);
        state.launcher_swipe.update("pad", 0.0, dy);
        swipe_end(state, "pad", false);
    };
    swipe(&mut state, 4, 100.0); // Overview owns the downward dismissal.
    assert!(!state.overview.open && !state.shell.quick_settings_open);
    swipe(&mut state, 4, 100.0);
    assert!(state.shell.quick_settings_open);
    swipe(&mut state, 3, -100.0); // Other global gestures cannot replace the shade.
    assert!(state.shell.quick_settings_open && !state.shell.launcher_open);
    swipe(&mut state, 4, -100.0);
    assert!(!state.overview.open && !state.shell.quick_settings_open);
    swipe(&mut state, 4, -100.0);
    assert!(state.overview.open);
    swipe(&mut state, 3, -100.0);
    assert!(state.overview.open && !state.shell.launcher_open);
    swipe(&mut state, 4, 100.0);
    swipe(&mut state, 3, -100.0);
    assert!(state.shell.launcher_open && !state.overview.open);
    swipe(&mut state, 4, -100.0);
    assert!(state.shell.launcher_open && !state.overview.open);
    swipe(&mut state, 3, 100.0);
    assert!(!state.shell.launcher_open);
}
#[test]
fn scrolled_shade_hits_exactly_one_offset_and_keeps_header_out_of_tiles() {
    let (_display, mut state) = tablet_state();
    state.shell.set_reduce_motion(true);
    state.shell.set_quick_settings_open(true);
    state.shell.shade_scroll = 84.0;
    let r = shell::quick_setting_tile_rect(
        1280.0,
        800.0,
        shell::quick_action_index(QuickAction::ReduceMotion),
    );
    let point = (
        f64::from(r.x + r.w / 2.0),
        f64::from(r.y + r.h / 2.0 - state.shell.shade_scroll),
    )
        .into();
    assert!(matches!(
        target::target(&state, point),
        target::Target::Shell(target::Control::Quick(QuickAction::ReduceMotion))
    ));
    let card = shell::quick_settings_rect(1280.0, 800.0);
    let header = (f64::from(card.x + 40.0), f64::from(card.y + 30.0)).into();
    assert!(!matches!(
        target::target(&state, header),
        target::Target::Shell(target::Control::Quick(_))
    ));
}
#[test]
fn shade_brightness_drag_uses_its_drawn_track_and_stays_out_of_the_tiles() {
    let (_display, mut state) = tablet_state();
    state.shell.set_reduce_motion(true);
    state.shell.set_quick_settings_open(true);
    state.shell.brightness_available = true;
    let r = shell::android_shade::shade_brightness_rect(1280.0, 800.0);
    let point = (f64::from(r.x + r.w / 2.0), f64::from(r.y + r.h / 2.0)).into();
    let target::Target::Shell(target::Control::ShadeBrightness(value)) =
        target::target(&state, point)
    else {
        panic!("drawn brightness capsule was not routed to its slider");
    };
    target::press(&mut state, target::Control::ShadeBrightness(value), point);
    assert!(matches!(state.interaction, Interaction::ShadeBrightness));
    on_pointer_motion(
        &mut state,
        (f64::from(r.x - 20.0), f64::from(r.y + r.h / 2.0)).into(),
        1,
    );
    assert_eq!(state.brightness, shell::BRIGHTNESS_MIN);
    on_pointer_motion(
        &mut state,
        (f64::from(r.right() + 20.0), f64::from(r.y + r.h / 2.0)).into(),
        2,
    );
    assert_eq!(state.brightness, shell::BRIGHTNESS_MAX);
    state.finish_interaction(BTN_LEFT);
    assert!(matches!(state.interaction, Interaction::None));
    state.shell.brightness_available = false;
    assert!(matches!(
        target::target(&state, point),
        target::Target::Shell(target::Control::Block)
    ));
}
#[test]
fn home_search_opens_the_app_drawer_from_the_visible_capsule() {
    let (_display, mut state) = tablet_state();
    state.shell.set_reduce_motion(true);
    let r = shell::home_layout(1280.0, 800.0).search;
    let point = (f64::from(r.x + r.w / 2.0), f64::from(r.y + r.h / 2.0)).into();
    assert!(matches!(
        target::target(&state, point),
        target::Target::Shell(target::Control::Home(shell::HomeHit::Search))
    ));
    target::press(
        &mut state,
        target::Control::Home(shell::HomeHit::Search),
        point,
    );
    actions::finish_compositor_press(&mut state, point);
    assert!(state.shell.launcher_open);
    assert!(!state.overview.open && !state.shell.quick_settings_open);
}
#[test]
fn shortcut_drag_moves_icon_without_launching_on_release() {
    let (_display, mut state) = tablet_state();
    state.shell.set_reduce_motion(true);
    let app = config::AppEntry {
        name: "Never spawn this test".into(),
        exec: "/does/not/exist".into(),
        icon: "apps".into(),
        terminal: false,
        desktop_file: None,
    };
    state.shell.toggle_home_app(app.clone()).unwrap();
    let from = state.shell.home_tile_rect(1280.0, 800.0, 0);
    let start = (f64::from(from.x + 20.0), f64::from(from.y + 30.0)).into();
    target::press(
        &mut state,
        target::Control::Home(shell::HomeHit::App(0)),
        start,
    );
    let moved = (start.x + 60.0, start.y + 20.0).into();
    assert!(super::shortcut_drag::motion(&mut state, moved));
    assert!(state.shortcut_drag.as_ref().unwrap().active);
    actions::finish_compositor_press(&mut state, moved);
    assert!(state.shortcut_drag.is_none());
    assert!(state.shell.home.pressed.is_none());
    assert!(state.shell.is_on_home(&app));
}

#[test]
fn closing_shade_animation_does_not_become_a_new_overview_gesture() {
    let (_display, mut state) = tablet_state();
    state.shell.set_quick_settings_open(true);
    state.shell.tick(0.05);
    swipe_begin(&mut state, "pad".into(), 4);
    state.launcher_swipe.update("pad", 0.0, -100.0);
    swipe_end(&mut state, "pad", false);
    assert!(!state.shell.quick_settings_open && state.shell.quick_settings_progress() > 0.001);
    swipe_begin(&mut state, "pad".into(), 4);
    state.launcher_swipe.update("pad", 0.0, -100.0);
    swipe_end(&mut state, "pad", false);
    assert!(!state.overview.open);
}
#[test]
fn status_strip_taps_open_matching_panels_and_clock_can_toggle_closed() {
    let (_display, mut state) = tablet_state();
    state.shell.set_reduce_motion(true);
    let tap = |state: &mut State, action| {
        let r = shell::statusbar_button(1280.0, action);
        let point = (f64::from(r.x + r.w / 2.0), f64::from(r.h / 2.0)).into();
        let target::Target::Shell(control) = target::target(state, point) else {
            panic!("status must own the tap");
        };
        assert!(matches!(control, target::Control::Status(hit) if hit == action));
        target::dismiss_outside(state, point);
        target::press(state, control, point);
        actions::finish_compositor_press(state, point);
    };
    tap(&mut state, shell::StatusAction::Clock);
    assert!(state.shell.calendar.open);
    tap(&mut state, shell::StatusAction::Clock);
    assert!(!state.shell.calendar.open);
    tap(&mut state, shell::StatusAction::Wifi);
    assert!(state.shell.quick_settings_open);
    assert_eq!(state.shell.connection_page, Some(QuickAction::Wifi));
    tap(&mut state, shell::StatusAction::Bluetooth);
    assert_eq!(state.shell.connection_page, Some(QuickAction::Bluetooth));
    tap(&mut state, shell::StatusAction::Notifications);
    assert!(state.shell.notifications_open && state.shell.connection_page.is_none());
    tap(&mut state, shell::StatusAction::Shade);
    assert!(state.shell.quick_settings_open && !state.shell.notifications_open);
    assert!(shell::quick_settings_rect(1280.0, 800.0).y >= shell::top_inset());
}
