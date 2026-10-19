//! Scroll frames retain source, high-resolution deltas and finger-stop events.
use super::*;
use smithay::{
    backend::input::{Axis, AxisSource, Event, PointerAxisEvent},
    input::pointer::AxisFrame,
};
pub fn on_pointer_axis<B: InputBackend>(state: &mut State, event: B::PointerAxisEvent) {
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
            let delta = if matches!(event.source(), AxisSource::Wheel | AxisSource::WheelTilt) {
                event
                    .amount_v120(Axis::Vertical)
                    .map(|v| v * 84.0 / 120.0)
                    .unwrap_or_else(|| event.amount(Axis::Vertical).unwrap_or(0.0) * 4.0)
            } else {
                event.amount(Axis::Vertical).unwrap_or(0.0)
            };
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
        let discrete = event.amount_v120(axis);
        let amount = event
            .amount(axis)
            .unwrap_or_else(|| discrete.unwrap_or(0.0) * 15.0 / 120.0);
        if amount != 0.0 {
            frame = frame
                .value(axis, amount)
                .relative_direction(axis, event.relative_direction(axis));
            if let Some(v) = discrete {
                frame = frame.v120(axis, v as i32);
            }
        } else if event.source() == AxisSource::Finger && event.amount(axis) == Some(0.0) {
            frame = frame.stop(axis);
        }
    }
    let pointer = state.pointer.clone();
    pointer.axis(state, frame);
    pointer.frame(state);
}
