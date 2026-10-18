//! Actions.
use super::*;
pub fn open_launcher(state: &mut State, origin: shell::RectF) {
    state.shell.launcher_open = true;
    state.shell.launcher_query.clear();
    state.shell.brightness_open = false;
    state.shell.set_brightness_open(false);
    state.shell.power_open = false;
    state.shell.set_power_open(false);
    state.shell.set_quick_settings_open(false);
    state.shell.set_launcher_origin(origin);
    state.shell.set_launcher_open(true);
    // Rasterise the overlay's cached layer now, so its open animation runs at
    // full speed instead of stalling on the first frame.
    let (w, h) = (state.display.size.w as f32, state.display.size.h as f32);
    let theme = state.theme;
    let _ = state
        .shell
        .render_launcher(&theme, &mut state.fonts, w, h, 1.0, None);
}

pub fn close_launcher(state: &mut State) {
    state.shell.launcher_open = false;
    state.shell.launcher_query.clear();
    state.shell.set_launcher_open(false);
}

pub(super) fn launch_first_filtered(state: &mut State) {
    let filtered = state.shell.filtered_apps();
    if let Some(&idx) = filtered.first() {
        let cmd = state.shell.apps[idx].command_in_terminal(&state.shell.terminal);
        spawn_program(&cmd);
    }
    close_launcher(state);
}

pub(crate) fn finish_compositor_press(state: &mut State, pos: Point<f64, Logical>) {
    let (w, h) = (state.display.size.w as f32, state.display.size.h as f32);
    let mut consumed = false;

    if let Some(action) = state.shell.press_top.take() {
        consumed = true;
        state.shell.press_end(PressKey::Top(action));
        if shell::topbar_hit(w, pos) == Some(action) {
            perform_top_action(state, action);
        }
    }

    if let Some(i) = state.shell.press_dock.take() {
        consumed = true;
        state.shell.press_end(PressKey::Dock(i));
        let items = dock_items(state);
        if shell::dock_hit(w, h, items.len(), pos) == Some(i) {
            let serial = state.next_serial();
            match items.get(i).cloned() {
                Some(DockItem::Launcher) => {
                    let origin = shell::dock_item_rect(w, h, items.len(), i);
                    open_launcher(state, origin);
                }
                Some(DockItem::Launch(app)) => {
                    let cmd = app.command_in_terminal(&state.shell.terminal);
                    spawn_program(&cmd);
                }
                Some(DockItem::Focus { window, .. }) => {
                    if state.window_props(&window).minimized {
                        state.restore_window(&window, serial);
                    } else {
                        // Raise without activating: `focus_window` owns
                        // activation and is the one that sends the configure.
                        state.windows.space.raise_element(&window, false);
                        state.focus_window(Some(window), serial);
                    }
                }
                None => {}
            }
        }
    }

    if let Some(action) = state.shell.press_tile.take() {
        consumed = true;
        let i = shell::quick_action_index(action);
        state.shell.press_end(PressKey::Tile(i));
        if state.shell.quick_settings_open
            && shell::quick_settings_hit(w, h, pos, state.shell.quick_settings_progress())
                == Some(action)
        {
            perform_quick_action(state, action);
        }
    }

    if let Some(i) = state.shell.press_chip.take() {
        consumed = true;
        state.shell.press_end(PressKey::Chip(i));
        if state.shell.quick_settings_open && shell::palette_chip_hit(w, h, pos) == Some(i) {
            state.set_variant(VARIANTS[i]);
        }
    }

    if let Some(slot) = state.shell.press_launch.take() {
        consumed = true;
        state.shell.press_end(PressKey::Launcher(slot));
        if state.shell.launcher_open {
            let filtered = state.shell.filtered_apps();
            if shell::launcher_hit(w, h, filtered.len(), pos) == LauncherHit::Tile(slot) {
                if let Some(&idx) = filtered.get(slot) {
                    let cmd = state.shell.apps[idx].command_in_terminal(&state.shell.terminal);
                    spawn_program(&cmd);
                }
                close_launcher(state);
            }
        }
    }

    if let Some(i) = state.shell.press_power.take() {
        consumed = true;
        state.shell.press_end(PressKey::Power(i));
        if state.shell.power_open && shell::power_row_at(w, pos) == Some(i) {
            if let Some(action) = shell::POWER_ACTIONS.get(i).copied() {
                perform_power_action(state, action);
            }
        }
    }

    if !consumed && state.shell.launcher_open {
        // A click on the scrim (or outside the tiles) dismisses the launcher.
        let filtered = state.shell.filtered_apps();
        if shell::launcher_hit(w, h, filtered.len(), pos) == LauncherHit::Scrim {
            close_launcher(state);
        }
    }
}

pub(crate) fn perform_top_action(state: &mut State, action: TopAction) {
    let w = state.display.size.w as f32;
    match action {
        TopAction::Launcher => {
            open_launcher(state, shell::topbar_button_rect(w, TopAction::Launcher));
        }
        TopAction::Brightness => {
            state.shell.power_open = false;
            state.shell.set_power_open(false);
            state.shell.brightness_open = !state.shell.brightness_open;
            let open = state.shell.brightness_open;
            state.shell.set_brightness_open(open);
        }
        TopAction::Theme => {
            // Radial reveal grows out of the button the user pressed.
            let rect = shell::topbar_button_rect(w, TopAction::Theme);
            state.toggle_theme(Point::from((
                rect.x as f64 + rect.w as f64 / 2.0,
                rect.y as f64 + rect.h as f64 / 2.0,
            )));
        }
        TopAction::QuickSettings => {
            let open = !state.shell.quick_settings_open;
            state.shell.brightness_open = false;
            state.shell.set_brightness_open(false);
            state.shell.power_open = false;
            state.shell.set_power_open(false);
            state
                .shell
                .set_qs_origin(shell::topbar_button_rect(w, TopAction::QuickSettings));
            state.shell.set_quick_settings_open(open);
        }
        TopAction::Power => {
            // The user button opens the session menu; quitting the compositor
            // is now a deliberate choice inside that menu.
            let open = !state.shell.power_open;
            state.shell.brightness_open = false;
            state.shell.set_brightness_open(false);
            state.shell.set_quick_settings_open(false);
            state.shell.power_open = open;
            state.shell.set_power_open(open);
        }
    }
}

pub(crate) fn perform_power_action(state: &mut State, action: shell::PowerAction) {
    if !state.session_mode && !matches!(action, shell::PowerAction::Logout) {
        return;
    }
    state.shell.power_open = false;
    state.shell.set_power_open(false);
    match action {
        shell::PowerAction::PowerOff => spawn_system(&["systemctl", "poweroff"]),
        shell::PowerAction::Restart => spawn_system(&["systemctl", "reboot"]),
        shell::PowerAction::Logout => state.running = false,
    }
}

pub(super) fn perform_quick_action(state: &mut State, action: QuickAction) {
    match action {
        QuickAction::Theme => {
            // Radial reveal grows out of the tile the user pressed.
            let (w, h) = (state.display.size.w as f32, state.display.size.h as f32);
            let tile = shell::quick_setting_tile_rect(w, h, shell::quick_action_index(action));
            state.toggle_theme(Point::from((
                tile.x as f64 + tile.w as f64 / 2.0,
                tile.y as f64 + tile.h as f64 / 2.0,
            )));
        }
        QuickAction::NightLight => state.shell.toggle_night_light(),
        QuickAction::ReduceMotion => {
            let enabled = !state.shell.reduce_motion;
            state.shell.set_reduce_motion(enabled);
        }
        QuickAction::Brightness => {
            state.shell.set_quick_settings_open(false);
            state.shell.brightness_open = true;
            state.shell.set_brightness_open(true);
        }
    }
}
