//! A two-finger sequence owns one shell list until release; inertia never reaches clients.
use super::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Owner {
    Launcher,
    Shade(Option<QuickAction>, bool),
    Notifications,
    Overview,
    Home,
}
#[derive(Default)]
pub(crate) struct TouchpadScroll {
    owner: Option<Owner>,
    device: String,
    horizontal: bool,
    last_time: Option<u32>,
    velocity: f64,
    coasting: bool,
    page_distance: f64,
    page_sent: bool,
    interrupted: bool,
}
fn available(state: &State) -> bool {
    matches!(state.interaction, Interaction::None)
        && !state.pointer.is_grabbed()
        && state.forwarded_buttons.is_empty()
        && state.swallowed_buttons.is_empty()
        && state.shortcut_drag.is_none()
        && state.shell.app_menu.is_none()
        && !state.shell.calendar.open
        && !state.shell.power_open
        && !state.shell.brightness_open
}
fn valid(state: &State, owner: Owner) -> bool {
    available(state)
        && match owner {
            Owner::Overview => state.overview.open,
            Owner::Launcher => state.shell.launcher_open && !state.overview.visible(),
            Owner::Shade(page, notices) => {
                state.shell.quick_settings_open
                    && state.shell.connection_page == page
                    && state.shell.notifications_open == notices
            }
            Owner::Notifications => {
                state.shell.quick_settings_open && shell::shade_split(state.display.size.w as f32)
            }
            Owner::Home => {
                !state.overview.visible()
                    && !state.shell.launcher_open
                    && state.shell.launcher_progress() <= 0.001
                    && !state.shell.quick_settings_open
                    && state.shell.quick_settings_progress() <= 0.001
                    && state.windows.space.elements().next().is_none()
            }
        }
}
fn pick(state: &State) -> Option<Owner> {
    if !available(state) {
        return None;
    }
    let (w, h) = (state.display.size.w as f32, state.display.size.h as f32);
    if state.overview.open {
        return Some(Owner::Overview);
    }
    if state.shell.quick_settings_open {
        let pos = state
            .shell
            .quick_settings_placement(w, h, state.shell.quick_settings_progress())
            .unmap(state.cursor);
        if shell::shade_split(w) && shell::notification_panel(w, h).contains(pos) {
            return Some(Owner::Notifications);
        }
        return shell::quick_settings_rect(w, h)
            .contains(pos)
            .then_some(Owner::Shade(
                state.shell.connection_page,
                state.shell.notifications_open,
            ));
    }
    if state.shell.launcher_open {
        let pos = state
            .shell
            .launcher_placement(w, h, state.shell.launcher_progress())
            .unmap(state.cursor);
        return shell::launcher_viewport(w, h)
            .contains(pos)
            .then_some(Owner::Launcher);
    }
    (valid(state, Owner::Home) && shell::home_layout(w, h).grid.contains(state.cursor))
        .then_some(Owner::Home)
}
pub(crate) fn stop(state: &mut State) {
    if state.touchpad_scroll.owner.is_some() {
        state.shell.stop_touchpad_scroll();
    }
    state.touchpad_scroll = Default::default();
}
pub(crate) fn interrupt(state: &mut State) {
    if state.touchpad_scroll.coasting {
        state.touchpad_scroll = Default::default();
    } else {
        state.touchpad_scroll.interrupted = true;
        state.touchpad_scroll.velocity = 0.0;
    }
    state.shell.stop_touchpad_scroll();
}
// Deltas are already in the configured scroll direction. None means no event on that axis.
pub(super) fn input(
    state: &mut State,
    device: String,
    x: Option<f64>,
    y: Option<f64>,
    time: u32,
) -> bool {
    let mut scroll = std::mem::take(&mut state.touchpad_scroll);
    if scroll.coasting {
        // Separate per-axis stop events can follow the release that started inertia.
        if ![x, y]
            .into_iter()
            .flatten()
            .any(|v| v.is_finite() && v != 0.0)
        {
            state.touchpad_scroll = scroll;
            return true;
        }
        scroll = Default::default();
        state.shell.stop_touchpad_scroll();
    }
    if scroll.owner.is_some() && scroll.device != device {
        state.touchpad_scroll = scroll;
        return true;
    }
    if scroll.owner.is_none() {
        let Some(owner) = pick(state) else {
            return false;
        };
        if owner == Owner::Home && x.unwrap_or(0.0).abs() <= y.unwrap_or(0.0).abs() * 1.4 {
            return true;
        }
        state.shell.stop_touchpad_scroll();
        scroll.owner = Some(owner);
        scroll.interrupted = false;
        scroll.device = device;
        scroll.horizontal = owner == Owner::Home
            || (owner == Owner::Overview && x.unwrap_or(0.0).abs() > y.unwrap_or(0.0).abs());
    }
    let owner = scroll.owner.unwrap();
    let amount = if scroll.horizontal { x } else { y };
    let Some(delta) = amount.filter(|v| v.is_finite()) else {
        if scroll.last_time.is_none() && (x == Some(0.0) || y == Some(0.0)) {
            return true;
        }
        state.touchpad_scroll = scroll;
        return true;
    };
    if delta == 0.0 {
        let fresh = scroll
            .last_time
            .is_some_and(|last| time.wrapping_sub(last) <= 100);
        scroll.coasting = fresh
            && !scroll.interrupted
            && valid(state, owner)
            && config::design().input.touchpad.kinetic_scroll
            && !state.shell.reduce_motion
            && matches!(
                owner,
                Owner::Launcher | Owner::Shade(..) | Owner::Notifications
            )
            && scroll.velocity.abs() >= 20.0;
        if scroll.coasting {
            state.touchpad_scroll = scroll;
        }
        return true;
    }
    if !scroll.interrupted && valid(state, owner) {
        if let Some(last) = scroll.last_time {
            let elapsed = time.wrapping_sub(last);
            if elapsed > 0 && elapsed <= 100 {
                let sample = (delta * 1000.0 / f64::from(elapsed)).clamp(-3000.0, 3000.0);
                if sample.signum() != scroll.velocity.signum() {
                    scroll.velocity = sample;
                } else {
                    scroll.velocity = 0.4 * scroll.velocity + 0.6 * sample;
                }
            } else if elapsed > 100 {
                scroll.velocity = 0.0;
            }
        }
        scroll.last_time = Some(time);
        if owner == Owner::Home {
            scroll.page_distance += delta;
            if !scroll.page_sent
                && scroll.page_distance.abs()
                    >= f64::from(config::design().input.touchpad.swipe_distance)
            {
                state.shell.turn_home_page(
                    state.display.size.w as f32,
                    state.display.size.h as f32,
                    if scroll.page_distance > 0.0 { 1 } else { -1 },
                );
                scroll.page_sent = true;
            }
        } else {
            apply(state, owner, delta);
        }
    } else {
        scroll.interrupted = true;
        scroll.velocity = 0.0;
    }
    state.touchpad_scroll = scroll;
    true
}
fn apply(state: &mut State, owner: Owner, delta: f64) -> bool {
    let (w, h) = (state.display.size.w as f32, state.display.size.h as f32);
    let before = state.shell.touchpad_scroll_positions();
    match owner {
        Owner::Launcher => state.shell.scroll_launcher(w, h, delta as f32),
        Owner::Shade(..) => state
            .shell
            .scroll_shade(delta as f32, state.shell.shade_scroll_limit(w, h)),
        Owner::Notifications => state.shell.scroll_notifications(delta as f32, w, h),
        Owner::Overview => state.overview.scroll(delta, state.shell.reduce_motion),
        Owner::Home => {}
    }
    state.shell.follow_touchpad_scroll();
    before != state.shell.touchpad_scroll_positions()
}
fn coast(velocity: f64, dt: f64, friction: f64) -> (f64, f64) {
    let decay = (-friction * dt).exp();
    (velocity * (1.0 - decay) / friction, velocity * decay)
}
pub(crate) fn tick(state: &mut State, dt: f32) -> bool {
    let mut scroll = std::mem::take(&mut state.touchpad_scroll);
    if !scroll.coasting {
        state.touchpad_scroll = scroll;
        return false;
    }
    let Some(owner) = scroll.owner else {
        return false;
    };
    if !valid(state, owner) || state.shell.reduce_motion {
        return false;
    }
    let (delta, velocity) = coast(
        scroll.velocity,
        f64::from(dt.max(0.0)),
        f64::from(config::design().input.touchpad.scroll_friction),
    );
    let changed = apply(state, owner, delta);
    scroll.velocity = velocity;
    if changed && velocity.abs() >= 8.0 {
        state.touchpad_scroll = scroll;
    }
    changed
}
#[cfg(test)]
#[path = "touchpad_scroll_tests.rs"]
mod tests;
