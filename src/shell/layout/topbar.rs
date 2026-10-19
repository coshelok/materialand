//! Topbar geometry.
use super::*;
pub fn topbar_rect(win_w: f32) -> RectF {
    let design = crate::config::design();
    RectF {
        x: design.topbar.margin,
        y: design.topbar.margin,
        w: (win_w - 2.0 * design.topbar.margin).max(1.0),
        h: design.topbar.height,
    }
}

/// Screen rectangle of one top-bar icon button; render and hit-testing share
/// this layout so a press always lands on the drawn control.
pub fn topbar_button_rect(win_w: f32, action: TopAction) -> RectF {
    let design = crate::config::design();
    let rect = topbar_rect(win_w);
    let btn = design.topbar.height;
    if action == TopAction::Clock {
        return RectF {
            x: rect.x + design.topbar.clock_gap + btn,
            y: rect.y + (rect.h - design.topbar.clock_height) / 2.0 + design.topbar.clock_offset_y,
            w: design.topbar.clock_width,
            h: design.topbar.clock_height,
        };
    }
    let x = match action {
        TopAction::Home | TopAction::Launcher => rect.x + design.topbar.button_inset,
        TopAction::Clock => unreachable!(),
        TopAction::Power => rect.right() - design.topbar.button_inset - btn,
        TopAction::Theme => {
            rect.right() - design.topbar.button_inset - 2.0 * btn - design.topbar.icon_gap
        }
        TopAction::Brightness => {
            rect.right() - design.topbar.button_inset - 3.0 * btn - 2.0 * design.topbar.icon_gap
        }
        TopAction::QuickSettings => {
            rect.right() - design.topbar.button_inset - 4.0 * btn - 3.0 * design.topbar.icon_gap
        }
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
        TopAction::Home,
        TopAction::Clock,
        TopAction::QuickSettings,
        TopAction::Brightness,
        TopAction::Theme,
        TopAction::Power,
    ]
    .into_iter()
    .find(|action| topbar_button_rect(win_w, *action).contains(pos))
}
