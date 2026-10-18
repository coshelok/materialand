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
