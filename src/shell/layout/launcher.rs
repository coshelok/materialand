//! Launcher geometry.
use super::*;
pub fn launcher_card(win_w: f32, win_h: f32) -> RectF {
    let w = config::LAUNCHER_WIDTH.min(win_w - 96.0).max(320.0);
    let h = config::LAUNCHER_MAX_HEIGHT.min(win_h - 120.0).max(240.0);
    RectF {
        x: (win_w - w) / 2.0,
        y: (win_h - h) / 2.0 - 24.0,
        w,
        h,
    }
}

pub fn search_rect(win_w: f32, win_h: f32) -> RectF {
    let card = launcher_card(win_w, win_h);
    RectF {
        x: card.x + 20.0,
        y: card.y + 32.0,
        w: card.w - 40.0,
        h: SEARCH_HEIGHT,
    }
}

pub fn launcher_columns(card_w: f32) -> usize {
    let usable = card_w - 2.0 * GRID_PAD + GRID_GAP;
    ((usable / (config::LAUNCHER_TILE + GRID_GAP)).floor() as usize).max(1)
}

pub fn launcher_grid_origin(win_w: f32, win_h: f32) -> (f32, f32) {
    let card = launcher_card(win_w, win_h);
    let search = search_rect(win_w, win_h);
    let cols = launcher_columns(card.w);
    let grid_w = cols as f32 * config::LAUNCHER_TILE + cols.saturating_sub(1) as f32 * GRID_GAP;
    (card.x + (card.w - grid_w) / 2.0, search.bottom() + 48.0)
}

pub fn launcher_tile_pos(origin: (f32, f32), slot: usize, cols: usize) -> (f32, f32) {
    let col = slot % cols;
    let row = slot / cols;
    let x = origin.0 + col as f32 * (config::LAUNCHER_TILE + GRID_GAP);
    let y = origin.1 + row as f32 * (config::LAUNCHER_TILE + GRID_GAP);
    (x, y)
}

/// Only the grid scrolls. Search and section labels stay fixed.
pub fn launcher_viewport(w: f32, h: f32) -> RectF {
    let card = launcher_card(w, h);
    let (_, y) = launcher_grid_origin(w, h);
    RectF {
        x: card.x + GRID_PAD,
        y,
        w: card.w - 2.0 * GRID_PAD,
        h: (card.bottom() - 16.0 - y).max(1.0),
    }
}
pub fn launcher_scroll_max(w: f32, h: f32, count: usize) -> f32 {
    let rows = count.div_ceil(launcher_columns(launcher_card(w, h).w));
    let content = rows as f32 * config::LAUNCHER_TILE + rows.saturating_sub(1) as f32 * GRID_GAP;
    (content - launcher_viewport(w, h).h).max(0.0)
}
pub fn launcher_hit(
    w: f32,
    h: f32,
    count: usize,
    scroll: f32,
    pos: Point<f64, Logical>,
) -> LauncherHit {
    let card = launcher_card(w, h);
    if !card.contains(pos) {
        return LauncherHit::Scrim;
    }
    if search_rect(w, h).contains(pos) {
        return LauncherHit::Input;
    }
    let viewport = launcher_viewport(w, h);
    if !viewport.contains(pos) {
        return LauncherHit::Inside;
    }
    let cols = launcher_columns(card.w);
    let mut origin = launcher_grid_origin(w, h);
    origin.1 -= scroll;
    for slot in 0..count {
        let (x, y) = launcher_tile_pos(origin, slot, cols);
        if y > viewport.bottom() {
            break;
        }
        if (RectF {
            x,
            y,
            w: config::LAUNCHER_TILE,
            h: config::LAUNCHER_TILE,
        })
        .contains(pos)
        {
            return LauncherHit::Tile(slot);
        }
    }
    LauncherHit::Inside
}
