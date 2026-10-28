//! A seat pause releases transient input state before dropping its devices.
use super::*;
// Shell entry must release a client's pointer grab without re-entering the
// keyboard filter that may have requested the overlay.
pub(crate) fn prepare_shell_overlay(state: &mut State) {
    super::cancel_swipe(state);
    state.clear_activation_input();
    super::touchpad_scroll::interrupt(state);
    state.overview.cancel_press(state.shell.reduce_motion);
    super::shortcut_drag::cancel(state);
    if let Some(button) = state.interaction.button() {
        state.finish_interaction(button);
    }
    let time = smithay::utils::Clock::<smithay::utils::Monotonic>::new()
        .now()
        .as_millis();
    let pointer = state.pointer.clone();
    for button in state.forwarded_buttons.drain().collect::<Vec<_>>() {
        state.swallowed_buttons.insert(button);
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
    state.last_interactive_press = None;
    // A release belonging to the old layer must not dismiss the new launcher
    // through its background/scrim fallback after cancel_presses cleared its owner.
    state.shell.consume_menu_release |= state.swallowed_buttons.contains(&BTN_LEFT);
    state.shell.cancel_presses();
}
pub fn cancel(state: &mut State) {
    state.clear_activation_input();
    state.super_tap = Default::default();
    super::stop_touchpad_scroll(state);
    state.overview.cancel_press(state.shell.reduce_motion);
    super::cancel_swipe(state);
    super::shortcut_drag::cancel(state);
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
    state.shell.consume_menu_release = false;
    state.last_interactive_press = None;
    state.shell.cancel_presses();
    state.shell.close_app_menu();
    state.shell.set_calendar_open(false);
}
