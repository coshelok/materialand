//! Topbar geometry.
use super::*;
pub fn topbar_rect(win_w: f32) -> RectF {
    RectF {
        x: config::TOPBAR_MARGIN,
        y: config::TOPBAR_MARGIN,
        w: (win_w - 2.0 * config::TOPBAR_MARGIN).max(1.0),
        h: config::TOPBAR_HEIGHT,
    }
}

/// Screen rectangle of one top-bar icon button; render and hit-testing share
/// this layout so a press always lands on the drawn control.
pub fn topbar_button_rect(win_w: f32, action: TopAction) -> RectF {
    let rect = topbar_rect(win_w);
    let btn = config::TOPBAR_HEIGHT;
    if action == TopAction::Clock {
        return RectF {
            x: rect.x + 16.0 + btn,
            y: rect.y + 8.0,
            w: 80.0,
            h: 32.0,
        };
    }
    let x = match action {
        TopAction::Launcher => rect.x + 8.0,
        TopAction::Clock => unreachable!(),
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
        TopAction::Clock,
        TopAction::QuickSettings,
        TopAction::Brightness,
        TopAction::Theme,
        TopAction::Power,
    ]
    .into_iter()
    .find(|action| topbar_button_rect(win_w, *action).contains(pos))
}
