//! Scroll frames retain source, high-resolution deltas and finger-stop events.
use super::*;
use smithay::{
    backend::input::{Axis, AxisRelativeDirection, AxisSource, Event, PointerAxisEvent},
    input::pointer::AxisFrame,
};
pub fn on_pointer_axis<B: InputBackend>(state: &mut State, event: B::PointerAxisEvent) {
    let invert = crate::config::design().input.touchpad.invert_scroll;
    if state.shell.app_menu.is_some() {
        return;
    }
    if state.shell.launcher_open && !state.pointer.is_grabbed() {
        let (w, h) = (state.display.size.w as f32, state.display.size.h as f32);
        let pos = state
            .shell
            .launcher_placement(w, h, state.shell.launcher_progress())
            .unmap(state.cursor);
        if shell::launcher_viewport(w, h).contains(pos) {
            let delta = launcher_delta(
                event.source(),
                event.amount(Axis::Vertical),
                event.amount_v120(Axis::Vertical),
                invert,
            );
            state.shell.scroll_launcher(w, h, delta as f32);
        }
        return;
    }
    if !matches!(
        super::target::target(state, state.cursor),
        super::target::Target::Window(crate::desktop::hit_test::WindowHit::Client { .. })
    ) && !state.pointer.is_grabbed()
    {
        return;
    }
    let mut frame = AxisFrame::new(event.time_msec()).source(event.source());
    for axis in [Axis::Horizontal, Axis::Vertical] {
        frame = append_axis(
            frame,
            axis,
            event.source(),
            event.amount(axis),
            event.amount_v120(axis),
            event.relative_direction(axis),
            invert,
        );
    }
    let pointer = state.pointer.clone();
    pointer.axis(state, frame);
    pointer.frame(state);
}

fn scroll_sign(source: AxisSource, invert: bool) -> f64 {
    if invert && source == AxisSource::Finger {
        -1.0
    } else {
        1.0
    }
}

fn launcher_delta(
    source: AxisSource,
    amount: Option<f64>,
    discrete: Option<f64>,
    invert: bool,
) -> f64 {
    let delta = if matches!(source, AxisSource::Wheel | AxisSource::WheelTilt) {
        discrete
            .map(|value| value * 84.0 / 120.0)
            .unwrap_or_else(|| amount.unwrap_or(0.0) * 4.0)
    } else {
        amount.unwrap_or(0.0)
    };
    delta * scroll_sign(source, invert)
}

fn append_axis(
    mut frame: AxisFrame,
    axis: Axis,
    source: AxisSource,
    raw: Option<f64>,
    discrete: Option<f64>,
    direction: AxisRelativeDirection,
    invert: bool,
) -> AxisFrame {
    let sign = scroll_sign(source, invert);
    let amount = raw.unwrap_or_else(|| discrete.unwrap_or(0.0) * 15.0 / 120.0) * sign;
    if amount != 0.0 {
        let direction = if sign < 0.0 {
            match direction {
                AxisRelativeDirection::Identical => AxisRelativeDirection::Inverted,
                AxisRelativeDirection::Inverted => AxisRelativeDirection::Identical,
            }
        } else {
            direction
        };
        frame = frame
            .value(axis, amount)
            .relative_direction(axis, direction);
        if let Some(value) = discrete {
            frame = frame.v120(axis, (value * sign) as i32);
        }
    } else if source == AxisSource::Finger && raw == Some(0.0) {
        frame = frame.stop(axis);
    }
    frame
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn launcher_matches_client_scroll_direction_without_changing_wheel_speed() {
        assert_eq!(
            launcher_delta(AxisSource::Finger, Some(9.25), None, true),
            -9.25
        );
        assert_eq!(
            launcher_delta(AxisSource::Finger, Some(9.25), None, false),
            9.25
        );
        for source in [AxisSource::Wheel, AxisSource::WheelTilt] {
            assert_eq!(launcher_delta(source, None, Some(120.0), true), 84.0);
            assert_eq!(launcher_delta(source, Some(3.0), None, true), 12.0);
        }
    }
    #[test]
    fn inversion_applies_only_to_finger_scrolling_and_preserves_precision() {
        for source in [
            AxisSource::Finger,
            AxisSource::Wheel,
            AxisSource::WheelTilt,
            AxisSource::Continuous,
        ] {
            for invert in [false, true] {
                let frame = append_axis(
                    AxisFrame::new(42).source(source),
                    Axis::Vertical,
                    source,
                    Some(0.125),
                    Some(120.0),
                    AxisRelativeDirection::Identical,
                    invert,
                );
                let expected = if invert && source == AxisSource::Finger {
                    -1.0
                } else {
                    1.0
                };
                assert_eq!(frame.axis.1, 0.125 * expected);
                assert_eq!(frame.v120.unwrap().1, (120.0 * expected) as i32);
                assert_eq!(
                    frame.relative_direction.1,
                    if expected < 0.0 {
                        AxisRelativeDirection::Inverted
                    } else {
                        AxisRelativeDirection::Identical
                    }
                );
                assert_eq!(frame.source, Some(source));
                assert_eq!(frame.time, 42);
            }
        }
    }
    #[test]
    fn horizontal_motion_inverts_and_finger_stop_events_survive() {
        let frame = append_axis(
            AxisFrame::new(1),
            Axis::Horizontal,
            AxisSource::Finger,
            Some(-3.5),
            None,
            AxisRelativeDirection::Inverted,
            true,
        );
        assert_eq!(frame.axis.0, 3.5);
        assert_eq!(frame.relative_direction.0, AxisRelativeDirection::Identical);
        let frame = append_axis(
            frame,
            Axis::Vertical,
            AxisSource::Finger,
            Some(0.0),
            None,
            AxisRelativeDirection::Identical,
            true,
        );
        assert!(frame.stop.1);
        assert!(frame.v120.is_none());
        let absent = append_axis(
            AxisFrame::new(1),
            Axis::Vertical,
            AxisSource::Finger,
            None,
            None,
            AxisRelativeDirection::Identical,
            true,
        );
        assert!(!absent.stop.1);
    }
}
