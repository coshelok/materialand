//! Pointer delivery: global origins, implicit grabs, matched presses/releases.
use super::target::{target, Target};
use super::*;
use crate::desktop::{hit_test::WindowHit, resize::ResizePlacement};

pub fn on_pointer_motion_relative(state: &mut State, dx: f64, dy: f64, time: u32) {
    on_pointer_motion(
        state,
        (state.cursor.x + dx, state.cursor.y + dy).into(),
        time,
    );
}
pub fn on_pointer_motion(state: &mut State, location: Point<f64, Logical>, time: u32) {
    let location = Point::from((
        location
            .x
            .clamp(0.0, f64::from((state.display.size.w - 1).max(0))),
        location
            .y
            .clamp(0.0, f64::from((state.display.size.h - 1).max(0))),
    ));
    state.cursor = location;
    let (w, h) = (state.display.size.w as f32, state.display.size.h as f32);
    state.shell.set_hover_top(if state.shell.launcher_open {
        None
    } else {
        shell::topbar_hit(w, location)
    });
    state.shell.set_hover_dock(if state.shell.launcher_open {
        None
    } else {
        state.shell.dock_hit(w, h, dock_item_count(state), location)
    });
    state.shell.set_hover_power(if state.shell.power_open {
        shell::power_row_at(
            w,
            state
                .shell
                .power_placement(w, state.shell.power_progress())
                .unmap(location),
        )
    } else {
        None
    });
    match state.interaction.clone() {
        Interaction::Move {
            button: _,
            window,
            start_pointer,
            start_location,
        } => {
            let delta = (location - start_pointer).to_i32_round();
            state
                .windows
                .space
                .map_element(window, start_location + delta, false);
            state.windows.space.refresh();
            return;
        }
        Interaction::Resize {
            button: _,
            window,
            edges,
            start_pointer,
            start_geometry,
        } => {
            let delta = (location - start_pointer).to_i32_round();
            let (_, size) = compute_resize(start_geometry, edges, delta.x, delta.y);
            let requested = state.constrained_size(&window, size.into());
            if state
                .windows
                .pending_resize
                .get(&window)
                .is_none_or(|p| p.requested != requested)
            {
                if let Some(toplevel) = window.toplevel() {
                    toplevel.with_pending_state(|pending| pending.size = Some(requested));
                    toplevel.send_configure();
                    state.windows.pending_resize.insert(
                        window,
                        ResizePlacement {
                            requested,
                            original: start_geometry,
                            edges,
                        },
                    );
                }
            }
            return;
        }
        Interaction::Brightness => {
            if let Some(value) = shell::brightness_hit(
                w,
                state
                    .shell
                    .brightness_placement(w, state.shell.brightness_progress())
                    .unmap(location),
            ) {
                state.brightness = value;
            }
            return;
        }
        Interaction::None => {}
    }
    refresh_focus(state, time);
}
fn refresh_focus(state: &mut State, time: u32) {
    let focus = match target(state, state.cursor) {
        Target::Window(WindowHit::Client {
            surface, origin, ..
        }) => Some((surface, origin)),
        _ => None,
    };
    let event = MotionEvent {
        location: state.cursor,
        serial: state.next_serial(),
        time,
    };
    let pointer = state.pointer.clone();
    pointer.motion(state, focus, &event);
    pointer.frame(state);
}
pub fn on_pointer_button<B: InputBackend, E: PointerButtonEvent<B>>(state: &mut State, event: &E) {
    let button = event.button_code();
    let pressed = event.state() == ButtonState::Pressed;
    let time = event.time_msec();
    if !pressed {
        state.finish_interaction(button);
        state.last_interactive_press = None;
        if state.swallowed_buttons.remove(&button) {
            if button == BTN_LEFT {
                finish_compositor_press(state, state.cursor);
                refresh_focus(state, time);
            }
            return;
        }
        if !state.forwarded_buttons.remove(&button) {
            return;
        }
        let event = ButtonEvent {
            serial: state.next_serial(),
            time,
            button,
            state: ButtonState::Released,
        };
        let pointer = state.pointer.clone();
        pointer.button(state, &event);
        pointer.frame(state);
        refresh_focus(state, time);
        return;
    }
    if !matches!(state.interaction, Interaction::None) {
        state.swallowed_buttons.insert(button);
        return;
    }
    // An active client grab owns additional button presses as well as motion.
    if state.pointer.is_grabbed() {
        state.forwarded_buttons.insert(button);
        let event = ButtonEvent {
            serial: state.next_serial(),
            time,
            button,
            state: ButtonState::Pressed,
        };
        let pointer = state.pointer.clone();
        pointer.button(state, &event);
        pointer.frame(state);
        return;
    }
    if let Some(menu) = &state.shell.app_menu {
        let rect = shell::app_menu_rect(
            state.display.size.w as f32,
            state.display.size.h as f32,
            menu.point,
        );
        if !rect.contains(state.cursor) {
            state.shell.close_app_menu();
            state.shell.consume_menu_release = button == BTN_LEFT;
            state.swallowed_buttons.insert(button);
            return;
        }
    }
    super::target::dismiss_outside(state, state.cursor);
    let serial = state.next_serial();
    state.last_interactive_press = None;
    match target(state, state.cursor) {
        Target::Window(WindowHit::Client {
            window,
            surface,
            origin,
        }) => {
            state.windows.space.raise_element(&window, false);
            state.focus_window(Some(window.clone()), serial);
            state.last_interactive_press = Some((serial, window));
            state.forwarded_buttons.insert(button);
            let pointer = state.pointer.clone();
            pointer.motion(
                state,
                Some((surface, origin)),
                &MotionEvent {
                    location: state.cursor,
                    serial,
                    time,
                },
            );
            pointer.button(
                state,
                &ButtonEvent {
                    serial,
                    time,
                    button,
                    state: ButtonState::Pressed,
                },
            );
            pointer.frame(state);
        }
        Target::Window(WindowHit::Decoration { window, geometry }) => {
            state.swallowed_buttons.insert(button);
            if button != BTN_LEFT {
                return;
            }
            state.windows.space.raise_element(&window, false);
            state.focus_window(Some(window.clone()), serial);
            match shell::decor_hit(&shell::decor_layout(geometry), state.cursor) {
                DecorHit::Close => state.request_close_window(&window),
                DecorHit::Minimize => state.minimize_window(&window),
                DecorHit::Maximize => state.toggle_maximize(&window),
                DecorHit::Move => {
                    state.interaction = Interaction::Move {
                        button: BTN_LEFT,
                        window,
                        start_pointer: state.cursor,
                        start_location: geometry.loc,
                    }
                }
                DecorHit::Resize(edges) => {
                    state.begin_resize(window, edges, geometry, BTN_LEFT);
                }
                DecorHit::None => {}
            }
        }
        Target::Shell(control) => {
            state.swallowed_buttons.insert(button);
            if button == BTN_LEFT {
                super::target::press(state, control, state.cursor);
            } else if button == 0x111 {
                super::app_menu::open(state, control, state.cursor);
            }
        }
        Target::Desktop => {
            state.swallowed_buttons.insert(button);
            if button == BTN_LEFT {
                state.focus_window(None, serial);
            }
        }
    }
}
