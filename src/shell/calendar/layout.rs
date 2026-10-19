//! Clock sheet geometry, shared by rendering and pointer handling.
use super::*;
pub fn calendar_rect(w: f32, h: f32) -> RectF {
    let bar = topbar_rect(w);
    RectF {
        x: bar.x + 8.0,
        y: bar.bottom() + 8.0,
        w: 360.0_f32.min((bar.w - 16.0).max(1.0)),
        h: 456.0_f32.min((h - bar.bottom() - 20.0).max(1.0)),
    }
}
pub(super) fn nav_rect(card: RectF, action: CalendarAction) -> RectF {
    let (x, w) = match action {
        CalendarAction::Previous => (card.w - 104.0, 40.0),
        CalendarAction::Next => (card.w - 56.0, 40.0),
        CalendarAction::Today => (20.0, 88.0),
    };
    RectF {
        x: card.x + x,
        y: card.y
            + if action == CalendarAction::Today {
                card.h - 52.0
            } else {
                150.0
            },
        w,
        h: 36.0,
    }
}
pub fn calendar_hit(w: f32, h: f32, pos: Point<f64, Logical>) -> Option<CalendarAction> {
    let card = calendar_rect(w, h);
    [
        CalendarAction::Previous,
        CalendarAction::Today,
        CalendarAction::Next,
    ]
    .into_iter()
    .find(|a| nav_rect(card, *a).contains(pos))
}
