//! Keyboard.
use super::*;
use smithay::input::keyboard::Keycode;
pub fn on_keyboard<B: InputBackend, E: KeyboardKeyEvent<B>>(state: &mut State, event: &E) {
    let serial = state.next_serial();
    let keyboard = state.keyboard.clone();
    let time = event.time_msec();
    let pressed = event.state() == KeyState::Pressed;
    let code = event.key_code();
    keyboard.input::<(), _>(
        state,
        event.key_code(),
        event.state(),
        serial,
        time,
        |state, mods, handle| on_key(state, mods, handle, pressed, code),
    );
}

pub(super) fn on_key(
    state: &mut State,
    mods: &ModifiersState,
    handle: smithay::input::keyboard::KeysymHandle<'_>,
    pressed: bool,
    code: Keycode,
) -> FilterResult<()> {
    let sym = handle.modified_sym();

    // Pair consumed presses with consumed releases even if the shell closes
    // between them (for example, Escape closing a popup or Super+P opening it).
    if !pressed {
        if state.swallowed_keys.remove(&code) {
            return FilterResult::Intercept(());
        }
        return if state.shell.launcher_open {
            FilterResult::Intercept(())
        } else {
            FilterResult::Forward
        };
    }

    if state.shell.app_menu.is_some() {
        state.swallowed_keys.insert(code);
        if sym == Keysym::Return || sym == Keysym::KP_Enter {
            state.shell.activate_pin_menu();
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
    if state.shell.quick_settings_open && sym == Keysym::Escape {
        state.swallowed_keys.insert(code);
        state.shell.set_quick_settings_open(false);
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

    FilterResult::Forward
}
