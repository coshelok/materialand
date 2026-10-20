//! The same ordered target selection is used for motion, wheel and buttons.
use super::*;
use crate::desktop::hit_test::WindowHit;
pub(super) enum Target {
    Shell(Control),
    Window(WindowHit),
    Desktop,
}
#[derive(Clone, Copy)]
pub(super) enum Control {
    Nav(shell::NavAction),
    Status(shell::StatusAction),
    Notification(shell::NotificationHit),
    Overview(crate::desktop::overview::OverviewHit),
    SplitDivider,
    ShadeHeader(TopAction),
    ConnectionPage(QuickAction),
    Connection(usize),
    ConnectionSettings,
    PinMenu,
    HomeMenu,
    Home(shell::HomeHit),
    Calendar(shell::CalendarAction),
    Top(TopAction),
    Dock(usize),
    Quick(QuickAction),
    Chip(usize),
    Power(usize),
    Brightness(f32),
    ShadeBrightness(f32),
    Launcher(LauncherHit),
    Block,
}
pub(super) fn target(state: &State, pos: Point<f64, Logical>) -> Target {
    let (w, h) = (state.display.size.w as f32, state.display.size.h as f32);
    if let Some(menu) = &state.shell.app_menu {
        let rect = shell::app_menu_rect(w, h, menu.point);
        return Target::Shell(
            if !shell::tablet_mode() && shell::app_menu_button(rect).contains(pos) {
                Control::PinMenu
            } else if shell::app_menu_home_button(rect).contains(pos) {
                Control::HomeMenu
            } else {
                Control::Block
            },
        );
    }
    let fullscreen = state.has_fullscreen_window();
    if state
        .shell
        .statusbar_visible(fullscreen, state.overview.visible())
    {
        if let Some(action) = shell::statusbar_hit(w, pos) {
            return Target::Shell(Control::Status(action));
        }
    }
    if !fullscreen || state.overview.visible() {
        if let Some(action) = shell::navigation_hit(w, h, pos) {
            return Target::Shell(Control::Nav(action));
        }
    }
    if state.overview.open {
        return Target::Shell(
            state
                .overview
                .hit(w, h, pos)
                .map(Control::Overview)
                .unwrap_or(Control::Block),
        );
    }
    if state.overview.visible() {
        return Target::Shell(Control::Block);
    }
    if state.shell.calendar.open {
        let local = state.shell.calendar_placement(w, h).unmap(pos);
        if shell::calendar_rect(w, h).contains(local) {
            return Target::Shell(
                shell::calendar_hit(w, h, local)
                    .map(Control::Calendar)
                    .unwrap_or(Control::Block),
            );
        }
    }
    let power_pos = state
        .shell
        .power_placement(w, state.shell.power_progress())
        .unmap(pos);
    let brightness_pos = state
        .shell
        .brightness_placement(w, state.shell.brightness_progress())
        .unmap(pos);
    let mut qs_pos = state
        .shell
        .quick_settings_placement(w, h, state.shell.quick_settings_progress())
        .unmap(pos);
    if state.shell.power_open && shell::power_menu_rect(w).contains(power_pos) {
        return Target::Shell(
            shell::power_row_at(w, power_pos)
                .map(Control::Power)
                .unwrap_or(Control::Block),
        );
    }
    if state.shell.brightness_open && shell::brightness_popup(w).contains(brightness_pos) {
        return Target::Shell(
            shell::brightness_hit(w, brightness_pos)
                .filter(|_| state.shell.brightness_available)
                .map(Control::Brightness)
                .unwrap_or(Control::Block),
        );
    }
    if state.shell.launcher_open {
        return Target::Shell(Control::Launcher(shell::launcher_hit(
            w,
            h,
            state.shell.filtered_apps().len(),
            state.shell.launcher_scroll(),
            state
                .shell
                .launcher_placement(w, h, state.shell.launcher_progress())
                .unmap(pos),
        )));
    }
    if state.shell.quick_settings_open && shell::quick_settings_rect(w, h).contains(qs_pos) {
        if shell::tablet_mode()
            && !state.shell.notifications_open
            && state.shell.connection_page.is_none()
        {
            if let Some(value) = shell::shade_brightness_hit(w, h, qs_pos) {
                return Target::Shell(if state.shell.brightness_available {
                    Control::ShadeBrightness(value)
                } else {
                    Control::Block
                });
            }
        }
        if let Some(action) = shell::shade_header_hit(w, h, qs_pos) {
            return Target::Shell(Control::ShadeHeader(action));
        }
        if qs_pos.y < f64::from(shell::quick_settings_rect(w, h).y + shell::quick_tiles_top()) {
            return Target::Shell(Control::Block);
        }
        if state.shell.notifications_open {
            return Target::Shell(
                state
                    .shell
                    .notification_hit(w, h, qs_pos)
                    .map(Control::Notification)
                    .unwrap_or(Control::Block),
            );
        }
        if let Some(page) = state.shell.connection_page {
            if (qs_pos.y as f32)
                < shell::quick_settings_rect(w, h).y + shell::quick_tiles_top() + 40.0
            {
                return Target::Shell(Control::ConnectionSettings);
            }
            let row = ((qs_pos.y as f32
                - shell::quick_settings_rect(w, h).y
                - shell::quick_tiles_top()
                - 48.0
                + state.shell.shade_scroll)
                / config::design().power.row_height)
                .floor();
            let len = if page == QuickAction::Wifi {
                state.services.status.networks.len()
            } else {
                state.services.status.devices.len()
            };
            return Target::Shell(if row >= 0.0 && (row as usize) < len {
                Control::Connection(row as usize)
            } else {
                Control::Block
            });
        }
        qs_pos.y += f64::from(state.shell.shade_scroll);
        for action in [QuickAction::Wifi, QuickAction::Bluetooth] {
            let r = shell::quick_setting_tile_rect(w, h, shell::quick_action_index(action));
            if r.contains(qs_pos)
                && qs_pos.x as f32 > r.right() - 48.0
                && (shell::tablet_mode() || qs_pos.y as f32 > r.y + r.h / 2.0)
            {
                return Target::Shell(Control::ConnectionPage(action));
            }
        }

        let control = shell::quick_settings_hit(w, h, qs_pos, 1.0)
            .map(Control::Quick)
            .or_else(|| {
                if state.shell.shade_scroll == 0.0 {
                    return shell::palette_chip_hit(w, h, qs_pos).map(Control::Chip);
                }
                (0..VARIANTS.len())
                    .find(|&i| {
                        shell::palette_chip_rect_in(shell::quick_settings_rect(w, h), i)
                            .contains(qs_pos)
                    })
                    .map(Control::Chip)
            })
            .unwrap_or(Control::Block);
        return Target::Shell(control);
    }
    if !fullscreen && !shell::tablet_mode() {
        if shell::topbar_rect(w).contains(pos) {
            return Target::Shell(
                shell::topbar_hit(w, pos)
                    .map(Control::Top)
                    .unwrap_or(Control::Block),
            );
        }
        if state
            .shell
            .dock_rect(w, h, dock_item_count(state))
            .contains(pos)
        {
            return Target::Shell(
                state
                    .shell
                    .dock_hit(w, h, dock_item_count(state), pos)
                    .map(Control::Dock)
                    .unwrap_or(Control::Block),
            );
        }
    }
    if state.split_divider_hit(pos) {
        return Target::Shell(Control::SplitDivider);
    }
    if let Some(window) = state.window_under(pos) {
        return Target::Window(window);
    }
    if !fullscreen {
        if let Some(hit) = state.shell.home_hit(w, h, pos) {
            return Target::Shell(Control::Home(hit));
        }
    }
    Target::Desktop
}
pub(super) fn dismiss_outside(state: &mut State, pos: Point<f64, Logical>) {
    let (w, h) = (state.display.size.w as f32, state.display.size.h as f32);
    let top = if shell::tablet_mode() {
        shell::statusbar_hit(w, pos).map(|a| {
            if a == shell::StatusAction::Clock {
                TopAction::Clock
            } else {
                TopAction::QuickSettings
            }
        })
    } else {
        shell::topbar_hit(w, pos)
    };
    if state.shell.calendar.open
        && !shell::calendar_rect(w, h).contains(state.shell.calendar_placement(w, h).unmap(pos))
        && top != Some(TopAction::Clock)
    {
        state.shell.set_calendar_open(false);
    }
    let power_pos = state
        .shell
        .power_placement(w, state.shell.power_progress())
        .unmap(pos);
    let brightness_pos = state
        .shell
        .brightness_placement(w, state.shell.brightness_progress())
        .unmap(pos);
    let qs_pos = state
        .shell
        .quick_settings_placement(w, h, state.shell.quick_settings_progress())
        .unmap(pos);
    if state.shell.power_open
        && !shell::power_menu_rect(w).contains(power_pos)
        && top != Some(TopAction::Power)
    {
        state.shell.set_power_open(false);
    }
    if state.shell.brightness_open
        && !shell::brightness_popup(w).contains(brightness_pos)
        && top != Some(TopAction::Brightness)
    {
        state.shell.set_brightness_open(false);
    }
    if state.shell.quick_settings_open
        && !shell::quick_settings_rect(w, h).contains(qs_pos)
        && top != Some(TopAction::QuickSettings)
    {
        state.shell.set_quick_settings_open(false);
    }
}
pub(super) fn press(state: &mut State, control: Control, pos: Point<f64, Logical>) {
    let design = crate::config::design();
    let (w, h) = (state.display.size.w as f32, state.display.size.h as f32);
    super::shortcut_drag::candidate(state, control, pos);
    let (key, rect) = match control {
        Control::Block if state.overview.open => {
            state.overview.background_pressed = true;
            return;
        }
        Control::Overview(hit) => {
            state.overview.start_drag(hit, pos);
            return;
        }
        Control::SplitDivider => {
            state.begin_split_drag();
            return;
        }
        Control::ConnectionPage(page) => {
            state.shell.connection_page = Some(page);
            state.shell.shade_scroll = 0.0;
            state.services.request(crate::system::Action::Refresh);
            return;
        }
        Control::Connection(index) => {
            super::navigation::connect(state, index);
            return;
        }
        Control::ConnectionSettings => {
            super::navigation::connection_settings(state);
            return;
        }
        Control::ShadeHeader(action) => {
            super::actions::perform_top_action(state, action);
            state.shell.consume_menu_release = true;
            return;
        }
        Control::Status(action) => {
            super::navigation::status_action(state, action);
            state.shell.consume_menu_release = true;
            return;
        }
        Control::Notification(hit) => {
            let id = match hit {
                shell::NotificationHit::Clear => None,
                shell::NotificationHit::Dismiss(id) => Some(id),
            };
            state.notifications.dismiss(id);
            if let Some(snapshot) = state.notifications.poll() {
                state.shell.notifications = snapshot;
            }
            state.shell.consume_menu_release = true;
            return;
        }
        Control::Nav(action) => {
            super::navigation::perform(state, action);
            state.shell.consume_menu_release = true;
            return;
        }
        Control::HomeMenu => {
            if let Some(menu) = &mut state.shell.app_menu {
                menu.press_home = true;
                menu.selected = 1;
            }
            return;
        }
        Control::Home(hit) => {
            state.shell.home.pressed = Some(hit);
            if let shell::HomeHit::App(i) = hit {
                (PressKey::Home(i), state.shell.home_tile_rect(w, h, i))
            } else {
                return;
            }
        }
        Control::Calendar(action) => {
            state.shell.calendar.pressed = Some(action);
            return;
        }
        Control::PinMenu => {
            state.shell.press_pin = true;
            if let Some(menu) = &mut state.shell.app_menu {
                menu.selected = 0;
            }
            return;
        }
        Control::Top(action) => {
            state.shell.press_top = Some(action);
            (PressKey::Top(action), shell::topbar_button_rect(w, action))
        }
        Control::Dock(i) => {
            state.shell.press_dock = Some(i);
            (
                PressKey::Dock(i),
                state.shell.dock_item_rect(w, h, dock_item_count(state), i),
            )
        }
        Control::Quick(action) => {
            state.shell.press_tile = Some(action);
            let i = shell::quick_action_index(action);
            let mut rect = shell::quick_setting_tile_rect(w, h, i);
            rect.y -= state.shell.shade_scroll;
            (PressKey::Tile(i), rect)
        }
        Control::Chip(i) => {
            state.shell.press_chip = Some(i);
            let mut rect = shell::palette_chip_rect_in(shell::quick_settings_rect(w, h), i);
            rect.y -= state.shell.shade_scroll;
            (PressKey::Chip(i), rect)
        }
        Control::Power(i) => {
            state.shell.press_power = Some(i);
            (PressKey::Power(i), shell::power_row_rect(w, i))
        }
        Control::Launcher(LauncherHit::Tile(i)) => {
            state.shell.press_launch = Some(i);
            let r = shell::RectF {
                x: 0.0,
                y: 0.0,
                w: design.launcher.tile_size,
                h: design.launcher.tile_size,
            };
            (PressKey::Launcher(i), r)
        }
        Control::ShadeBrightness(value) => {
            if state.shell.brightness_available {
                state.brightness = value;
                state.interaction = Interaction::ShadeBrightness;
                state.shell.set_slider_active(true);
            }
            return;
        }
        Control::Brightness(value) => {
            if !state.shell.brightness_available {
                return;
            }
            state.brightness = value;
            state.interaction = Interaction::Brightness;
            state.shell.set_slider_active(true);
            return;
        }
        _ => return,
    };
    let center = match control {
        Control::Home(_) | Control::Launcher(_) => (rect.w / 2.0, rect.h / 2.0),
        Control::Top(_) => (
            rect.x - shell::topbar_rect(w).x + rect.w / 2.0,
            rect.y - shell::topbar_rect(w).y + rect.h / 2.0,
        ),
        Control::Dock(_) => (
            rect.x - state.shell.dock_rect(w, h, dock_item_count(state)).x
                + rect.w / 2.0
                + shell::floating_dock_padding(),
            rect.y - state.shell.dock_rect(w, h, dock_item_count(state)).y
                + rect.h / 2.0
                + shell::floating_dock_padding(),
        ),
        Control::Power(_) => (
            rect.x - shell::power_menu_rect(w).x + design.shadow.padding + rect.w / 2.0,
            rect.y - shell::power_menu_rect(w).y + design.shadow.padding + rect.h / 2.0,
        ),
        _ => (
            rect.x - shell::quick_settings_rect(w, h).x + design.shadow.padding + rect.w / 2.0,
            rect.y - shell::quick_settings_rect(w, h).y + design.shadow.padding + rect.h / 2.0,
        ),
    };
    let _ = pos;
    state.shell.press_begin(
        key,
        center,
        rect.w.max(rect.h) * design.motion.ripple_radius_ratio,
        design.motion.press_scale,
    );
}
