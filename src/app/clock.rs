//! The top-bar clock.
//!
//! Moved out of `main.rs` because both the nested and the KMS backends drive
//! it through the shared [`crate::crate::app::Ticker`].

use std::time::Instant;

/// Spawning `date` once per second used to cost a process per tick. Instead the
/// host's UTC offset is fetched once at start-up (and refreshed hourly, which
/// also picks up DST changes), and "HH:MM" is derived from `SystemTime` in
/// between — no process spawns on the hot path.
pub struct Clock {
    offset_secs: i64,
    last_sync: Instant,
    /// Set when the host would not report its UTC offset: falls back to
    /// spawning `date`, throttled to one call per minute.
    fallback_text: Option<String>,
}

impl Clock {
    pub fn new() -> Self {
        let mut clock = Self {
            offset_secs: 0,
            last_sync: Instant::now(),
            fallback_text: None,
        };
        clock.sync();
        clock
    }

    fn sync(&mut self) {
        self.last_sync = Instant::now();
        match utc_offset_secs() {
            Some(offset) => {
                self.offset_secs = offset;
                self.fallback_text = None;
            }
            None => {
                if self.fallback_text.is_none() {
                    self.fallback_text = Some(current_clock());
                }
            }
        }
    }

    pub fn text(&mut self) -> String {
        if let Some(text) = &self.fallback_text {
            if self.last_sync.elapsed().as_secs() >= 60 {
                self.last_sync = Instant::now();
                let fresh = current_clock();
                self.fallback_text = Some(fresh.clone());
                return fresh;
            }
            return text.clone();
        }
        if self.last_sync.elapsed().as_secs() >= 3600 {
            self.sync();
            if let Some(text) = &self.fallback_text {
                return text.clone();
            }
        }
        let secs = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);
        let day = (secs + self.offset_secs).rem_euclid(86_400);
        format!("{:02}:{:02}", day / 3600, (day % 3600) / 60)
    }
}

/// Ask `date` for the local UTC offset, e.g. `+0300`.
fn utc_offset_secs() -> Option<i64> {
    let out = std::process::Command::new("date")
        .arg("+%z")
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    parse_utc_offset(&String::from_utf8_lossy(&out.stdout))
}

fn parse_utc_offset(raw: &str) -> Option<i64> {
    let raw = raw.trim();
    let (sign, rest) = if let Some(r) = raw.strip_prefix('+') {
        (1, r)
    } else if let Some(r) = raw.strip_prefix('-') {
        (-1, r)
    } else {
        (1, raw)
    };
    let digits: String = rest.chars().filter(|c| c.is_ascii_digit()).collect();
    if digits.len() < 4 {
        return None;
    }
    let hours: i64 = digits[0..2].parse().ok()?;
    let minutes: i64 = digits[2..4].parse().ok()?;
    Some(sign * (hours * 3600 + minutes * 60))
}

fn current_clock() -> String {
    if let Ok(out) = std::process::Command::new("date").arg("+%H:%M").output() {
        if out.status.success() {
            let s = String::from_utf8_lossy(&out.stdout).trim().to_string();
            if !s.is_empty() {
                return s;
            }
        }
    }
    "00:00".to_string()
}
