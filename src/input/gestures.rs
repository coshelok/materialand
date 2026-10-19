//! Three-finger launcher swipe; cancelled/diagonal/incomplete gestures do nothing.
use super::*;
#[derive(Default)]
pub(crate) struct LauncherSwipe {
    device: Option<String>,
    x: f64,
    y: f64,
    open: bool,
}
#[derive(Debug, PartialEq, Eq)]
enum SwipeAction {
    Open,
    Close,
}
impl LauncherSwipe {
    pub fn begin(&mut self, device: String, fingers: u32, allowed: bool, open: bool) {
        *self = Self {
            device: (fingers == 3 && allowed).then_some(device),
            x: 0.0,
            y: 0.0,
            open,
        };
    }
    pub fn update(&mut self, device: &str, dx: f64, dy: f64) {
        if self.device.as_deref() == Some(device) && dx.is_finite() && dy.is_finite() {
            self.x += dx;
            self.y += dy;
        }
    }
    fn end(&mut self, device: &str, cancelled: bool) -> Option<SwipeAction> {
        if self.device.as_deref() != Some(device) {
            return None;
        }
        let distance = if self.open { self.y } else { -self.y };
        let trigger = !cancelled && distance >= 70.0 && distance > self.x.abs() * 1.4;
        let action = if self.open {
            SwipeAction::Close
        } else {
            SwipeAction::Open
        };
        *self = Self::default();
        trigger.then_some(action)
    }
}
pub(crate) fn swipe_begin(state: &mut State, device: String, fingers: u32) {
    let allowed = matches!(state.interaction, Interaction::None)
        && !state.pointer.is_grabbed()
        && state.forwarded_buttons.is_empty();
    state
        .launcher_swipe
        .begin(device, fingers, allowed, state.shell.launcher_open);
}
pub(crate) fn swipe_end(state: &mut State, device: &str, cancelled: bool) {
    let action = state.launcher_swipe.end(device, cancelled);
    if !matches!(state.interaction, Interaction::None) || state.pointer.is_grabbed() {
        return;
    }
    match action {
        Some(SwipeAction::Close) if state.shell.launcher_open => close_launcher(state),
        Some(SwipeAction::Open) if !state.shell.launcher_open => {
            let (w, h) = (state.display.size.w as f32, state.display.size.h as f32);
            let origin = state.shell.dock_item_rect(w, h, dock_item_count(state), 0);
            open_launcher(state, origin);
        }
        _ => {}
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn only_complete_three_finger_up_swipes_open_launcher_once() {
        let mut swipe = LauncherSwipe::default();
        for (fingers, dx, dy, cancelled, allowed, want) in [
            (3, 5.0, -100.0, false, true, true),
            (2, 0.0, -120.0, false, true, false),
            (4, 0.0, -120.0, false, true, false),
            (3, 0.0, -30.0, false, true, false),
            (3, 110.0, -80.0, false, true, false),
            (3, 0.0, 120.0, false, true, false),
            (3, 0.0, -120.0, true, true, false),
            (3, 0.0, -120.0, false, false, false),
        ] {
            swipe.begin("touchpad".into(), fingers, allowed, false);
            swipe.update("touchpad", dx, dy);
            assert_eq!(swipe.end("touchpad", cancelled).is_some(), want);
            assert!(swipe.end("touchpad", false).is_none());
        }
    }
    #[test]
    fn events_from_another_device_cannot_complete_or_cancel_a_swipe() {
        let mut swipe = LauncherSwipe::default();
        swipe.begin("a".into(), 3, true, false);
        swipe.update("b", 0.0, -200.0);
        assert!(swipe.end("b", false).is_none());
        swipe.update("a", 0.0, -100.0);
        assert!(swipe.end("a", false).is_some());
    }
}

#[cfg(test)]
mod closing_tests {
    use super::*;
    #[test]
    fn open_launcher_accepts_only_downward_completed_three_finger_swipes() {
        let mut swipe = LauncherSwipe::default();
        for (dx, dy, cancelled, want) in [
            (3.0, 100.0, false, true),
            (0.0, -100.0, false, false),
            (100.0, 80.0, false, false),
            (0.0, 20.0, false, false),
            (0.0, 100.0, true, false),
        ] {
            swipe.begin("touchpad".into(), 3, true, true);
            swipe.update("touchpad", dx, dy);
            assert_eq!(
                swipe.end("touchpad", cancelled),
                want.then_some(SwipeAction::Close)
            );
            assert!(swipe.end("touchpad", false).is_none());
        }
    }
}
