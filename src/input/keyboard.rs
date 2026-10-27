//! Keyboard.
use super::*;
use smithay::input::keyboard::Keycode;
pub fn on_keyboard<B: InputBackend, E: KeyboardKeyEvent<B>>(state: &mut State, event: &E) {
    let serial = state.next_serial();
    let keyboard = state.keyboard.clone();
    let time = event.time_msec();
    let pressed = event.state() == KeyState::Pressed;
    if pressed {
        super::stop_touchpad_scroll(state);
        state.launcher_swipe = Default::default();
    }
    let code = event.key_code();
    let mut forwarded = false;
    keyboard.input::<(), _>(
        state,
        event.key_code(),
        event.state(),
        serial,
        time,
        |state, mods, handle| {
            let result = on_key(state, mods, handle, pressed, code);
            forwarded = matches!(result, FilterResult::Forward);
            result
        },
    );
    if forwarded {
        if let Some(surface) = keyboard.current_focus() {
            state.record_activation_input(serial, &surface);
        }
    }
}

pub(super) fn on_key(
    state: &mut State,
    mods: &ModifiersState,
    handle: smithay::input::keyboard::KeysymHandle<'_>,
    pressed: bool,
    code: Keycode,
) -> FilterResult<()> {
    let sym = handle.modified_sym();
    let is_super = matches!(sym, Keysym::Super_L | Keysym::Super_R);
    let alone = is_super
        && !mods.ctrl
        && !mods.alt
        && !mods.shift
        && state
            .keyboard
            .pressed_keys()
            .iter()
            .all(|held| *held == code)
        && state.forwarded_buttons.is_empty()
        && state.swallowed_buttons.is_empty()
        && matches!(state.interaction, Interaction::None);
    if state.super_tap.key(code, pressed, is_super, alone) {
        super::navigation::perform(state, shell::NavAction::Home);
    }
    // Keep modifier delivery intact for client shortcuts and either Super key.
    if is_super {
        return FilterResult::Forward;
    }

    // Pair consumed presses with consumed releases even if the shell closes
    // between them (for example, Escape closing a popup or Super+P opening it).
    if !pressed {
        if state.swallowed_keys.remove(&code) {
            return FilterResult::Intercept(());
        }
        return FilterResult::Forward;
    }

    let raw = handle.raw_latin_sym_or_raw_current_sym().unwrap_or(sym);
    if mods.logo && raw == Keysym::BackSpace {
        state.swallowed_keys.insert(code);
        state.end_split();
        return FilterResult::Intercept(());
    }
    if mods.logo && (raw == Keysym::Tab || raw == Keysym::s) {
        state.swallowed_keys.insert(code);
        super::navigation::perform(
            state,
            if raw == Keysym::Tab {
                shell::NavAction::Overview
            } else {
                shell::NavAction::Shade
            },
        );
        return FilterResult::Intercept(());
    }
    if mods.logo && raw == Keysym::p {
        state.swallowed_keys.insert(code);
        super::navigation::perform(state, shell::NavAction::Launcher);
        return FilterResult::Intercept(());
    }
    if state.overview.open && !(mods.logo && raw == Keysym::h) {
        state.swallowed_keys.insert(code);
        match sym {
            Keysym::Escape => {
                state.overview.pending_split = None;
                state.overview.close(state.shell.reduce_motion);
            }
            Keysym::Left | Keysym::Right | Keysym::Up | Keysym::Down => {
                let grid = crate::desktop::overview::Overview::grid_layout(
                    state.display.size.w as f32,
                    state.display.size.h as f32,
                );
                let step = if grid && matches!(sym, Keysym::Left | Keysym::Right) {
                    config::design().overview.grid_rows as i32
                } else {
                    1
                };
                let sign = if matches!(sym, Keysym::Left | Keysym::Up) {
                    -1
                } else {
                    1
                };
                state
                    .overview
                    .navigate(sign * step, state.shell.reduce_motion);
            }
            Keysym::Return | Keysym::KP_Enter => {
                state.overview_action(crate::desktop::overview::OverviewHit {
                    index: state.overview.selected,
                    action: crate::desktop::overview::OverviewAction::Select,
                })
            }
            Keysym::Delete => state.overview_action(crate::desktop::overview::OverviewHit {
                index: state.overview.selected,
                action: crate::desktop::overview::OverviewAction::Close,
            }),
            _ => {}
        }
        return FilterResult::Intercept(());
    }
    if mods.logo && raw == Keysym::h {
        state.swallowed_keys.insert(code);
        show_home(state);
        return FilterResult::Intercept(());
    }
    if state.shell.app_menu.is_some() {
        state.swallowed_keys.insert(code);
        if sym == Keysym::Return || sym == Keysym::KP_Enter {
            if state
                .shell
                .app_menu
                .as_ref()
                .is_some_and(|m| m.selected == 1)
            {
                state.shell.activate_home_menu();
            } else {
                state.shell.activate_pin_menu();
            }
        } else if sym == Keysym::Up || sym == Keysym::Down {
            if let Some(menu) = &mut state.shell.app_menu {
                menu.selected = if shell::tablet_mode() {
                    1
                } else {
                    usize::from(sym == Keysym::Down)
                };
            }
        } else if sym == Keysym::Escape {
            state.shell.close_app_menu();
        }
        return FilterResult::Intercept(());
    }
    if state.shell.launcher_open {
        state.swallowed_keys.insert(code);
        if sym == Keysym::Escape {
            close_launcher(state);
            return FilterResult::Intercept(());
        }
        if sym == Keysym::Return || sym == Keysym::KP_Enter {
            launch_first_filtered(state);
            return FilterResult::Intercept(());
        }
        if matches!(
            sym,
            Keysym::Page_Up | Keysym::Page_Down | Keysym::Home | Keysym::End
        ) {
            let (w, h) = (state.display.size.w as f32, state.display.size.h as f32);
            let max = shell::launcher_scroll_max(w, h, state.shell.filtered_apps().len());
            let page = shell::launcher_viewport(w, h).h * 0.85;
            let delta = match sym {
                Keysym::Home => -max,
                Keysym::End => max,
                Keysym::Page_Up => -page,
                _ => page,
            };
            state.shell.scroll_launcher(w, h, delta);
            return FilterResult::Intercept(());
        }
        if sym == Keysym::BackSpace {
            state.shell.launcher_backspace();
            return FilterResult::Intercept(());
        }
        let cp = smithay::input::keyboard::xkb::keysym_to_utf32(sym);
        if let Some(c) = char::from_u32(cp) {
            if !c.is_control() {
                state.shell.launcher_push(c);
                return FilterResult::Intercept(());
            }
        }
        return FilterResult::Intercept(());
    }

    if state.shell.calendar.open {
        state.swallowed_keys.insert(code);
        match sym {
            Keysym::Escape => state.shell.set_calendar_open(false),
            Keysym::Left | Keysym::Page_Up => state
                .shell
                .calendar
                .navigate(shell::CalendarAction::Previous),
            Keysym::Right | Keysym::Page_Down => {
                state.shell.calendar.navigate(shell::CalendarAction::Next)
            }
            Keysym::Home => state.shell.calendar.navigate(shell::CalendarAction::Today),
            _ => {}
        }
        return FilterResult::Intercept(());
    }
    if state.shell.brightness_open && sym == Keysym::Escape {
        state.swallowed_keys.insert(code);
        state.finish_interaction(BTN_LEFT);
        state.shell.set_brightness_open(false);
        return FilterResult::Intercept(());
    }
    if state.shell.quick_settings_open && sym == Keysym::Escape {
        state.swallowed_keys.insert(code);
        if std::mem::take(&mut state.shell.notifications_open)
            | state.shell.connection_page.take().is_some()
            | std::mem::take(&mut state.shell.appearance_page)
        {
            state.shell.reset_shade_scroll();
        } else {
            state.shell.set_quick_settings_open(false);
        }
        return FilterResult::Intercept(());
    }

    if state.shell.power_open && sym == Keysym::Escape {
        state.swallowed_keys.insert(code);
        state.shell.power_open = false;
        state.shell.set_power_open(false);
        return FilterResult::Intercept(());
    }

    // Chords the shell owns (see `config::KEY_BINDINGS`): Super+P opens the
    // launcher, Super+Return a terminal, Super+D toggles the theme and
    // Super+Q closes the focused window. Everything else stays with clients.
    // The chord key is read from the raw Latin keysym so the chords keep
    // working while a non-Latin layout (Russian, for one) is selected.
    let chord_sym = handle.raw_latin_sym_or_raw_current_sym().unwrap_or(sym);
    let chord = if sym == Keysym::Return {
        Some(config::BindingKey::Return)
    } else {
        char::from_u32(smithay::input::keyboard::xkb::keysym_to_utf32(chord_sym))
            .filter(|c| !c.is_control())
            .map(config::BindingKey::Char)
    };
    if let Some(action) = chord.and_then(|key| config::binding_for(mods.logo, key)) {
        state.swallowed_keys.insert(code);
        match action {
            config::BindingAction::Home => show_home(state),
            config::BindingAction::Terminal => {
                let terminal = state.shell.terminal.clone();
                spawn_program(&terminal);
            }
            config::BindingAction::Launcher => {
                let w = state.display.size.w as f32;
                open_launcher(state, shell::topbar_button_rect(w, TopAction::Launcher));
            }
            config::BindingAction::ToggleTheme => {
                let w = state.display.size.w as f32;
                let rect = shell::topbar_button_rect(w, TopAction::Theme);
                state.toggle_theme(Point::from((
                    rect.x as f64 + rect.w as f64 / 2.0,
                    rect.y as f64 + rect.h as f64 / 2.0,
                )));
            }
            config::BindingAction::CloseWindow => {
                if let Some(w) = state.windows.focused.clone() {
                    state.request_close_window(&w);
                }
            }
        }
        return FilterResult::Intercept(());
    }

    if state.shell.quick_settings_open || state.shell.power_open || state.shell.brightness_open {
        state.swallowed_keys.insert(code);
        return FilterResult::Intercept(());
    }
    FilterResult::Forward
}
