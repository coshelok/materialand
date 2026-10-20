//! Shared animation/clock tick.
use super::clock::LocalTime;
use crate::desktop::State;
use std::time::{Duration, Instant};
const MAX_TICK: f32 = 0.1;

/// Drives the parts of the shell that depend on wall-clock time: the top-bar
/// clock and the animation springs. Shared by the nested and the KMS loops.
pub struct Ticker {
    last_clock: Instant,
    last_frame: Instant,
    animating: bool,
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
            animating: false,
        }
    }

    /// Advance the clock, shell animations and pending window closes.
    /// Returns `true` while something is still animating, i.e. the caller
    /// should keep scheduling frames.
    pub fn tick(&mut self, state: &mut State, interval: Duration) -> bool {
        state
            .shell
            .clamp_home_page(state.display.size.w as f32, state.display.size.h as f32);
        let items = crate::input::dock_items(state);
        state.sync_overview();
        let mut dirty = state.services.poll();
        if let Some(snapshot) = state.notifications.poll() {
            state.shell.notifications = snapshot;
            dirty = true;
        }
        if dirty {
            state.shell.set_system_status(state.services.status.clone());
        }
        dirty |= state.shell.sync_dock(state.display.size.w as f32, &items);
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
        let dt = animation_step(now - self.last_frame, interval, self.animating);
        self.last_frame = now;
        self.animating = state.shell.tick(dt)
            | state.tick_overview(dt)
            | state.tick_split(dt)
            | crate::input::tick_shortcut_drag(state, dt);
        if self.animating {
            dirty = true;
        }
        let hover = if !crate::shell::tablet_mode()
            && !state.shell.launcher_open
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

/// Preserve real frame time during motion, but never consume a new animation
/// with the idle second that preceded the first frame.
fn animation_step(elapsed: Duration, interval: Duration, was_animating: bool) -> f32 {
    // Slow rendering is still elapsed animation time. Only an actual idle
    // period gets bounded by one refresh interval; dropped frames must not
    // stretch the configured animation duration on high-refresh displays.
    let elapsed = if was_animating {
        elapsed
    } else {
        elapsed.min(interval)
    };
    elapsed.as_secs_f32().min(MAX_TICK)
}

pub fn frame_interval(refresh_millihz: Option<i32>) -> Duration {
    let refresh = refresh_millihz.filter(|rate| *rate > 0).unwrap_or(60_000) as u64;
    Duration::from_nanos((1_000_000_000_000 / refresh).max(1))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn animation_clock_supports_high_and_fractional_refresh_without_a_sixty_hz_cap() {
        for rate in [
            30_000, 59_940, 60_000, 120_000, 144_000, 165_000, 240_000, 360_000,
        ] {
            let interval = frame_interval(Some(rate));
            assert_eq!(interval.as_nanos(), 1_000_000_000_000 / rate as u128);
            assert!(
                (animation_step(interval, interval, true) - 1000.0 / rate as f32).abs() < 0.000001
            );
            assert_eq!(
                animation_step(Duration::from_secs(1), interval, false),
                interval.as_secs_f32()
            );
        }
        let interval = frame_interval(Some(144_000));
        assert_eq!(
            animation_step(interval * 2, interval, true),
            (interval * 2).as_secs_f32()
        );
        assert_eq!(
            animation_step(Duration::from_millis(30), interval, true),
            0.03
        );
        assert_eq!(
            animation_step(Duration::from_secs(1), interval, true),
            MAX_TICK
        );
        assert_eq!(frame_interval(None), frame_interval(Some(0)));
        assert_eq!(frame_interval(None), frame_interval(Some(-1)));
    }
}
