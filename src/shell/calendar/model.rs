//! Clock state updates and animated panel placement.
use super::*;
impl Shell {
    pub fn set_calendar_open(&mut self, open: bool) {
        self.calendar.open = open;
        self.calendar.pressed = None;
        if open {
            self.calendar.navigate(CalendarAction::Today);
        }
        let target = if open { 1.0 } else { 0.0 };
        self.calendar.animation.set_target(target);
        if self.reduce_motion {
            self.calendar.animation.snap(target);
        }
    }
    pub fn update_time(&mut self, time: LocalTime) -> bool {
        let text = time.text();
        let changed = self.clock != text || (self.calendar.open && time != self.calendar.now);
        // Follow a new day only while the current month is being displayed.
        if self.calendar.year == self.calendar.now.year
            && self.calendar.month == self.calendar.now.month
        {
            self.calendar.year = time.year;
            self.calendar.month = time.month;
        }
        self.clock = text;
        self.calendar.now = time;
        changed
    }
    pub fn calendar_progress(&self) -> f32 {
        self.calendar.animation.value()
    }
    pub fn calendar_placement(&self, w: f32, h: f32) -> CardPlacement {
        let source = calendar_rect(w, h);
        let t = self.calendar_progress().clamp(0.0, 1.06);
        let scale = 0.86 + 0.14 * t;
        CardPlacement {
            source,
            target: RectF {
                x: source.x,
                y: source.y - 14.0 * (1.0 - t),
                w: source.w * scale,
                h: source.h * scale,
            },
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn month_navigation_crosses_years_and_returns_to_today() {
        let mut c = Calendar::new();
        c.year = 2026;
        c.month = 1;
        c.navigate(CalendarAction::Previous);
        assert_eq!((c.year, c.month), (2025, 12));
        c.navigate(CalendarAction::Next);
        assert_eq!((c.year, c.month), (2026, 1));
        c.navigate(CalendarAction::Today);
        assert_eq!((c.year, c.month), (c.now.year, c.now.month));
    }
}

#[cfg(test)]
mod clock_tests {
    use super::*;
    #[test]
    fn closed_clock_stays_idle_but_open_calendar_tracks_seconds_and_rollover() {
        let mut shell = Shell::new(vec![], ThemeSpec::DEFAULT);
        let mut time = LocalTime {
            year: 2026,
            month: 12,
            day: 31,
            weekday: 3,
            hour: 23,
            minute: 59,
            second: 57,
            zone: "UTC".into(),
        };
        shell.update_time(time.clone());
        time.second = 58;
        assert!(!shell.update_time(time.clone()));
        shell.set_calendar_open(true);
        time.second = 59;
        assert!(shell.update_time(time.clone()));
        time.year = 2027;
        time.month = 1;
        time.day = 1;
        time.weekday = 4;
        time.hour = 0;
        time.minute = 0;
        time.second = 0;
        assert!(shell.update_time(time.clone()));
        assert_eq!((shell.calendar.year, shell.calendar.month), (2027, 1));
        shell.calendar.navigate(CalendarAction::Previous);
        time.day = 2;
        time.weekday = 5;
        shell.update_time(time);
        assert_eq!((shell.calendar.year, shell.calendar.month), (2026, 12));
    }
}
