//! Shared animation/clock tick.
use super::clock::LocalTime;
use crate::desktop::State;
use std::time::Instant;
const MAX_TICK: f32 = 0.1;

/// Drives the parts of the shell that depend on wall-clock time: the top-bar
/// clock and the animation springs. Shared by the nested and the KMS loops.
pub struct Ticker {
    last_clock: Instant,
    last_frame: Instant,
}

impl Ticker {
    pub fn new(state: &mut State) -> Self {
        state.shell.sync_dock(
            state.display.size.w as f32,
            &crate::input::dock_items(state),
        );
        state.shell.update_time(LocalTime::now());
        Self {
            last_clock: Instant::now(),
            last_frame: Instant::now(),
        }
    }

    /// Advance the clock, shell animations and pending window closes.
    /// Returns `true` while something is still animating, i.e. the caller
    /// should keep scheduling frames.
    pub fn tick(&mut self, state: &mut State) -> bool {
        state
            .shell
            .clamp_home_page(state.display.size.w as f32, state.display.size.h as f32);
        let items = crate::input::dock_items(state);
        let mut dirty = state.shell.sync_dock(state.display.size.w as f32, &items);
        state
            .shell
            .clamp_launcher_scroll(state.display.size.w as f32, state.display.size.h as f32);

        // Update the clock once per second, but only mark the frame dirty when
        // the displayed minute actually changes: an idle compositor must not
        // repaint once per second just to redraw the same "HH:MM".
        if self.last_clock.elapsed().as_secs() >= 1 {
            self.last_clock = Instant::now();
            dirty |= state.shell.update_time(LocalTime::now());
        }

        // Advance shell animations; keep redrawing while any is in flight.
        // The step is clamped: ticks can span nearly a second when the
        // compositor has been idle, and such a leap would complete animations
        // that just started in a single frame (see `MAX_TICK`).
        let now = Instant::now();
        let dt = (now - self.last_frame).as_secs_f32().min(MAX_TICK);
        self.last_frame = now;
        if state.shell.tick(dt) {
            dirty = true;
        }
        let hover = if !state.shell.launcher_open
            && state.shell.app_menu.is_none()
            && !state.has_fullscreen_window()
        {
            state.shell.dock_hit(
                state.display.size.w as f32,
                state.display.size.h as f32,
                items.len(),
                state.cursor,
            )
        } else {
            None
        };
        dirty |= state.shell.set_hover_dock(hover);
        // Windows whose close animation finished get their `send_close` now.
        state.finish_closing();

        dirty
    }
}
