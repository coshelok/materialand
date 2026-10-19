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
    let tile_w = ((rect.w - 2.0 * design.quick_settings.padding - design.quick_settings.tile_gap)
        / 2.0)
        .max(1.0);
    let tile_h = design.quick_settings.tile_height;
    let x = rect.x
        + if index.is_multiple_of(2) {
            design.quick_settings.padding
        } else {
            design.quick_settings.padding + tile_w + design.quick_settings.tile_gap
        };
    let y = rect.y
        + design.quick_settings.tile_top
        + if index < 2 {
            0.0
        } else {
            tile_h + design.quick_settings.tile_gap
        };
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
            + design.quick_settings.chips_y
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
