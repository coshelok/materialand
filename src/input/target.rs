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
    PinMenu,
    Calendar(shell::CalendarAction),
    Top(TopAction),
    Dock(usize),
    Quick(QuickAction),
    Chip(usize),
    Power(usize),
    Brightness(f32),
    Launcher(LauncherHit),
    Block,
}
pub(super) fn target(state: &State, pos: Point<f64, Logical>) -> Target {
    let (w, h) = (state.display.size.w as f32, state.display.size.h as f32);
    if let Some(menu) = &state.shell.app_menu {
        let rect = shell::app_menu_rect(w, h, menu.point);
        return Target::Shell(if shell::app_menu_button(rect).contains(pos) {
            Control::PinMenu
        } else {
            Control::Block
        });
    }
    let fullscreen = state.has_fullscreen_window();
    if !fullscreen && state.shell.calendar.open {
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
    let qs_pos = state
        .shell
        .quick_settings_placement(w, h, state.shell.quick_settings_progress())
        .unmap(pos);
    if !fullscreen && state.shell.power_open && shell::power_menu_rect(w).contains(power_pos) {
        return Target::Shell(
            shell::power_row_at(w, power_pos)
                .map(Control::Power)
                .unwrap_or(Control::Block),
        );
    }
    if !fullscreen
        && state.shell.brightness_open
        && shell::brightness_popup(w).contains(brightness_pos)
    {
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
    if !fullscreen
        && state.shell.quick_settings_open
        && shell::quick_settings_rect(w, h).contains(qs_pos)
    {
        let control = shell::quick_settings_hit(w, h, qs_pos, 1.0)
            .map(Control::Quick)
            .or_else(|| shell::palette_chip_hit(w, h, qs_pos).map(Control::Chip))
            .unwrap_or(Control::Block);
        return Target::Shell(control);
    }
    if !fullscreen {
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
    state
        .window_under(pos)
        .map(Target::Window)
        .unwrap_or(Target::Desktop)
}
pub(super) fn dismiss_outside(state: &mut State, pos: Point<f64, Logical>) {
    let (w, h) = (state.display.size.w as f32, state.display.size.h as f32);
    let top = shell::topbar_hit(w, pos);
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
    let (w, h) = (state.display.size.w as f32, state.display.size.h as f32);
    let (key, rect) = match control {
        Control::Calendar(action) => {
            state.shell.calendar.pressed = Some(action);
            return;
        }
        Control::PinMenu => {
            state.shell.press_pin = true;
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
            (PressKey::Tile(i), shell::quick_setting_tile_rect(w, h, i))
        }
        Control::Chip(i) => {
            state.shell.press_chip = Some(i);
            let Some(rect) = shell::palette_chip_rect(w, h, i) else {
                return;
            };
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
                w: config::LAUNCHER_TILE,
                h: config::LAUNCHER_TILE,
            };
            (PressKey::Launcher(i), r)
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
        Control::Launcher(_) => (rect.w / 2.0, rect.h / 2.0),
        Control::Top(_) => (
            rect.x - shell::topbar_rect(w).x + rect.w / 2.0,
            rect.y - shell::topbar_rect(w).y + rect.h / 2.0,
        ),
        Control::Dock(_) => (
            rect.x - state.shell.dock_rect(w, h, dock_item_count(state)).x + rect.w / 2.0,
            rect.y - state.shell.dock_rect(w, h, dock_item_count(state)).y + rect.h / 2.0,
        ),
        Control::Power(_) => (
            rect.x - shell::power_menu_rect(w).x + shell::SHADOW_PAD + rect.w / 2.0,
            rect.y - shell::power_menu_rect(w).y + shell::SHADOW_PAD + rect.h / 2.0,
        ),
        _ => (
            rect.x - shell::quick_settings_rect(w, h).x + shell::SHADOW_PAD + rect.w / 2.0,
            rect.y - shell::quick_settings_rect(w, h).y + shell::SHADOW_PAD + rect.h / 2.0,
        ),
    };
    let _ = pos;
    state
        .shell
        .press_begin(key, center, rect.w.max(rect.h) * 0.7, 0.90);
}
