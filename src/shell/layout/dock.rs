//! Dock geometry.
use super::*;
pub fn dock_rect(win_w: f32, win_h: f32, items: usize) -> RectF {
    let design = crate::config::design();
    let (icon, gap) = dock_metrics(win_w, items);
    let w = items as f32 * icon + items.saturating_sub(1) as f32 * gap + 2.0 * design.dock.padding;
    RectF {
        x: (win_w - w) / 2.0,
        y: win_h - design.dock.height - design.dock.margin,
        w,
        h: design.dock.height,
    }
}

/// Screen rectangle of one dock icon; the launcher grows out of this.
pub fn dock_item_rect(win_w: f32, win_h: f32, items: usize, index: usize) -> RectF {
    let rect = dock_rect(win_w, win_h, items);
    let (icon, gap) = dock_metrics(win_w, items);
    let x = rect.x + crate::config::design().dock.padding + index as f32 * (icon + gap);
    RectF {
        x,
        y: rect.y + (rect.h - icon) / 2.0,
        w: icon,
        h: icon,
    }
}

/// Compress the row as a unit on small outputs or with many running windows.
fn dock_metrics(win_w: f32, items: usize) -> (f32, f32) {
    let design = crate::config::design();
    let total =
        items as f32 * design.dock.icon_size + items.saturating_sub(1) as f32 * design.dock.gap;
    let scale = ((win_w - 2.0 * design.dock.margin - 2.0 * design.dock.padding).max(1.0)
        / total.max(1.0))
    .min(1.0);
    (design.dock.icon_size * scale, design.dock.gap * scale)
}

pub fn dock_hit(win_w: f32, win_h: f32, items: usize, pos: Point<f64, Logical>) -> Option<usize> {
    (0..items).find(|&i| dock_item_rect(win_w, win_h, items, i).contains(pos))
}
