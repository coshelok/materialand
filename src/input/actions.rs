//! Actions.
use super::*;
pub fn open_launcher(state: &mut State, origin: shell::RectF) {
    super::prepare_shell_overlay(state);
    state.overview.close(state.shell.reduce_motion);
    state.shell.close_app_menu();
    state.shell.set_calendar_open(false);
    state.shell.launcher_open = true;
    state.shell.launcher_query.clear();
    state.shell.reset_launcher_scroll();
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
    state.shell.close_app_menu();
    state.shell.launcher_open = false;
    state.shell.launcher_query.clear();
    state.shell.reset_launcher_scroll();
    state.shell.set_launcher_open(false);
}

pub(super) fn launch_first_filtered(state: &mut State) {
    let filtered = state.shell.filtered_apps();
    if let Some(&idx) = filtered.first() {
        let app = state.shell.apps[idx].clone();
        let origin = launcher_launch_origin(state, 0);
        launch_or_focus(state, &app, origin);
    }
    close_launcher(state);
}

pub(crate) fn finish_compositor_press(state: &mut State, pos: Point<f64, Logical>) {
    if std::mem::take(&mut state.shell.consume_menu_release) {
        return;
    }
    if super::shortcut_drag::release(state, pos) {
        return;
    }
    if state.overview.open {
        if let Some(mut hit) = state.overview.pressed.take() {
            if state.overview.drag_y < -config::design().overview.dismiss_threshold {
                hit.action = crate::desktop::overview::OverviewAction::Close;
                state.overview_action(hit);
            } else if state.overview.drag_y.abs() < config::design().drag.threshold
                && state.overview.hit(
                    state.display.size.w as f32,
                    state.display.size.h as f32,
                    pos,
                ) == Some(hit)
            {
                state.overview_action(hit);
            } else {
                state
                    .overview
                    .cancel_drag(hit.index, state.shell.reduce_motion);
            }
        } else if std::mem::take(&mut state.overview.background_pressed) {
            state.overview.close(state.shell.reduce_motion);
        }
        state.overview.drag_y = 0.0;
        return;
    }
    let (w, h) = (state.display.size.w as f32, state.display.size.h as f32);
    if state.shell.app_menu.is_some() {
        let pressed = std::mem::take(&mut state.shell.press_pin);
        let home_pressed = state
            .shell
            .app_menu
            .as_mut()
            .is_some_and(|m| std::mem::take(&mut m.press_home));
        if let Some(menu) = &state.shell.app_menu {
            if pressed
                && shell::app_menu_button(shell::app_menu_rect(w, h, menu.point)).contains(pos)
            {
                state.shell.activate_pin_menu();
            } else if home_pressed
                && shell::app_menu_home_button(shell::app_menu_rect(w, h, menu.point)).contains(pos)
            {
                state.shell.activate_home_menu();
            }
        }
        return;
    }
    if let Some(action) = state.shell.calendar.pressed.take() {
        if state.shell.calendar.open
            && shell::calendar_hit(w, h, state.shell.calendar_placement(w, h).unmap(pos))
                == Some(action)
        {
            state.shell.calendar.navigate(action);
        }
        return;
    }
    if let Some(hit) = state.shell.home.pressed.take() {
        if let shell::HomeHit::App(i) = hit {
            state.shell.press_end(PressKey::Home(i));
        }
        if matches!(super::target::target(state,pos), super::target::Target::Shell(super::target::Control::Home(released)) if released == hit)
        {
            match hit {
                shell::HomeHit::App(i) => {
                    if let Some(app) = state.shell.home.apps.get(i).cloned() {
                        let tile = state.shell.home_tile_rect(w, h, i);
                        let icon = shell::app_icon_rect(tile);
                        launch_or_focus(state, &app, Some(icon));
                    }
                }
                shell::HomeHit::Previous => state.shell.turn_home_page(w, h, -1),
                shell::HomeHit::Next => state.shell.turn_home_page(w, h, 1),
                shell::HomeHit::Search => {
                    open_launcher(state, shell::home_layout(w, h).search);
                }
            }
        }
        return;
    }
    let mut consumed = false;
    let mut qs_pos = state
        .shell
        .quick_settings_placement(w, h, state.shell.quick_settings_progress())
        .unmap(pos);
    let launcher_pos = state
        .shell
        .launcher_placement(w, h, state.shell.launcher_progress())
        .unmap(pos);
    let qs_body_contains = qs_pos.y
        >= f64::from(shell::quick_settings_rect(w, h).y + shell::quick_tiles_top())
        && shell::quick_settings_rect(w, h).contains(qs_pos);
    qs_pos.y += f64::from(state.shell.shade_scroll);
    let power_pos = state
        .shell
        .power_placement(w, state.shell.power_progress())
        .unmap(pos);

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
        if state.shell.dock_hit(w, h, items.len(), pos) == Some(i) {
            let serial = state.next_serial();
            match items.get(i).cloned() {
                Some(DockItem::Launcher) => {
                    let origin = state.shell.dock_item_rect(w, h, items.len(), i);
                    open_launcher(state, origin);
                }
                Some(DockItem::Launch(app)) => {
                    let icon = state.shell.dock_item_rect(w, h, items.len(), i);
                    launch_or_focus(state, &app, Some(icon));
                }
                Some(DockItem::Focus { window, .. }) => {
                    if state.window_props(&window).minimized {
                        let icon = state.shell.dock_item_rect(w, h, items.len(), i);
                        state.restore_window(&window, serial);
                        state.shell.set_window_launch_origin(&window, icon);
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
            && qs_body_contains
            && shell::quick_settings_hit(w, h, qs_pos, 1.0) == Some(action)
        {
            perform_quick_action(state, action);
        }
    }

    if let Some(i) = state.shell.press_chip.take() {
        consumed = true;
        state.shell.press_end(PressKey::Chip(i));
        if state.shell.quick_settings_open
            && qs_body_contains
            && shell::palette_chip_rect_in(shell::quick_settings_rect(w, h), i).contains(qs_pos)
        {
            state.set_variant(VARIANTS[i]);
        }
    }

    if let Some(slot) = state.shell.press_launch.take() {
        consumed = true;
        state.shell.press_end(PressKey::Launcher(slot));
        if state.shell.launcher_open {
            let filtered = state.shell.filtered_apps();
            if shell::launcher_hit(
                w,
                h,
                filtered.len(),
                state.shell.launcher_scroll(),
                launcher_pos,
            ) == LauncherHit::Tile(slot)
            {
                if let Some(&idx) = filtered.get(slot) {
                    let app = state.shell.apps[idx].clone();
                    let origin = launcher_launch_origin(state, slot);
                    launch_or_focus(state, &app, origin);
                }
                close_launcher(state);
            }
        }
    }

    if let Some(i) = state.shell.press_power.take() {
        consumed = true;
        state.shell.press_end(PressKey::Power(i));
        if state.shell.power_open && shell::power_row_at(w, power_pos) == Some(i) {
            if let Some(action) = shell::POWER_ACTIONS.get(i).copied() {
                perform_power_action(state, action);
            }
        }
    }

    if !consumed && state.shell.launcher_open {
        // A click on the scrim (or outside the tiles) dismisses the launcher.
        let filtered = state.shell.filtered_apps();
        if shell::launcher_hit(
            w,
            h,
            filtered.len(),
            state.shell.launcher_scroll(),
            launcher_pos,
        ) == LauncherHit::Scrim
        {
            close_launcher(state);
        }
    }
}

pub(crate) fn perform_top_action(state: &mut State, action: TopAction) {
    let w = state.display.size.w as f32;
    if action != TopAction::Clock {
        state.shell.set_calendar_open(false);
    }
    match action {
        TopAction::Home => show_home(state),
        TopAction::Clock => {
            let open = !state.shell.calendar.open;
            state.shell.set_brightness_open(false);
            state.shell.set_power_open(false);
            state.shell.set_quick_settings_open(false);
            state.shell.set_calendar_open(open);
        }
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
        QuickAction::Wifi => {
            if state.services.status.wifi.available && !state.services.status.wifi.busy {
                state.services.request(crate::system::Action::Wifi(
                    !state.services.status.wifi.enabled,
                ));
                state.shell.set_system_status(state.services.status.clone());
            }
        }
        QuickAction::Bluetooth => {
            if state.services.status.bluetooth.available && !state.services.status.bluetooth.busy {
                state.services.request(crate::system::Action::Bluetooth(
                    !state.services.status.bluetooth.enabled,
                ));
                state.shell.set_system_status(state.services.status.clone());
            }
        }
        QuickAction::Theme => {
            // Radial reveal grows out of the tile the user pressed.
            let (w, h) = (state.display.size.w as f32, state.display.size.h as f32);
            let mut tile = shell::quick_setting_tile_rect(w, h, shell::quick_action_index(action));
            tile.y -= state.shell.shade_scroll;
            let tile = state
                .shell
                .quick_settings_placement(w, h, state.shell.quick_settings_progress())
                .map_rect(tile);
            state.toggle_theme(Point::from((
                tile.x as f64 + tile.w as f64 / 2.0,
                tile.y as f64 + tile.h as f64 / 2.0,
            )));
        }
        QuickAction::NightLight => {
            if state.shell.night_light_available {
                state.shell.toggle_night_light();
            }
        }
        QuickAction::ReduceMotion => {
            state.toggle_reduce_motion();
        }
        QuickAction::Brightness => {
            state.shell.set_quick_settings_open(false);
            state.shell.brightness_open = true;
            state.shell.set_brightness_open(true);
        }
    }
}
