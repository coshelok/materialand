use super::*;
use crate::graphics::canvas::Surface;

fn ink_rows(s: &Surface) -> Option<(u32, u32)> {
    let (mut top, mut bottom, mut any) = (u32::MAX, 0u32, false);
    for y in 0..s.pixmap.height() {
        for x in 0..s.width() {
            if s.pixmap.pixel(x, y).unwrap().alpha() > 20 {
                top = top.min(y);
                bottom = bottom.max(y);
                any = true;
            }
        }
    }
    any.then_some((top, bottom))
}

fn ink_bounds(s: &Surface) -> Option<(u32, u32, u32, u32)> {
    let (mut x0, mut y0, mut x1, mut y1, mut any) = (u32::MAX, u32::MAX, 0u32, 0u32, false);
    for y in 0..s.pixmap.height() {
        for x in 0..s.width() {
            if s.pixmap.pixel(x, y).unwrap().alpha() > 20 {
                x0 = x0.min(x);
                y0 = y0.min(y);
                x1 = x1.max(x);
                y1 = y1.max(y);
                any = true;
            }
        }
    }
    any.then_some((x0, y0, x1, y1))
}

#[test]
fn glyph_sits_above_baseline() {
    let mut f = Fonts::load().expect("test system font");
    let mut s = Surface::new(140, 96);
    let (px, y) = (48.0, 20.0);
    f.draw_text_styled_at(
        &mut s,
        10.0,
        y,
        FontKind::Bold,
        "H",
        px,
        0.0,
        tiny_skia::Color::from_rgba8(255, 255, 255, 255),
    );
    let baseline = (y + f.ascent(FontKind::Bold, px)).round() as i32;
    let (top, bottom) = ink_rows(&s).expect("glyph produced ink");
    assert!(
        (bottom as i32) < baseline + 2,
        "bottom {bottom} must sit on the baseline {baseline}"
    );
    assert!(
        (top as i32) < baseline,
        "glyph must rise above the baseline"
    );
    assert!(
        ((top + bottom) as i32) / 2 < baseline,
        "glyph center must be above the baseline"
    );
}

#[test]
fn icon_names_resolve() {
    let f = Fonts::load().expect("test system font");
    for name in ["terminal", "public", "settings", "folder", "apps"] {
        assert!(f.icon(name).is_some(), "missing icon: {name}");
    }
}

#[test]
fn centered_icon_ink_is_centered() {
    let mut f = Fonts::load().expect("test system font");
    let (cx, cy) = (60.0, 40.0);
    let mut s = Surface::new(120, 80);
    f.draw_icon_centered(
        &mut s,
        cx,
        cy,
        "apps",
        32.0,
        tiny_skia::Color::from_rgba8(255, 255, 255, 255),
    );
    let (x0, y0, x1, y1) = ink_bounds(&s).expect("icon produced ink");
    let mid_x = (x0 + x1) as f32 / 2.0;
    let mid_y = (y0 + y1) as f32 / 2.0;
    assert!((mid_x - cx).abs() <= 2.0, "x center {mid_x} != {cx}");
    assert!((mid_y - cy).abs() <= 2.0, "y center {mid_y} != {cy}");
}
