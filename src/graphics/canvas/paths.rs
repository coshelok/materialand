//! Vector paths.
use super::*;
pub(super) fn rect(x: f32, y: f32, w: f32, h: f32) -> tiny_skia::Rect {
    tiny_skia::Rect::from_xywh(x, y, w, h)
        .unwrap_or_else(|| tiny_skia::Rect::from_xywh(0.0, 0.0, 1.0, 1.0).unwrap())
}

/// Build a closed polygon path from a list of points.
pub fn polygon_path(points: &[(f32, f32)]) -> Path {
    let mut pb = PathBuilder::new();
    if let Some((&(x0, y0), rest)) = points.split_first() {
        pb.move_to(x0, y0);
        for &(x, y) in rest {
            pb.line_to(x, y);
        }
        pb.close();
    } else {
        pb.move_to(0.0, 0.0);
    }
    pb.finish().unwrap_or_else(|| {
        let mut p = PathBuilder::new();
        p.move_to(0.0, 0.0);
        p.finish().unwrap()
    })
}

pub fn round_rect_path(x: f32, y: f32, w: f32, h: f32, radius: f32) -> Path {
    let r = radius.min(w / 2.0).min(h / 2.0).max(0.0);
    let mut pb = PathBuilder::new();
    if r <= 0.01 {
        pb.push_rect(rect(x, y, w, h));
        return pb.finish().unwrap();
    }
    pb.move_to(x + r, y);
    pb.line_to(x + w - r, y);
    pb.quad_to(x + w, y, x + w, y + r);
    pb.line_to(x + w, y + h - r);
    pb.quad_to(x + w, y + h, x + w - r, y + h);
    pb.line_to(x + r, y + h);
    pb.quad_to(x, y + h, x, y + h - r);
    pb.line_to(x, y + r);
    pb.quad_to(x, y, x + r, y);
    pb.close();
    pb.finish().unwrap()
}
