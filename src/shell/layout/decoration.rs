//! Decoration geometry.
use super::*;
pub fn decor_layout(window: Rectangle<i32, Logical>) -> DecorLayout {
    let design = crate::config::design();
    let content = RectF {
        x: window.loc.x as f32,
        y: window.loc.y as f32,
        w: window.size.w as f32,
        h: window.size.h as f32,
    };
    let frame = RectF {
        x: content.x - design.window.border,
        y: content.y - design.window.titlebar_height,
        w: content.w + 2.0 * design.window.border,
        h: content.h + design.window.titlebar_height + design.window.border,
    };
    let titlebar = RectF {
        x: frame.x,
        y: frame.y,
        w: frame.w,
        h: design.window.titlebar_height,
    };
    let btn_r = design.window.button_radius;
    let btn_gap = design.window.button_gap;
    let cy = frame.y + design.window.titlebar_height / 2.0;
    let close_x = frame.right() - btn_r - design.window.button_margin;
    let maximize = (close_x - btn_gap, cy);
    let minimize = (close_x - 2.0 * btn_gap, cy);
    let close = (close_x, cy);
    DecorLayout {
        frame,
        content,
        titlebar,
        minimize,
        maximize,
        close,
    }
}

// ---- Hit testing --------------------------------------------------------

pub fn decor_hit(l: &DecorLayout, pos: Point<f64, Logical>) -> DecorHit {
    let design = crate::config::design();
    let (px, py) = (pos.x as f32, pos.y as f32);
    let near = |c: (f32, f32)| {
        let (dx, dy) = (px - c.0, py - c.1);
        dx * dx + dy * dy <= design.window.button_hit_radius.powi(2)
    };
    if near(l.close) {
        return DecorHit::Close;
    }
    if near(l.maximize) {
        return DecorHit::Maximize;
    }
    if near(l.minimize) {
        return DecorHit::Minimize;
    }
    // Outer resize bands take precedence over the title bar so every edge is
    // grabbable, including the top one (which otherwise reads as a move).
    let band = design.window.resize_band;
    if l.frame.contains(pos) {
        let mut e = ResizeEdges::default();
        if px <= l.frame.x + band {
            e.left = true;
        }
        if px >= l.frame.right() - band {
            e.right = true;
        }
        if py <= l.frame.y + band {
            e.top = true;
        }
        if py >= l.frame.bottom() - band {
            e.bottom = true;
        }
        if e != ResizeEdges::default() {
            return DecorHit::Resize(e);
        }
    }
    if l.titlebar.contains(pos) && !l.content.contains(pos) {
        return DecorHit::Move;
    }
    DecorHit::None
}
