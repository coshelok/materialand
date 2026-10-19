//! Clock sheet geometry, shared by rendering and pointer handling.
use super::*;
pub fn calendar_rect(w: f32, h: f32) -> RectF {
    let design = crate::config::design();
    let bar = topbar_rect(w);
    RectF {
        x: bar.x + design.panels.edge_inset,
        y: bar.bottom() + design.panels.edge_inset,
        w: design
            .calendar
            .width
            .min((bar.w - 2.0 * design.panels.edge_inset).max(1.0)),
        h: design
            .calendar
            .height
            .min((h - bar.bottom() - design.panels.bottom_gap).max(1.0)),
    }
}
pub(super) fn nav_rect(card: RectF, action: CalendarAction) -> Option<RectF> {
    let design = crate::config::design();
    let (x, w) = match action {
        CalendarAction::Previous => (
            card.w
                - design.calendar.nav_right
                - 2.0 * design.calendar.nav_width
                - design.calendar.nav_gap,
            design.calendar.nav_width,
        ),
        CalendarAction::Next => (
            card.w - design.calendar.nav_right - design.calendar.nav_width,
            design.calendar.nav_width,
        ),
        CalendarAction::Today => return None,
    };
    Some(RectF {
        x: card.x + x,
        y: card.y + design.calendar.nav_y,
        w,
        h: design.calendar.nav_height,
    })
}
pub fn calendar_hit(w: f32, h: f32, pos: Point<f64, Logical>) -> Option<CalendarAction> {
    let card = calendar_rect(w, h);
    [CalendarAction::Previous, CalendarAction::Next]
        .into_iter()
        .find(|a| nav_rect(card, *a).is_some_and(|r| r.contains(pos)))
}
