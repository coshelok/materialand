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
