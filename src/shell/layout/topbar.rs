//! Topbar geometry.
use super::*;
pub fn topbar_rect(win_w: f32) -> RectF {
    RectF {
        x: 0.0,
        y: 0.0,
        w: win_w,
        h: config::TOPBAR_HEIGHT,
    }
}

/// Screen rectangle of one top-bar icon button; render and hit-testing share
/// this layout so a press always lands on the drawn control.
pub fn topbar_button_rect(win_w: f32, action: TopAction) -> RectF {
    let rect = topbar_rect(win_w);
    let btn = config::TOPBAR_HEIGHT;
    let x = match action {
        TopAction::Launcher => rect.x,
        TopAction::Power => rect.right() - 8.0 - btn,
        TopAction::Theme => rect.right() - 8.0 - 2.0 * btn - TOPBAR_ICON_GAP,
        TopAction::Brightness => rect.right() - 8.0 - 3.0 * btn - 2.0 * TOPBAR_ICON_GAP,
        TopAction::QuickSettings => rect.right() - 8.0 - 4.0 * btn - 3.0 * TOPBAR_ICON_GAP,
    };
    RectF {
        x,
        y: rect.y,
        w: btn,
        h: btn,
    }
}

pub fn topbar_hit(win_w: f32, pos: Point<f64, Logical>) -> Option<TopAction> {
    let rect = topbar_rect(win_w);
    if !rect.contains(pos) {
        return None;
    }
    [
        TopAction::Launcher,
        TopAction::QuickSettings,
        TopAction::Brightness,
        TopAction::Theme,
        TopAction::Power,
    ]
    .into_iter()
    .find(|action| topbar_button_rect(win_w, *action).contains(pos))
}
