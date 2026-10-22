//! Context-owned three-finger drawer/shade and four-finger recents gestures.
use super::*;
#[derive(Default)]
pub(crate) struct LauncherSwipe {
    device: Option<String>,
    x: f64,
    y: f64,
    fingers: u32,
    context: Option<SwipeContext>,
}
#[derive(Debug, PartialEq, Eq)]
enum SwipeAction {
    ThreeUp,
    ThreeDown,
    FourUp,
    FourDown,
}
impl LauncherSwipe {
    pub fn begin(&mut self, device: String, fingers: u32, allowed: bool) {
        *self = Self {
            device: (matches!(fingers, 3 | 4) && allowed).then_some(device),
            fingers,
            context: None,
            x: 0.0,
            y: 0.0,
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
        let distance = self.y.abs();
        let trigger = !cancelled
            && distance >= f64::from(config::design().input.touchpad.swipe_distance)
            && distance > self.x.abs() * 1.4;
        let action = match (self.fingers, self.y < 0.0) {
            (3, true) => SwipeAction::ThreeUp,
            (3, false) => SwipeAction::ThreeDown,
            (4, true) => SwipeAction::FourUp,
            _ => SwipeAction::FourDown,
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
        if state.shell.app_menu.is_some() {
            Self::Transition
        } else if state.shell.calendar.open || state.shell.power_open || state.shell.brightness_open
        {
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
        (C::Desktop, A::ThreeUp) => Some(E::OpenLauncher),
        (C::Desktop, A::ThreeDown) => Some(E::OpenShade),
        (C::Desktop, A::FourUp) => Some(E::OpenOverview),
        (C::Launcher, A::ThreeDown) => Some(E::CloseLauncher),
        (C::Overview, A::FourDown) => Some(E::CloseOverview),
        (C::Shade, A::ThreeUp) => Some(E::CloseShade),
        (C::SystemPanel, A::ThreeUp) => Some(E::CloseSystemPanel),
        _ => None,
    }
}
pub(crate) fn swipe_begin(state: &mut State, device: String, fingers: u32) {
    state.super_tap.cancel();
    super::stop_touchpad_scroll(state);
    let context = SwipeContext::current(state);
    let allowed = context != SwipeContext::Transition
        && matches!(state.interaction, Interaction::None)
        && !state.pointer.is_grabbed()
        && state.forwarded_buttons.is_empty()
        && state.swallowed_buttons.is_empty()
        && state.shortcut_drag.is_none();
    state.launcher_swipe.begin(device, fingers, allowed);
    state.launcher_swipe.context = Some(context);
}
pub(crate) fn swipe_end(state: &mut State, device: &str, cancelled: bool) {
    let context = state.launcher_swipe.context;
    let action = state.launcher_swipe.end(device, cancelled);
    if context != Some(SwipeContext::current(state))
        || !matches!(state.interaction, Interaction::None)
        || state.pointer.is_grabbed()
        || !state.forwarded_buttons.is_empty()
        || !state.swallowed_buttons.is_empty()
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
    fn completed_direction_is_independent_of_the_visible_layer() {
        for (fingers, x, y, cancelled, allowed, expected) in [
            (3, 0.0, -100.0, false, true, Some(SwipeAction::ThreeUp)),
            (3, 0.0, 100.0, false, true, Some(SwipeAction::ThreeDown)),
            (4, 0.0, -100.0, false, true, Some(SwipeAction::FourUp)),
            (4, 0.0, 100.0, false, true, Some(SwipeAction::FourDown)),
            (2, 0.0, -100.0, false, true, None),
            (5, 0.0, -100.0, false, true, None),
            (3, 100.0, -80.0, false, true, None),
            (3, 0.0, -30.0, false, true, None),
            (3, 0.0, -100.0, true, true, None),
            (3, 0.0, -100.0, false, false, None),
        ] {
            let mut swipe = LauncherSwipe::default();
            swipe.begin("pad".into(), fingers, allowed);
            swipe.update("pad", x, y);
            assert_eq!(swipe.end("pad", cancelled), expected);
            assert_eq!(swipe.end("pad", false), None);
        }
    }
    #[test]
    fn events_from_another_device_cannot_complete_or_cancel_a_swipe() {
        let mut swipe = LauncherSwipe::default();
        swipe.begin("a".into(), 3, true);
        swipe.update("b", 0.0, -200.0);
        assert!(swipe.end("b", false).is_none());
        swipe.update("a", 0.0, -100.0);
        assert_eq!(swipe.end("a", false), Some(SwipeAction::ThreeUp));
    }
    #[test]
    fn every_context_owns_its_gestures_and_four_down_never_opens_the_shade() {
        use SwipeAction as A;
        use SwipeContext as C;
        use SwipeEffect as E;
        let cases = [
            (
                C::Desktop,
                [
                    Some(E::OpenLauncher),
                    Some(E::OpenShade),
                    Some(E::OpenOverview),
                    None,
                ],
            ),
            (C::Launcher, [None, Some(E::CloseLauncher), None, None]),
            (C::Shade, [Some(E::CloseShade), None, None, None]),
            (C::Overview, [None, None, None, Some(E::CloseOverview)]),
            (
                C::SystemPanel,
                [Some(E::CloseSystemPanel), None, None, None],
            ),
            (C::Transition, [None, None, None, None]),
        ];
        for (context, expected) in cases {
            for (action, expected) in [A::ThreeUp, A::ThreeDown, A::FourUp, A::FourDown]
                .into_iter()
                .zip(expected)
            {
                assert_eq!(effect(context, action), expected, "{context:?}");
            }
        }
    }
}
