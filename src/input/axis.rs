//! Scroll frames retain source, high-resolution deltas and finger-stop events.
use super::*;
use smithay::{
    backend::input::{Axis, AxisSource, Event, PointerAxisEvent},
    input::pointer::AxisFrame,
};
pub fn on_pointer_axis<B: InputBackend>(state: &mut State, event: B::PointerAxisEvent) {
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
