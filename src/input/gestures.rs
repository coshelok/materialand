//! Three-finger app drawer and four-finger overview/system-shade gestures.
use super::*;
#[derive(Default)]
pub(crate) struct LauncherSwipe {
    device: Option<String>,
    x: f64,
    y: f64,
    open: bool,
    fingers: u32,
    context: Option<SwipeContext>,
}
#[derive(Debug, PartialEq, Eq)]
enum SwipeAction {
    Open,
    Close,
    Overview,
    Shade,
}
impl LauncherSwipe {
    pub fn begin(&mut self, device: String, fingers: u32, allowed: bool, open: bool) {
        *self = Self {
            device: (matches!(fingers, 3 | 4) && allowed).then_some(device),
            fingers,
            context: None,
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
        let distance = if self.fingers == 4 {
            self.y.abs()
        } else if self.open {
            self.y
        } else {
            -self.y
        };
        let trigger = !cancelled && distance >= 70.0 && distance > self.x.abs() * 1.4;
        let action = if self.fingers == 4 {
            if self.y < 0.0 {
                SwipeAction::Overview
            } else {
                SwipeAction::Shade
            }
        } else if self.open {
            SwipeAction::Close
        } else {
            SwipeAction::Open
        };
        *self = Self::default();
        trigger.then_some(action)
    }
}
// A gesture belongs to the layer visible when it began. Its inverse dismisses
// that layer; it must never fall through to another global navigation action.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SwipeContext {
    Desktop,
    Launcher,
    Overview,
    Shade,
    SystemPanel,
    Transition,
}
impl SwipeContext {
    fn current(state: &State) -> Self {
        if state.shell.calendar.open || state.shell.power_open || state.shell.brightness_open {
            Self::SystemPanel
        } else if state.shell.quick_settings_open {
            Self::Shade
        } else if state.overview.open {
            Self::Overview
        } else if state.shell.launcher_open {
            Self::Launcher
        } else if state.overview.visible()
            || state.shell.quick_settings_progress() > 0.001
            || state.shell.launcher_progress() > 0.001
            || state.shell.calendar_progress() > 0.001
            || state.shell.power_progress() > 0.001
            || state.shell.brightness_progress() > 0.001
        {
            Self::Transition
        } else {
            Self::Desktop
        }
    }
}
#[derive(Debug, PartialEq, Eq)]
enum SwipeEffect {
    OpenLauncher,
    CloseLauncher,
    OpenOverview,
    CloseOverview,
    OpenShade,
    CloseShade,
    CloseSystemPanel,
}
fn effect(context: SwipeContext, action: SwipeAction) -> Option<SwipeEffect> {
    use SwipeAction as A;
    use SwipeContext as C;
    use SwipeEffect as E;
    match (context, action) {
        (C::Desktop, A::Open) => Some(E::OpenLauncher),
        (C::Desktop, A::Overview) => Some(E::OpenOverview),
        (C::Desktop, A::Shade) => Some(E::OpenShade),
        (C::Launcher, A::Close) => Some(E::CloseLauncher),
        (C::Overview, A::Shade) => Some(E::CloseOverview),
        (C::Shade, A::Overview) => Some(E::CloseShade),
        (C::SystemPanel, A::Overview) => Some(E::CloseSystemPanel),
        _ => None,
    }
}
pub(crate) fn swipe_begin(state: &mut State, device: String, fingers: u32) {
    let context = SwipeContext::current(state);
    let allowed = context != SwipeContext::Transition
        && matches!(state.interaction, Interaction::None)
        && !state.pointer.is_grabbed()
        && state.forwarded_buttons.is_empty();
    state
        .launcher_swipe
        .begin(device, fingers, allowed, state.shell.launcher_open);
    state.launcher_swipe.context = Some(context);
}
pub(crate) fn swipe_end(state: &mut State, device: &str, cancelled: bool) {
    let context = state.launcher_swipe.context;
    let action = state.launcher_swipe.end(device, cancelled);
    if context != Some(SwipeContext::current(state))
        || !matches!(state.interaction, Interaction::None)
        || state.pointer.is_grabbed()
    {
        return;
    }
    let effect = context
        .zip(action)
        .and_then(|(context, action)| effect(context, action));
    match effect {
        Some(SwipeEffect::OpenOverview) => state.open_overview(),
        Some(SwipeEffect::CloseOverview) => state.overview.close(state.shell.reduce_motion),
        Some(SwipeEffect::OpenShade) => super::navigation::open_shade(state, true),
        Some(SwipeEffect::CloseShade) => super::navigation::open_shade(state, false),
        Some(SwipeEffect::CloseSystemPanel) => {
            state.shell.set_calendar_open(false);
            state.shell.set_power_open(false);
            state.shell.set_brightness_open(false);
        }
        Some(SwipeEffect::CloseLauncher) => close_launcher(state),
        Some(SwipeEffect::OpenLauncher) => {
            let (w, h) = (state.display.size.w as f32, state.display.size.h as f32);
            let origin = if shell::tablet_mode() {
                shell::navigation_rect(w, h)
            } else {
                state.shell.dock_item_rect(w, h, dock_item_count(state), 0)
            };
            open_launcher(state, origin);
        }
        None => {}
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
            assert_eq!(
                swipe.end("touchpad", cancelled) == Some(SwipeAction::Open),
                want
            );
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

#[cfg(test)]
mod four_finger_tests {
    use super::*;
    #[test]
    fn overview_and_shade_have_distinct_four_finger_directions() {
        for (fingers, dx, dy, cancelled, want) in [
            (4, 0.0, -100.0, false, Some(SwipeAction::Overview)),
            (4, 0.0, 100.0, false, Some(SwipeAction::Shade)),
            (4, 100.0, -80.0, false, None),
            (4, 0.0, -30.0, false, None),
            (4, 0.0, -100.0, true, None),
            (5, 0.0, -100.0, false, None),
        ] {
            let mut s = LauncherSwipe::default();
            s.begin("pad".into(), fingers, true, false);
            s.update("pad", dx, dy);
            assert_eq!(s.end("pad", cancelled), want);
            assert_eq!(s.end("pad", false), None);
        }
    }
}

#[cfg(test)]
mod routing_tests {
    use super::*;
    #[test]
    fn inverse_gestures_close_the_current_layer_without_opening_another() {
        use SwipeAction as A;
        use SwipeContext as C;
        use SwipeEffect as E;
        assert_eq!(effect(C::Shade, A::Overview), Some(E::CloseShade));
        assert_eq!(effect(C::Overview, A::Shade), Some(E::CloseOverview));
        assert_eq!(effect(C::Launcher, A::Close), Some(E::CloseLauncher));
        assert_eq!(
            effect(C::SystemPanel, A::Overview),
            Some(E::CloseSystemPanel)
        );
        for context in [C::Shade, C::Overview, C::SystemPanel, C::Transition] {
            assert_eq!(effect(context, A::Open), None);
        }
        assert_eq!(effect(C::Shade, A::Shade), None);
        assert_eq!(effect(C::Overview, A::Overview), None);
        assert_eq!(effect(C::Launcher, A::Overview), None);
        assert_eq!(effect(C::Launcher, A::Shade), None);
    }
}
