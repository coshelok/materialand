//! Smooth bounded scrolling shared by launcher painting, picking and wheel input.
use super::*;
impl Shell {
    pub fn launcher_scroll(&self) -> f32 {
        self.launcher_scroll.value().max(0.0)
    }
    pub fn reset_launcher_scroll(&mut self) {
        self.launcher_scroll.snap(0.0);
        self.launcher_scroll_target = 0.0;
        self.press_launch = None;
        self.press
            .retain(|key, _| !matches!(key, PressKey::Launcher(_)));
    }
    pub fn clamp_launcher_scroll(&mut self, w: f32, h: f32) {
        let max = launcher_scroll_max(w, h, self.filtered_apps().len());
        self.launcher_scroll_target = self.launcher_scroll_target.clamp(0.0, max);
        if self.launcher_scroll() > max {
            self.launcher_scroll.snap(max);
        }
        self.launcher_scroll.set_target(self.launcher_scroll_target);
    }
    pub fn scroll_launcher(&mut self, w: f32, h: f32, delta: f32) {
        if !delta.is_finite() {
            return;
        }
        self.clamp_launcher_scroll(w, h);
        let max = launcher_scroll_max(w, h, self.filtered_apps().len());
        self.launcher_scroll_target = (self.launcher_scroll_target + delta).clamp(0.0, max);
        self.launcher_scroll.set_target(self.launcher_scroll_target);
        if self.reduce_motion {
            self.launcher_scroll.snap(self.launcher_scroll_target);
        }
        self.press_launch = None;
        self.press
            .retain(|key, _| !matches!(key, PressKey::Launcher(_)));
    }
}

impl Shell {
    /// Finger input follows directly; only release inertia moves it afterwards.
    pub(crate) fn follow_touchpad_scroll(&mut self) {
        self.launcher_scroll.snap(self.launcher_scroll_target);
        self.shade_scroll_motion.snap(self.shade_scroll_target);
        self.shade_scroll = self.shade_scroll_target;
        self.notice_scroll.snap(self.notice_scroll_target);
    }
    pub(crate) fn stop_touchpad_scroll(&mut self) {
        self.launcher_scroll_target = self.launcher_scroll();
        self.shade_scroll_target = self.shade_scroll;
        self.notice_scroll_target = self.notice_scroll.value();
        self.follow_touchpad_scroll();
    }
    pub(crate) fn touchpad_scroll_positions(&self) -> (f32, f32, f32) {
        (
            self.launcher_scroll(),
            self.shade_scroll,
            self.notice_scroll.value(),
        )
    }
}
