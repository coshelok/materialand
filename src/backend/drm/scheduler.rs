//! Vblank paces presentation; a coalesced timer advances animation and the clock.
use super::*;
impl App {
    pub(super) fn frame_budget(&self) -> Duration {
        let refresh = self
            .state
            .display
            .handle
            .current_mode()
            .map(|m| m.refresh)
            .unwrap_or(60_000)
            .max(1) as u64;
        Duration::from_nanos((1_000_000_000_000 / refresh).clamp(1_000_000, 100_000_000))
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
        self.animating = self.ticker.tick(&mut self.state);
        self.dirty |= self.animating;
        if self.dirty && !self.pending() {
            render(self);
        }
        self.schedule(if self.animating || self.dirty || self.pending() {
            self.frame_budget()
        } else {
            IDLE_TICK
        });
        TimeoutAction::Drop
    }
    pub(super) fn request_redraw(&mut self) {
        self.dirty = true;
        self.schedule(Duration::ZERO);
    }
}
