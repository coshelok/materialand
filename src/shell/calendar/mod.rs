//! Clock sheet, month navigation and shared rendering/input geometry.
use super::placement::CardPlacement;
use super::*;
use crate::app::clock::{days_in_month, first_weekday, LocalTime, MONTHS};
mod layout;
mod model;
mod paint;
use layout::nav_rect;
pub use layout::{calendar_hit, calendar_rect};
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum CalendarAction {
    Previous,
    Today,
    Next,
}
pub struct Calendar {
    pub open: bool,
    pub now: LocalTime,
    pub year: i32,
    pub month: u32,
    pub(super) animation: Spring,
    pub pressed: Option<CalendarAction>,
}
impl Calendar {
    pub(super) fn new() -> Self {
        let now = LocalTime::now();
        Self {
            year: now.year,
            month: now.month,
            now,
            open: false,
            animation: Spring::spatial(0.0),
            pressed: None,
        }
    }
    pub fn navigate(&mut self, action: CalendarAction) {
        let delta = match action {
            CalendarAction::Previous => -1,
            CalendarAction::Next => 1,
            CalendarAction::Today => {
                self.year = self.now.year;
                self.month = self.now.month;
                return;
            }
        };
        let absolute = (self.year * 12 + self.month as i32 - 1 + delta).clamp(12, 9999 * 12 + 11);
        self.year = absolute.div_euclid(12);
        self.month = absolute.rem_euclid(12) as u32 + 1;
    }
}
