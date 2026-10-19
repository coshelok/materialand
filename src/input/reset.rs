//! A seat pause releases transient input state before dropping its devices.
use super::*;
pub fn cancel(state: &mut State) {
    let time = smithay::utils::Clock::<smithay::utils::Monotonic>::new()
        .now()
        .as_millis();
    if let Some(button) = state.interaction.button() {
        state.finish_interaction(button);
    }
    let keyboard = state.keyboard.clone();
    keyboard.unset_grab(state);
    for code in keyboard.pressed_keys() {
        let serial = state.next_serial();
        keyboard.input::<(), _>(state, code, KeyState::Released, serial, time, |_, _, _| {
            FilterResult::Forward
        });
    }
    let pointer = state.pointer.clone();
    for button in state.forwarded_buttons.drain().collect::<Vec<_>>() {
        let serial = state.next_serial();
        pointer.button(
            state,
            &ButtonEvent {
                serial,
                time,
                button,
                state: ButtonState::Released,
            },
        );
    }
    let serial = state.next_serial();
    pointer.unset_grab(state, serial, time);
    pointer.motion(
        state,
        None,
        &MotionEvent {
            location: state.cursor,
            serial,
            time,
        },
    );
    pointer.frame(state);
    state.swallowed_keys.clear();
    state.swallowed_buttons.clear();
    state.last_interactive_press = None;
    state.shell.cancel_presses();
    state.shell.close_app_menu();
    state.shell.set_calendar_open(false);
}
