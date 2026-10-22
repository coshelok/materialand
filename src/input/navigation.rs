//! Gesture, keyboard and mouse entry points share the same shell transitions.
use super::*;
pub(super) fn perform(state: &mut State, action: shell::NavAction) {
    match action {
        shell::NavAction::Home => show_home(state),
        shell::NavAction::Launcher => {
            state.overview.close(state.shell.reduce_motion);
            state.shell.set_quick_settings_open(false);
            let origin =
                shell::navigation_rect(state.display.size.w as f32, state.display.size.h as f32);
            open_launcher(state, origin);
        }
        shell::NavAction::Overview => {
            if state.overview.open {
                state.overview.close(state.shell.reduce_motion);
            } else {
                state.open_overview();
            }
        }
        shell::NavAction::Shade => open_shade(state, !state.shell.quick_settings_open),
    }
}
pub(crate) fn open_shade(state: &mut State, open: bool) {
    state.overview.close(state.shell.reduce_motion);
    close_launcher(state);
    state.shell.close_app_menu();
    state.shell.set_calendar_open(false);
    state.shell.set_power_open(false);
    state.shell.set_brightness_open(false);
    if open {
        super::prepare_shell_overlay(state);
        state.shell.connection_page = None;
        state.shell.notifications_open = false;
        state.shell.reset_shade_scroll();
        state.services.request(crate::system::Action::Refresh);
    }
    state.shell.set_quick_settings_open(open);
}

pub(super) fn connect(state: &mut State, index: usize) {
    if state.shell.connection_page == Some(QuickAction::Wifi) {
        if let Some(network) = state.services.status.networks.get(index) {
            state
                .services
                .request(crate::system::Action::Connect(network.ssid.clone()));
        }
    } else if let Some(device) = state.services.status.devices.get(index) {
        state
            .services
            .request(crate::system::Action::BluetoothDevice(
                device.path.clone(),
                !device.connected,
            ));
    }
    state.shell.set_system_status(state.services.status.clone());
}
pub(super) fn connection_settings(state: &mut State) {
    let wifi = state.shell.connection_page == Some(QuickAction::Wifi);
    let program = if wifi {
        "nm-connection-editor"
    } else {
        "blueman-manager"
    };
    let available = program_available(program);
    state.shell.set_quick_settings_open(false);
    if available {
        spawn_program(&[program.into()]);
    } else if wifi && program_available("nmtui") {
        let entry = config::AppEntry {
            name: "Network connections".into(),
            exec: "nmtui connect".into(),
            icon: "wifi".into(),
            terminal: true,
            desktop_file: None,
        };
        spawn_program(&entry.command_in_terminal(&state.shell.terminal));
    } else {
        state.services.status.error = Some(
            if wifi {
                "Install nm-connection-editor or nmtui to manage Wi-Fi connections"
            } else {
                "Bluetooth pairing needs blueman-manager"
            }
            .into(),
        );
        state.shell.set_system_status(state.services.status.clone());
        state.shell.set_quick_settings_open(true);
    }
}

fn program_available(program: &str) -> bool {
    std::env::var_os("PATH")
        .is_some_and(|path| std::env::split_paths(&path).any(|dir| dir.join(program).is_file()))
}

pub(super) fn status_action(state: &mut State, action: shell::StatusAction) {
    match action {
        shell::StatusAction::Clock => {
            super::prepare_shell_overlay(state);
            state.overview.close(state.shell.reduce_motion);
            close_launcher(state);
            super::actions::perform_top_action(state, TopAction::Clock);
        }
        shell::StatusAction::Notifications => {
            let open = !(state.shell.quick_settings_open && state.shell.notifications_open);
            open_shade(state, open);
            state.shell.notifications_open = open;
        }
        shell::StatusAction::Wifi | shell::StatusAction::Bluetooth => {
            open_shade(state, true);
            state.shell.connection_page = Some(if action == shell::StatusAction::Wifi {
                QuickAction::Wifi
            } else {
                QuickAction::Bluetooth
            });
        }
        shell::StatusAction::Shade => {
            let open = !state.shell.quick_settings_open
                || state.shell.notifications_open
                || state.shell.connection_page.is_some();
            open_shade(state, open);
        }
    }
}
