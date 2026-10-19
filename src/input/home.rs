//! Returning home suspends windows; opening an existing app restores its slot.
use super::*;
pub fn show_home(state: &mut State) {
    state.shell.cancel_presses();
    if let Some(button) = state.interaction.button() {
        state.finish_interaction(button);
    }
    close_launcher(state);
    state.shell.set_calendar_open(false);
    state.shell.set_brightness_open(false);
    state.shell.set_quick_settings_open(false);
    state.shell.set_power_open(false);
    let windows: Vec<_> = state.windows.space.elements().cloned().collect();
    for window in windows {
        state.minimize_window(&window);
    }
    state.shell.reveal_home();
}
pub(super) fn launch_or_focus(
    state: &mut State,
    app: &config::AppEntry,
    origin: Option<shell::RectF>,
) {
    let existing = dock_items(state).into_iter().find_map(|item| match item {
        DockItem::Focus {
            window,
            app: Some(known),
            ..
        } if known.identity() == app.identity() && !state.shell.is_closing(&window) => Some(window),
        _ => None,
    });
    if let Some(window) = existing {
        let serial = state.next_serial();
        if state.window_props(&window).minimized {
            state.restore_window(&window, serial);
            if let Some(origin) = origin {
                state.shell.set_window_launch_origin(&window, origin);
            }
        } else {
            state.windows.space.raise_element(&window, false);
            state.focus_window(Some(window), serial);
        }
    } else {
        if let Some(origin) = origin {
            state.shell.remember_app_launch(app, origin);
        }
        spawn_program(&app.command_in_terminal(&state.shell.terminal));
    }
}

pub(super) fn launcher_launch_origin(state: &State, slot: usize) -> Option<shell::RectF> {
    let (w, h) = (state.display.size.w as f32, state.display.size.h as f32);
    let mut origin = shell::launcher_grid_origin(w, h);
    origin.1 -= state.shell.launcher_scroll();
    let (x, y) = shell::launcher_tile_pos(
        origin,
        slot,
        shell::launcher_columns(shell::launcher_card(w, h).w),
    );
    let icon = shell::RectF {
        x: x + 22.0,
        y: y + 2.0,
        w: 68.0,
        h: 68.0,
    };
    let viewport = shell::launcher_viewport(w, h);
    if icon.bottom() <= viewport.y || icon.y >= viewport.bottom() {
        return None;
    }
    Some(
        state
            .shell
            .launcher_placement(w, h, state.shell.launcher_progress())
            .map_rect(icon),
    )
}
