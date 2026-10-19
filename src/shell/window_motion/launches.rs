//! Match each pending launch to its own application, including slow startups.
use super::*;
use std::{
    collections::VecDeque,
    time::{Duration, Instant},
};
#[derive(Default)]
pub(in crate::shell) struct LaunchOrigins(VecDeque<Pending>);
struct Pending {
    app: AppEntry,
    origin: RectF,
    time: Instant,
}
impl LaunchOrigins {
    pub(super) fn remember(&mut self, app: AppEntry, origin: RectF) {
        self.0
            .retain(|p| p.time.elapsed() < Duration::from_secs(20));
        if self.0.len() >= 32 {
            self.0.pop_front();
        }
        self.0.push_back(Pending {
            app,
            origin,
            time: Instant::now(),
        });
    }
    pub(super) fn take(&mut self, app_id: &str) -> Option<RectF> {
        self.0
            .retain(|p| p.time.elapsed() < Duration::from_secs(20));
        let i = self.0.iter().position(|p| p.app.matches_app_id(app_id))?;
        self.0.remove(i).map(|p| p.origin)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn app(name: &str) -> AppEntry {
        AppEntry {
            name: name.into(),
            exec: name.into(),
            icon: String::new(),
            terminal: false,
            desktop_file: Some(format!("/apps/{name}.desktop").into()),
        }
    }
    fn rect(x: f32) -> RectF {
        RectF {
            x,
            y: 100.0,
            w: 56.0,
            h: 56.0,
        }
    }
    #[test]
    fn out_of_order_startups_get_their_own_icon_and_origins_are_used_once() {
        let mut origins = LaunchOrigins::default();
        origins.remember(app("firefox"), rect(30.0));
        origins.remember(app("code"), rect(300.0));
        assert!(origins.take("").is_none());
        assert!(origins.take("unrelated").is_none());
        assert_eq!(origins.take("code").unwrap().x, 300.0);
        assert_eq!(origins.take("firefox").unwrap().x, 30.0);
        assert!(origins.take("code").is_none());
    }
    #[test]
    fn failed_or_expired_launches_cannot_animate_unrelated_later_windows() {
        let mut origins = LaunchOrigins::default();
        origins.remember(app("firefox"), rect(30.0));
        origins.0.front_mut().unwrap().time = Instant::now() - Duration::from_secs(21);
        assert!(origins.take("firefox").is_none());
    }
}
