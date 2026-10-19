//! Local civil time from libc, including date, zone and DST on every tick.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LocalTime {
    pub year: i32,
    pub month: u32,
    pub day: u32,
    /// Monday = 0.
    pub weekday: usize,
    pub hour: u32,
    pub minute: u32,
    pub second: u32,
    pub zone: String,
}
impl LocalTime {
    pub fn now() -> Self {
        // localtime_r owns no shared tm storage. strftime reads the returned
        // tm immediately; the zone string is copied, never retained by us.
        unsafe {
            let timestamp = libc::time(std::ptr::null_mut());
            let mut local: libc::tm = std::mem::zeroed();
            if !libc::localtime_r(&timestamp, &mut local).is_null() {
                let mut zone = [0i8; 64];
                libc::strftime(
                    zone.as_mut_ptr(),
                    zone.len(),
                    c"%Z · UTC%z".as_ptr(),
                    &local,
                );
                return Self {
                    year: local.tm_year + 1900,
                    month: (local.tm_mon + 1) as u32,
                    day: local.tm_mday as u32,
                    weekday: ((local.tm_wday + 6) % 7) as usize,
                    hour: local.tm_hour as u32,
                    minute: local.tm_min as u32,
                    second: local.tm_sec as u32,
                    zone: std::ffi::CStr::from_ptr(zone.as_ptr())
                        .to_string_lossy()
                        .into_owned(),
                };
            }
        }
        Self {
            year: 1970,
            month: 1,
            day: 1,
            weekday: 3,
            hour: 0,
            minute: 0,
            second: 0,
            zone: "Time unavailable".into(),
        }
    }
    pub fn text(&self) -> String {
        format!("{:02}:{:02}", self.hour, self.minute)
    }
    pub fn seconds_text(&self) -> String {
        format!("{:02}:{:02}:{:02}", self.hour, self.minute, self.second)
    }
    pub fn date_text(&self) -> String {
        const WEEKDAYS: [&str; 7] = [
            "Monday",
            "Tuesday",
            "Wednesday",
            "Thursday",
            "Friday",
            "Saturday",
            "Sunday",
        ];
        format!(
            "{}, {} {} {}",
            WEEKDAYS[self.weekday],
            self.day,
            MONTHS[(self.month - 1) as usize],
            self.year
        )
    }
}
pub const MONTHS: [&str; 12] = [
    "January",
    "February",
    "March",
    "April",
    "May",
    "June",
    "July",
    "August",
    "September",
    "October",
    "November",
    "December",
];

pub fn days_in_month(year: i32, month: u32) -> u32 {
    match month {
        2 => {
            if year % 4 == 0 && (year % 100 != 0 || year % 400 == 0) {
                29
            } else {
                28
            }
        }
        4 | 6 | 9 | 11 => 30,
        _ => 31,
    }
}
/// Gregorian weekday without timezone conversion or a noon/DST ambiguity.
pub fn first_weekday(year: i32, month: u32) -> usize {
    let y = year - i32::from(month < 3);
    let offsets = [0, 3, 2, 5, 0, 3, 5, 1, 4, 6, 2, 4];
    (y + y.div_euclid(4) - y.div_euclid(100)
        + y.div_euclid(400)
        + offsets[(month - 1) as usize]
        + 1
        + 6)
    .rem_euclid(7) as usize
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn gregorian_calendar_handles_leap_centuries_and_monday_first() {
        assert_eq!(days_in_month(2000, 2), 29);
        assert_eq!(days_in_month(2100, 2), 28);
        assert_eq!(days_in_month(2024, 2), 29);
        assert_eq!(first_weekday(2026, 10), 3);
        assert_eq!(first_weekday(2024, 1), 0);
        assert_eq!(first_weekday(2026, 2), 6);
    }
}
