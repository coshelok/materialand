//! Panels geometry.
use super::*;
pub fn brightness_popup(win_w: f32) -> RectF {
    let rect = topbar_rect(win_w);
    let w = 260.0_f32.min((rect.w - 16.0).max(0.0)).max(160.0);
    let h = 64.0;
    RectF {
        x: rect.right() - 8.0 - w,
        y: rect.bottom() + 8.0,
        w,
        h,
    }
}

/// Height of one session-menu row.
pub const POWER_ROW_H: f32 = 56.0;
/// Gap between session-menu rows.
pub const POWER_ROW_GAP: f32 = 4.0;
/// Padding between the session-menu card edge and its rows.
pub const POWER_MENU_PAD: f32 = 8.0;

/// Card of the session menu, anchored under the top-bar user button.
pub fn power_menu_rect(win_w: f32) -> RectF {
    let bar = topbar_rect(win_w);
    let w = 232.0_f32.min((bar.w - 16.0).max(0.0)).max(180.0);
    let rows = POWER_ACTIONS.len() as f32;
    let h = 2.0 * POWER_MENU_PAD + rows * POWER_ROW_H + (rows - 1.0) * POWER_ROW_GAP;
    RectF {
        x: bar.right() - 8.0 - w,
        y: bar.bottom() + 8.0,
        w,
        h,
    }
}

/// Screen rectangle of one session-menu row.
pub fn power_row_rect(win_w: f32, index: usize) -> RectF {
    let card = power_menu_rect(win_w);
    RectF {
        x: card.x + POWER_MENU_PAD,
        y: card.y + POWER_MENU_PAD + index as f32 * (POWER_ROW_H + POWER_ROW_GAP),
        w: card.w - 2.0 * POWER_MENU_PAD,
        h: POWER_ROW_H,
    }
}

/// Index of the session-menu row under `pos`, if any.
pub fn power_row_at(win_w: f32, pos: Point<f64, Logical>) -> Option<usize> {
    (0..POWER_ACTIONS.len()).find(|i| power_row_rect(win_w, *i).contains(pos))
}

pub fn quick_settings_rect(win_w: f32, win_h: f32) -> RectF {
    let bar = topbar_rect(win_w);
    let w = (win_w - 24.0).clamp(160.0, 376.0);
    let h = QS_SHEET_HEIGHT.min((win_h - bar.bottom() - 20.0).max(120.0));
    RectF {
        x: (bar.right() - w).max(12.0),
        y: (bar.bottom() + 8.0).min((win_h - h - 12.0).max(12.0)),
        w,
        h,
    }
}

pub fn quick_setting_tile(rect: RectF, index: usize, progress: f32) -> RectF {
    let progress = progress.clamp(0.0, 1.0);
    let dy = -(1.0 - progress) * 14.0;
    let tile_w = ((rect.w - 44.0) / 2.0).max(56.0);
    let tile_h = 72.0;
    let x = rect.x
        + if index.is_multiple_of(2) {
            16.0
        } else {
            16.0 + tile_w + 12.0
        };
    let y = rect.y + 48.0 + dy + if index < 2 { 0.0 } else { tile_h + 12.0 };
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
    let cw = (rect.w - 2.0 * QS_CHIP_MARGIN - 2.0 * QS_CHIP_GAP) / 3.0;
    let col = index % 3;
    let row = index / 3;
    RectF {
        x: rect.x + QS_CHIP_MARGIN + col as f32 * (cw + QS_CHIP_GAP),
        y: rect.y + QS_CHIPS_Y + row as f32 * (QS_CHIP_H + QS_CHIP_GAP),
        w: cw,
        h: QS_CHIP_H,
    }
}

/// Palette chip rectangle in screen space, or `None` when the sheet is too
/// short for this row (clamped height on small outputs).
pub fn palette_chip_rect(win_w: f32, win_h: f32, index: usize) -> Option<RectF> {
    let rect = quick_settings_rect(win_w, win_h);
    let chip = palette_chip_rect_in(rect, index);
    (chip.bottom() <= rect.bottom() - 6.0).then_some(chip)
}

pub fn palette_chip_hit(win_w: f32, win_h: f32, pos: Point<f64, Logical>) -> Option<usize> {
    (0..VARIANTS.len())
        .find(|&i| palette_chip_rect(win_w, win_h, i).is_some_and(|rect| rect.contains(pos)))
}

/// Draw a Material 3 switch whose handle is driven by a spring (`t` may
pub fn brightness_track(win_w: f32) -> RectF {
    let card = brightness_popup(win_w);
    let left = card.x + 56.0;
    let right = card.right() - 20.0;
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
