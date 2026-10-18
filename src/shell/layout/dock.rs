//! Dock geometry.
use super::*;
pub fn dock_rect(win_w: f32, win_h: f32, _items: usize) -> RectF {
    RectF {
        x: 0.0,
        y: win_h - config::DOCK_HEIGHT,
        w: win_w,
        h: config::DOCK_HEIGHT,
    }
}

/// Screen rectangle of one dock icon; the launcher grows out of this.
pub fn dock_item_rect(win_w: f32, win_h: f32, items: usize, index: usize) -> RectF {
    let rect = dock_rect(win_w, win_h, items);
    let x = dock_row_start(rect.w, items) + index as f32 * (config::DOCK_ICON + config::DOCK_GAP);
    RectF {
        x,
        y: rect.y + (rect.h - config::DOCK_ICON) / 2.0,
        w: config::DOCK_ICON,
        h: config::DOCK_ICON,
    }
}

/// Left edge of the centred dock icon row.
pub fn dock_row_start(win_w: f32, items: usize) -> f32 {
    let total =
        items as f32 * config::DOCK_ICON + items.saturating_sub(1) as f32 * config::DOCK_GAP;
    ((win_w - total) / 2.0).max(DOCK_PAD)
}

pub fn dock_hit(win_w: f32, win_h: f32, items: usize, pos: Point<f64, Logical>) -> Option<usize> {
    let rect = dock_rect(win_w, win_h, items);
    if !rect.contains(pos) {
        return None;
    }
    let (px, _) = (pos.x as f32, pos.y as f32);
    let mut x = dock_row_start(rect.w, items);
    for i in 0..items {
        if px >= x && px <= x + config::DOCK_ICON {
            return Some(i);
        }
        x += config::DOCK_ICON + config::DOCK_GAP;
    }
    None
}
