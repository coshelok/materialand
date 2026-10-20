//! Panels geometry.
use super::*;
pub fn brightness_popup(win_w: f32) -> RectF {
    let design = crate::config::design();
    let rect = topbar_rect(win_w);
    let w = design
        .brightness
        .width
        .min((rect.w - 2.0 * design.panels.edge_inset).max(1.0));
    let h = design.brightness.height;
    RectF {
        x: rect.right() - design.panels.edge_inset - w,
        y: rect.bottom() + design.panels.edge_inset,
        w,
        h,
    }
}

/// Card of the session menu, anchored under the top-bar user button.
pub fn power_menu_rect(win_w: f32) -> RectF {
    let design = crate::config::design();
    let bar = topbar_rect(win_w);
    let w = design
        .power
        .width
        .min((bar.w - 2.0 * design.panels.edge_inset).max(1.0));
    let rows = POWER_ACTIONS.len() as f32;
    let h = 2.0 * design.power.padding
        + rows * design.power.row_height
        + (rows - 1.0) * design.power.row_gap;
    RectF {
        x: bar.right() - design.panels.edge_inset - w,
        y: bar.bottom() + design.panels.edge_inset,
        w,
        h,
    }
}

/// Screen rectangle of one session-menu row.
pub fn power_row_rect(win_w: f32, index: usize) -> RectF {
    let design = crate::config::design();
    let card = power_menu_rect(win_w);
    RectF {
        x: card.x + design.power.padding,
        y: card.y
            + design.power.padding
            + index as f32 * (design.power.row_height + design.power.row_gap),
        w: card.w - 2.0 * design.power.padding,
        h: design.power.row_height,
    }
}

/// Index of the session-menu row under `pos`, if any.
pub fn power_row_at(win_w: f32, pos: Point<f64, Logical>) -> Option<usize> {
    (0..POWER_ACTIONS.len()).find(|i| power_row_rect(win_w, *i).contains(pos))
}

pub fn quick_settings_rect(win_w: f32, win_h: f32) -> RectF {
    let design = crate::config::design();
    if tablet_mode() {
        let d = &design.shade;
        if d.fullscreen {
            return RectF {
                x: 0.0,
                y: top_inset(),
                w: win_w.max(1.0),
                h: (win_h - top_inset() - bottom_inset()).max(1.0),
            };
        }
        let w = d.width.min((win_w - 2.0 * d.margin).max(1.0));
        let h = d
            .height
            .min((win_h - top_inset() - 2.0 * d.margin - bottom_inset()).max(1.0));
        return RectF {
            x: (win_w - w) / 2.0,
            y: top_inset() + d.margin,
            w,
            h,
        };
    }
    let bar = topbar_rect(win_w);
    let w = (bar.w - 2.0 * design.panels.edge_inset).clamp(1.0, design.quick_settings.width);
    let h = design.quick_settings.height.min(
        (win_h - bar.bottom() - design.panels.bottom_gap).max(design.quick_settings.min_height),
    );
    RectF {
        x: (bar.right() - design.panels.edge_inset - w).max(design.panels.viewport_margin),
        y: (bar.bottom() + design.panels.edge_inset)
            .min((win_h - h - design.panels.viewport_margin).max(design.panels.viewport_margin)),
        w,
        h,
    }
}

pub fn quick_setting_tile(rect: RectF, index: usize, progress: f32) -> RectF {
    let design = crate::config::design();
    let _ = progress;
    let columns = quick_columns(rect.w);
    let tile_w = ((rect.w
        - 2.0 * design.quick_settings.padding
        - (columns - 1) as f32 * design.quick_settings.tile_gap)
        / columns as f32)
        .max(1.0);
    let index = if tablet_mode() {
        match index {
            4 => 0,
            5 => 1,
            0 => 2,
            1 => 3,
            2 => 4,
            _ => 5,
        }
    } else {
        index
    };
    let tile_h = design.quick_settings.tile_height;
    let x = rect.x
        + design.quick_settings.padding
        + (index % columns) as f32 * (tile_w + design.quick_settings.tile_gap);
    let y = rect.y
        + quick_tiles_top()
        + (index / columns) as f32 * (tile_h + design.quick_settings.tile_gap);
    RectF {
        x,
        y,
        w: tile_w,
        h: tile_h,
    }
}

/// Screen rectangle of one quick-settings tile at rest.
pub fn quick_setting_tile_rect(win_w: f32, win_h: f32, index: usize) -> RectF {
    quick_setting_tile(quick_settings_rect(win_w, win_h), index, 1.0)
}

pub fn quick_settings_hit(
    win_w: f32,
    win_h: f32,
    pos: Point<f64, Logical>,
    progress: f32,
) -> Option<QuickAction> {
    let rect = quick_settings_rect(win_w, win_h);
    let actions = [
        QuickAction::Theme,
        QuickAction::NightLight,
        QuickAction::ReduceMotion,
        QuickAction::Brightness,
        QuickAction::Wifi,
        QuickAction::Bluetooth,
    ];
    actions.iter().enumerate().find_map(|(i, action)| {
        quick_setting_tile(rect, i, progress)
            .contains(pos)
            .then_some(*action)
    })
}

/// Palette chip rectangle inside the sheet.
pub fn palette_chip_rect_in(rect: RectF, index: usize) -> RectF {
    let design = crate::config::design();
    let cw =
        (rect.w - 2.0 * design.quick_settings.chip_margin - 2.0 * design.quick_settings.chip_gap)
            / 3.0;
    let col = index % 3;
    let row = index / 3;
    RectF {
        x: rect.x
            + design.quick_settings.chip_margin
            + col as f32 * (cw + design.quick_settings.chip_gap),
        y: rect.y
            + palette_top(rect.w)
            + row as f32 * (design.quick_settings.chip_height + design.quick_settings.chip_gap),
        w: cw,
        h: design.quick_settings.chip_height,
    }
}

/// Palette chip rectangle in screen space, or `None` when the sheet is too
/// short for this row (clamped height on small outputs).
pub fn palette_chip_rect(win_w: f32, win_h: f32, index: usize) -> Option<RectF> {
    let rect = quick_settings_rect(win_w, win_h);
    let chip = palette_chip_rect_in(rect, index);
    (chip.bottom() <= rect.bottom() - crate::config::design().quick_settings.chip_bottom_gap)
        .then_some(chip)
}

pub fn palette_chip_hit(win_w: f32, win_h: f32, pos: Point<f64, Logical>) -> Option<usize> {
    (0..VARIANTS.len())
        .find(|&i| palette_chip_rect(win_w, win_h, i).is_some_and(|rect| rect.contains(pos)))
}

/// Draw a Material 3 switch whose handle is driven by a spring (`t` may
pub fn brightness_track(win_w: f32) -> RectF {
    let design = crate::config::design();
    let card = brightness_popup(win_w);
    let left = card.x + design.brightness.track_left;
    let right = card.right() - design.brightness.track_right;
    RectF {
        x: left,
        y: card.y,
        w: (right - left).max(1.0),
        h: card.h,
    }
}

pub fn brightness_hit(win_w: f32, pos: Point<f64, Logical>) -> Option<f32> {
    let card = brightness_popup(win_w);
    if !card.contains(pos) {
        return None;
    }
    let track = brightness_track(win_w);
    let t = ((pos.x as f32 - track.x) / track.w).clamp(0.0, 1.0);
    Some(BRIGHTNESS_MIN + t * (BRIGHTNESS_MAX - BRIGHTNESS_MIN))
}

pub fn quick_tiles_top() -> f32 {
    let d = config::design();
    if tablet_mode() {
        d.shade
            .header_height
            .max(d.shade.brightness_y + d.shade.brightness_height + d.quick_settings.tile_gap)
    } else {
        d.quick_settings.tile_top
    }
}
pub fn quick_columns(width: f32) -> usize {
    let d = &config::design().shade;
    if tablet_mode() && width >= d.wide_threshold {
        d.wide_columns
    } else {
        2
    }
}
pub fn palette_top(width: f32) -> f32 {
    let d = &config::design().quick_settings;
    quick_tiles_top()
        + 6usize.div_ceil(quick_columns(width)) as f32 * (d.tile_height + d.tile_gap)
        + d.palette_title_gap
}
pub fn shade_scroll_max(w: f32, h: f32) -> f32 {
    let d = &config::design().quick_settings;
    let card = quick_settings_rect(w, h);
    (palette_top(card.w) + 3.0 * (d.chip_height + d.chip_gap) + d.padding - card.h).max(0.0)
}
pub fn shade_header_hit(w: f32, h: f32, pos: Point<f64, Logical>) -> Option<TopAction> {
    let r = quick_settings_rect(w, h);
    if !r.contains(pos)
        || pos.y as f32 > r.y + (2.0 * config::design().statusbar.height).min(quick_tiles_top())
    {
        return None;
    }
    if pos.x as f32 > r.right() - 56.0 {
        Some(TopAction::Power)
    } else if pos.x as f32 > r.right() - 112.0 {
        Some(TopAction::Clock)
    } else {
        None
    }
}
