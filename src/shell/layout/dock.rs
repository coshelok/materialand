//! Dock geometry.
use super::*;
pub fn dock_rect(win_w: f32, win_h: f32, items: usize) -> RectF {
    let (icon, gap) = dock_metrics(win_w, items);
    let w = items as f32 * icon + items.saturating_sub(1) as f32 * gap + 2.0 * DOCK_PAD;
    RectF {
        x: (win_w - w) / 2.0,
        y: win_h - config::DOCK_HEIGHT - config::DOCK_MARGIN,
        w,
        h: config::DOCK_HEIGHT,
    }
}

/// Screen rectangle of one dock icon; the launcher grows out of this.
pub fn dock_item_rect(win_w: f32, win_h: f32, items: usize, index: usize) -> RectF {
    let rect = dock_rect(win_w, win_h, items);
    let (icon, gap) = dock_metrics(win_w, items);
    let x = rect.x + DOCK_PAD + index as f32 * (icon + gap);
    RectF {
        x,
        y: rect.y + (rect.h - icon) / 2.0,
        w: icon,
        h: icon,
    }
}

/// Compress the row as a unit on small outputs or with many running windows.
fn dock_metrics(win_w: f32, items: usize) -> (f32, f32) {
    let total =
        items as f32 * config::DOCK_ICON + items.saturating_sub(1) as f32 * config::DOCK_GAP;
    let scale =
        ((win_w - 2.0 * config::DOCK_MARGIN - 2.0 * DOCK_PAD).max(1.0) / total.max(1.0)).min(1.0);
    (config::DOCK_ICON * scale, config::DOCK_GAP * scale)
}

pub fn dock_hit(win_w: f32, win_h: f32, items: usize, pos: Point<f64, Logical>) -> Option<usize> {
    (0..items).find(|&i| dock_item_rect(win_w, win_h, items, i).contains(pos))
}
