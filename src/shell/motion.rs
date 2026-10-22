//! Motion.
use super::*;

impl Shell {
    pub fn theme_fade(&self) -> Option<(Theme, f32)> {
        match &self.prev_theme {
            Some(prev) if !self.theme_mix.is_done() => Some((*prev, self.theme_mix.value())),
            _ => None,
        }
    }

    pub fn launcher_progress(&self) -> f32 {
        self.launcher.value()
    }

    pub fn brightness_progress(&self) -> f32 {
        self.brightness.value()
    }

    pub fn set_launcher_open(&mut self, open: bool) {
        self.launcher.set_target(if open { 1.0 } else { 0.0 });
        if self.reduce_motion {
            self.launcher.snap(if open { 1.0 } else { 0.0 });
        }
    }

    pub fn set_launcher_origin(&mut self, rect: RectF) {
        self.launcher_origin = Some(rect);
    }

    pub fn set_brightness_open(&mut self, open: bool) {
        self.brightness_open = open;
        self.brightness.set_target(if open { 1.0 } else { 0.0 });
        if self.reduce_motion {
            self.brightness.snap(if open { 1.0 } else { 0.0 });
        }
    }

    pub fn quick_settings_progress(&self) -> f32 {
        self.quick_settings.value()
    }

    pub fn set_quick_settings_open(&mut self, open: bool) {
        self.quick_settings_open = open;
        self.quick_settings.set_target(if open { 1.0 } else { 0.0 });
        if self.reduce_motion {
            self.quick_settings.snap(if open { 1.0 } else { 0.0 });
        }
    }

    pub fn set_qs_origin(&mut self, rect: RectF) {
        self.qs_origin = Some(rect);
    }

    pub fn power_progress(&self) -> f32 {
        self.power.value()
    }

    pub fn set_power_open(&mut self, open: bool) {
        self.power_open = open;
        self.power.set_target(if open { 1.0 } else { 0.0 });
        if self.reduce_motion {
            self.power.snap(if open { 1.0 } else { 0.0 });
        }
    }

    pub fn night_light_progress(&self) -> f32 {
        self.night_light.value()
    }

    pub fn toggle_night_light(&mut self) {
        self.night_light_on = !self.night_light_on;
        let target = if self.night_light_on { 1.0 } else { 0.0 };
        self.night_light.set_target(target);
        let switch = &mut self.switches[quick_action_index(QuickAction::NightLight)];
        switch.set_target(target);
        if self.reduce_motion {
            self.night_light.snap(target);
            switch.snap(target);
        }
    }

    pub fn set_theme_switch(&mut self, dark: bool) {
        let target = if dark { 1.0 } else { 0.0 };
        let switch = &mut self.switches[quick_action_index(QuickAction::Theme)];
        switch.set_target(target);
        if self.reduce_motion {
            switch.snap(target);
        }
    }

    pub fn set_slider_active(&mut self, active: bool) {
        let target = if active { 1.0 } else { 0.0 };
        self.slider.set_target(target);
        if self.reduce_motion {
            self.slider.snap(target);
        }
    }

    pub fn set_reduce_motion(&mut self, enabled: bool) {
        self.reduce_motion = enabled;
        // The tile's own switch must follow the setting, not just the springs.
        let switch = &mut self.switches[quick_action_index(QuickAction::ReduceMotion)];
        switch.set_target(if enabled { 1.0 } else { 0.0 });
        if enabled {
            self.dock_motion.snap();
            self.home.snap();
            self.calendar.animation.finish();
            self.launcher_scroll.snap(self.launcher_scroll_target);
            self.shade_scroll_motion.snap(self.shade_scroll_target);
            self.shade_scroll = self.shade_scroll_target;
            self.notice_scroll.snap(self.notice_scroll_target);
            switch.snap(1.0);
            self.launcher
                .snap(if self.launcher_open { 1.0 } else { 0.0 });
            self.brightness
                .snap(if self.brightness_open { 1.0 } else { 0.0 });
            self.quick_settings
                .snap(if self.quick_settings_open { 1.0 } else { 0.0 });
            self.power.snap(if self.power_open { 1.0 } else { 0.0 });
            self.night_light
                .snap(if self.night_light_on { 1.0 } else { 0.0 });
            self.theme_mix.snap(1.0);
            self.prev_theme = None;
            self.reveal = None;
            self.slider.snap(self.slider.value());
            for s in self.switches.iter_mut() {
                s.snap(s.value());
            }
            self.window_in.clear();
            self.launch_origins = Default::default();
            for s in self.window_out.values_mut() {
                s.snap(1.0);
            }
            self.press.clear();
        }
    }

    pub fn begin_theme_change(&mut self, old: Theme) {
        self.reveal = None;
        if self.reduce_motion {
            self.prev_theme = None;
            self.theme_mix.snap(1.0);
            return;
        }
        self.prev_theme = Some(old);
        self.theme_mix.start(0.0, 1.0);
    }

    pub fn begin_reveal(&mut self, from: Theme, origin: Point<f64, Logical>) {
        self.prev_theme = None;
        self.theme_mix.snap(1.0);
        if self.reduce_motion {
            self.reveal = None;
            return;
        }
        self.reveal = Some(Reveal {
            from,
            origin,
            progress: Tween::new(
                0.0,
                1.0,
                crate::config::design().motion.reveal_duration,
                CubicBezier::emphasized_decelerate(),
            ),
        });
    }

    pub fn reveal_frame(&self) -> Option<(Theme, Point<f64, Logical>, f32)> {
        self.reveal
            .as_ref()
            .map(|r| (r.from, r.origin, r.progress.value()))
    }

    pub fn effective_theme(&self, target: &Theme) -> Theme {
        match &self.prev_theme {
            Some(prev) if !self.theme_mix.is_done() => {
                Theme::blend(prev, target, self.theme_mix.value())
            }
            _ => *target,
        }
    }

    pub fn tick(&mut self, dt: f32) -> bool {
        let mut active = self.dock_motion.tick(dt);
        active |= self.shade_scroll_motion.step(dt);
        active |= self.notice_scroll.step(dt);
        self.shade_scroll = self.shade_scroll_motion.value().max(0.0);
        active |= self.home.tick(dt);
        active |= self.calendar.animation.step(dt);
        active |= self.launcher_scroll.step(dt);
        active |= self.launcher.step(dt);
        active |= self.brightness.step(dt);
        active |= self.quick_settings.step(dt);
        active |= self.power.step(dt);
        active |= self.night_light.step(dt);
        active |= self.slider.step(dt);
        for s in self.switches.iter_mut() {
            active |= s.step(dt);
        }
        for s in self.window_in.values_mut() {
            active |= s.step(dt);
        }
        for s in self.window_out.values_mut() {
            active |= s.step(dt);
        }
        self.window_in.retain(|_, s| !s.settled());
        for p in self.press.values_mut() {
            active |= p.scale.step(dt);
            if let Some(r) = &mut p.ripple {
                active |= r.progress.step(dt);
            }
        }
        self.press.retain(|_, p| {
            let ripple = p.ripple.as_ref().is_some_and(|r| !r.progress.is_done());
            p.pressed || ripple || !p.scale.settled()
        });
        if !self.theme_mix.is_done() {
            active |= self.theme_mix.step(dt);
            if self.theme_mix.is_done() {
                self.prev_theme = None;
            }
        }
        let mut reveal_done = false;
        if let Some(r) = &mut self.reveal {
            if !r.progress.is_done() {
                active |= r.progress.step(dt);
                reveal_done = r.progress.is_done();
            }
        }
        if reveal_done {
            self.reveal = None;
        }
        active
    }
}
