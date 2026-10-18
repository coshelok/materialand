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
        y: card.y + 20.0,
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
    (card.x + GRID_PAD, search.bottom() + 16.0)
}

pub fn launcher_tile_pos(origin: (f32, f32), slot: usize, cols: usize) -> (f32, f32) {
    let col = slot % cols;
    let row = slot / cols;
    let x = origin.0 + col as f32 * (config::LAUNCHER_TILE + GRID_GAP);
    let y = origin.1 + row as f32 * (config::LAUNCHER_TILE + GRID_GAP);
    (x, y)
}

pub fn launcher_hit(
    win_w: f32,
    win_h: f32,
    filtered_count: usize,
    pos: Point<f64, Logical>,
) -> LauncherHit {
    let card = launcher_card(win_w, win_h);
    if !card.contains(pos) {
        return LauncherHit::Scrim;
    }
    if search_rect(win_w, win_h).contains(pos) {
        return LauncherHit::Input;
    }
    let cols = launcher_columns(card.w);
    let origin = launcher_grid_origin(win_w, win_h);
    for slot in 0..filtered_count {
        let (tx, ty) = launcher_tile_pos(origin, slot, cols);
        // Same cutoff as `render_launcher`: only tiles that fit the card exist.
        if ty + config::LAUNCHER_TILE > card.bottom() - 16.0 {
            break;
        }
        let tile = RectF {
            x: tx,
            y: ty,
            w: config::LAUNCHER_TILE,
            h: config::LAUNCHER_TILE,
        };
        if tile.contains(pos) {
            return LauncherHit::Tile(slot);
        }
    }
    LauncherHit::Inside
}
