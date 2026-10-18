//! Decoration geometry.
use super::*;
pub fn decor_layout(window: Rectangle<i32, Logical>) -> DecorLayout {
    let content = RectF {
        x: window.loc.x as f32,
        y: window.loc.y as f32,
        w: window.size.w as f32,
        h: window.size.h as f32,
    };
    let frame = RectF {
        x: content.x - config::FRAME_BORDER,
        y: content.y - config::FRAME_TITLEBAR,
        w: content.w + 2.0 * config::FRAME_BORDER,
        h: content.h + config::FRAME_TITLEBAR + config::FRAME_BORDER,
    };
    let titlebar = RectF {
        x: frame.x,
        y: frame.y,
        w: frame.w,
        h: config::FRAME_TITLEBAR,
    };
    let btn_r = 15.0;
    let btn_gap = 36.0;
    let cy = frame.y + config::FRAME_TITLEBAR / 2.0;
    let close_x = frame.right() - btn_r - 8.0;
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
    let (px, py) = (pos.x as f32, pos.y as f32);
    let near = |c: (f32, f32)| {
        let (dx, dy) = (px - c.0, py - c.1);
        dx * dx + dy * dy <= 16.0 * 16.0
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
    let band = 6.0;
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
