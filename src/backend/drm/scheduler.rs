//! Page flips pace animation; timers start redraws, retry unchanged scenes and maintain the idle clock.
use super::*;
impl App {
    pub(super) fn frame_budget(&self) -> Duration {
        crate::app::frame_interval(self.state.display.handle.current_mode().map(|m| m.refresh))
    }
    pub(super) fn schedule(&mut self, delay: Duration) {
        if !self.backend.active || self.backend.gpu.is_none() {
            return;
        }
        let deadline = Instant::now() + delay;
        if let Some((token, current)) = self.timer {
            if current <= deadline {
                return;
            }
            self.backend.handle.remove(token);
            self.timer = None;
        }
        match self
            .backend
            .handle
            .insert_source(Timer::from_duration(delay), |_, _, app: &mut App| {
                app.on_timer()
            }) {
            Ok(token) => self.timer = Some((token, deadline)),
            Err(error) => tracing::error!(%error,"frame timer registration failed"),
        }
    }
    pub(super) fn pending(&self) -> bool {
        self.backend
            .gpu
            .as_ref()
            .is_some_and(|gpu| gpu.output.frame.is_pending())
    }
    fn on_timer(&mut self) -> TimeoutAction {
        self.timer = None;
        self.redraw();
        TimeoutAction::Drop
    }
    pub(super) fn redraw(&mut self) {
        // A queued page flip owns the next wakeup. Input may mark the scene
        // dirty, but must never advance animations or enqueue a second frame.
        if self.pending() {
            return;
        }
        let was_animating = self.animating;
        let interval = self.frame_budget();
        self.animating = self.ticker.tick(&mut self.state, interval);
        // Include the final settled pose even when tick reports no more motion.
        self.dirty |= was_animating || self.animating;
        if let Some(gpu) = self.backend.gpu.as_mut() {
            if let Some(backlight) = &mut gpu.backlight {
                self.dirty |= backlight.sync(&mut self.state);
            }
            if let Some(light) = &mut gpu.night_light {
                if let Err(error) = light.update(self.state.shell.night_light_progress()) {
                    tracing::warn!(%error, "night light is unavailable on this output");
                    gpu.night_light = None;
                    self.state.shell.night_light_available = false;
                    if self.state.shell.night_light_on {
                        self.state.shell.toggle_night_light();
                    }
                    self.dirty = true;
                }
            }
        }
        if self.dirty && !self.pending() {
            render(self);
        }
        if !self.pending() {
            self.schedule(if self.animating || self.dirty {
                self.frame_budget()
            } else {
                IDLE_TICK
            });
        }
    }
    pub(super) fn request_redraw(&mut self) {
        self.dirty = true;
        if !self.pending() {
            self.schedule(Duration::ZERO);
        }
    }
}
