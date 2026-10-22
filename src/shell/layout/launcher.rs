//! Launcher geometry.
use super::*;
pub fn launcher_card(win_w: f32, win_h: f32) -> RectF {
    let design = crate::config::design();
    if tablet_mode() && design.launcher.fullscreen {
        return RectF {
            x: 0.0,
            y: top_inset(),
            w: win_w.max(1.0),
            h: (win_h - top_inset() - bottom_inset()).max(1.0),
        };
    }
    let available_w = (win_w - 2.0 * design.launcher.margin_x).max(1.0);
    let top = if tablet_mode() { top_inset() } else { 0.0 };
    let bottom = if tablet_mode() { bottom_inset() } else { 0.0 };
    let area_h = (win_h - top - bottom).max(1.0);
    let available_h = (area_h - 2.0 * design.launcher.margin_y).max(1.0);
    let w = design
        .launcher
        .width
        .min(available_w)
        .max(design.launcher.min_width.min(available_w));
    let h = design
        .launcher
        .max_height
        .min(available_h)
        .max(design.launcher.min_height.min(available_h));
    RectF {
        x: (win_w - w) / 2.0,
        y: (top + (area_h - h) / 2.0 + design.launcher.offset_y)
            .clamp(top, top + (area_h - h).max(0.0)),
        w,
        h,
    }
}

pub fn search_rect(win_w: f32, win_h: f32) -> RectF {
    let design = crate::config::design();
    let card = launcher_card(win_w, win_h);
    let width = (card.w - 2.0 * design.launcher.search_inset_x).max(1.0);
    RectF {
        x: card.x + (card.w - width) / 2.0,
        y: card.y + design.launcher.search_inset_y,
        w: width,
        h: design.launcher.search_height,
    }
}

pub fn launcher_columns(card_w: f32) -> usize {
    let design = crate::config::design();
    let usable = card_w - 2.0 * design.launcher.grid_padding + design.launcher.grid_gap;
    ((usable / (design.launcher.tile_size + design.launcher.grid_gap)).floor() as usize)
        .max(1)
        .min(design.launcher.max_columns)
}

pub fn launcher_grid_origin(win_w: f32, win_h: f32) -> (f32, f32, f32) {
    let d = config::design();
    let card = launcher_card(win_w, win_h);
    let cols = launcher_columns(card.w);
    let pitch = if tablet_mode() && d.launcher.fullscreen {
        (card.w - 2.0 * d.launcher.grid_padding).max(1.0) / cols as f32
    } else {
        d.launcher.tile_size + d.launcher.grid_gap
    };
    let width = cols as f32 * pitch;
    (
        card.x + (card.w - width) / 2.0 + (pitch - d.launcher.tile_size) / 2.0,
        search_rect(win_w, win_h).bottom() + d.launcher.grid_header_height,
        pitch,
    )
}
pub fn launcher_tile_pos(origin: (f32, f32, f32), slot: usize, cols: usize) -> (f32, f32) {
    let d = config::design();
    (
        origin.0 + (slot % cols) as f32 * origin.2,
        origin.1 + (slot / cols) as f32 * (d.launcher.tile_size + d.launcher.grid_gap),
    )
}

/// Only the grid scrolls. Search and section labels stay fixed.
pub fn launcher_viewport(w: f32, h: f32) -> RectF {
    let design = crate::config::design();
    let card = launcher_card(w, h);
    let (_, y, _) = launcher_grid_origin(w, h);
    RectF {
        x: card.x + design.launcher.grid_padding,
        y,
        w: card.w - 2.0 * design.launcher.grid_padding,
        h: (card.bottom() - design.launcher.bottom_padding - y).max(1.0),
    }
}
pub fn launcher_scroll_max(w: f32, h: f32, count: usize) -> f32 {
    let design = crate::config::design();
    let rows = count.div_ceil(launcher_columns(launcher_card(w, h).w));
    let content = rows as f32 * design.launcher.tile_size
        + rows.saturating_sub(1) as f32 * design.launcher.grid_gap;
    (content - launcher_viewport(w, h).h).max(0.0)
}
pub fn launcher_hit(
    w: f32,
    h: f32,
    count: usize,
    scroll: f32,
    pos: Point<f64, Logical>,
) -> LauncherHit {
    let design = crate::config::design();
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
            w: design.launcher.tile_size,
            h: design.launcher.tile_size,
        })
        .contains(pos)
        {
            return LauncherHit::Tile(slot);
        }
    }
    LauncherHit::Inside
}

/// The same medallion bounds used to draw tiles and start window entrances.
pub fn app_icon_rect(tile: RectF) -> RectF {
    let app = &crate::config::design().apps;
    let size = 2.0 * app.medallion_radius;
    RectF {
        x: tile.x + (tile.w - size) / 2.0,
        y: tile.y + app.icon_center_y - app.medallion_radius,
        w: size,
        h: size,
    }
}
