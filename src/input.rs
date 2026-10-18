use smithay::{
    backend::input::{
        Axis, AxisSource, ButtonState, Event, InputBackend, KeyState, KeyboardKeyEvent,
        PointerAxisEvent, PointerButtonEvent,
    },
    desktop::WindowSurfaceType,
    reexports::wayland_server::protocol::wl_surface::WlSurface,
    input::{
        keyboard::{FilterResult, Keysym, ModifiersState},
        pointer::{AxisFrame, ButtonEvent, MotionEvent},
    },
    utils::{Logical, Point, Rectangle},
};

use crate::{
    config,
    shell::{self, DecorHit, DockItem, LauncherHit, PressKey, QuickAction, TopAction},
    state::{Interaction, ResizeEdges, State},
    theme::VARIANTS,
};

const BTN_LEFT: u32 = 0x110;

/// Forward one pointer motion and close the Wayland pointer frame.
fn send_pointer_motion(
    state: &mut State,
    focus: Option<(WlSurface, Point<f64, Logical>)>,
    event: &MotionEvent,
) {
    let pointer = state.pointer.clone();
    pointer.motion(state, focus, event);
    pointer.frame(state);
}

/// Forward one pointer button event and close the Wayland pointer frame.
fn send_pointer_button(state: &mut State, event: &ButtonEvent) {
    let pointer = state.pointer.clone();
    pointer.button(state, event);
    pointer.frame(state);
}

/// Forward wheel/touchpad axis events with their source and high-resolution
/// values intact, then terminate the Wayland pointer frame.
pub(crate) fn on_pointer_axis<B: InputBackend>(
    state: &mut State,
    event: B::PointerAxisEvent,
) {
    let horizontal_discrete = event.amount_v120(Axis::Horizontal);
    let vertical_discrete = event.amount_v120(Axis::Vertical);
    let horizontal = event.amount(Axis::Horizontal).unwrap_or_else(|| {
        horizontal_discrete.unwrap_or(0.0) * 15.0 / 120.0
    });
    let vertical = event.amount(Axis::Vertical).unwrap_or_else(|| {
        vertical_discrete.unwrap_or(0.0) * 15.0 / 120.0
    });

    let mut frame = AxisFrame::new(event.time_msec()).source(event.source());
    if horizontal != 0.0 {
        frame = frame
            .relative_direction(Axis::Horizontal, event.relative_direction(Axis::Horizontal))
            .value(Axis::Horizontal, horizontal);
        if let Some(discrete) = horizontal_discrete {
            frame = frame.v120(Axis::Horizontal, discrete as i32);
        }
    }
    if vertical != 0.0 {
        frame = frame
            .relative_direction(Axis::Vertical, event.relative_direction(Axis::Vertical))
            .value(Axis::Vertical, vertical);
        if let Some(discrete) = vertical_discrete {
            frame = frame.v120(Axis::Vertical, discrete as i32);
        }
    }
    if event.source() == AxisSource::Finger {
        if event.amount(Axis::Horizontal) == Some(0.0) {
            frame = frame.stop(Axis::Horizontal);
        }
        if event.amount(Axis::Vertical) == Some(0.0) {
            frame = frame.stop(Axis::Vertical);
        }
    }

    let pointer = state.pointer.clone();
    pointer.axis(state, frame);
    pointer.frame(state);
}

/// Forward a client press as one frame containing motion followed by button.
fn send_client_press(
    state: &mut State,
    surface: WlSurface,
    surface_location: Point<f64, Logical>,
    location: Point<f64, Logical>,
    button: u32,
    serial: smithay::utils::Serial,
    time: u32,
) {
    let pointer = state.pointer.clone();
    let motion = MotionEvent {
        location,
        serial,
        time,
    };
    pointer.motion(state, Some((surface, surface_location)), &motion);
    let press = ButtonEvent {
        serial,
        time,
        button,
        state: ButtonState::Pressed,
    };
    pointer.button(state, &press);
    pointer.frame(state);
}

/// Set when the compositor runs as a real session (`materialand` on DRM)
/// instead of nested inside another compositor.
static SESSION_MODE: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// Tell power controls whether this compositor owns the hardware session.
///
/// This flag is deliberately not used to choose the child application's Wayland
/// socket or session bus: both backends use the per-instance socket and private
/// bus established at startup.
pub fn set_session_mode(enabled: bool) {
    SESSION_MODE.store(enabled, std::sync::atomic::Ordering::Relaxed);
}

/// Spawn a program so that it connects to this compositor.
pub fn spawn_program(args: &[String]) {
    if args.is_empty() {
        return;
    }
    let mut cmd = std::process::Command::new(&args[0]);
    cmd.args(&args[1..]);
    let socket_name = std::env::var("WAYLAND_DISPLAY")
        .unwrap_or_else(|_| config::SOCKET_NAME.to_owned());
    cmd.env("WAYLAND_DISPLAY", socket_name)
        // WAYLAND_SOCKET takes precedence over WAYLAND_DISPLAY in clients. If
        // inherited from a launcher, it can bypass our socket and reconnect
        // straight to the host compositor.
        .env_remove("WAYLAND_SOCKET");
    if let Ok(runtime) = std::env::var("XDG_RUNTIME_DIR") {
        cmd.env("XDG_RUNTIME_DIR", runtime);
    }
    // The child belongs to this compositor, including in nested development
    // mode. Do not let host X11 selection or desktop identity override it.
    cmd.env("XDG_SESSION_TYPE", "wayland");
    cmd.env("XDG_CURRENT_DESKTOP", "Materialand");
    cmd.env_remove("DISPLAY");

    // The compositor starts a private D-Bus session in both backends. Never
    // forward a host activation token into this independent desktop session.
    cmd.env_remove("XDG_ACTIVATION_TOKEN")
        .env_remove("DBUS_STARTER_ADDRESS")
        .env_remove("DBUS_STARTER_BUS_TYPE")
        .env_remove("DBUS_SESSION_BUS_PID");
    // A client must not inherit the compositor's stdio: doing so leaks the
    // controlling terminal / pipe and lets a chatty client block on a full pipe.
    cmd.stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null());
    match cmd.spawn() {
        Ok(mut child) => {
            tracing::info!(program = %args[0], "launched");
            // Reap the child when it exits, so it never lingers as a zombie.
            std::thread::spawn(move || {
                let _ = child.wait();
            });
        }
        Err(e) => tracing::warn!(program = %args[0], "failed to launch: {e}"),
    }
}

/// The dock's current items: the launcher button, pinned launchers, then
/// running windows.
pub fn dock_items(state: &State) -> Vec<DockItem> {
    let mut items: Vec<DockItem> = vec![DockItem::Launcher];
    items.extend(state.shell.pinned.iter().cloned().map(DockItem::Launch));
    for window in state.space.elements() {
        // A toplevel without a buffer paints nothing (see `window_has_content`)
        // and must not occupy a dock slot.
        if !state.window_has_content(window) {
            continue;
        }
        let meta = state.window_meta(window);
        let key = if meta.app_id.is_empty() {
            meta.title
        } else {
            meta.app_id
        };
        items.push(DockItem::Focus {
            window: window.clone(),
            key,
        });
    }
    // Minimized windows are unmapped but remain launchable from the dock.
    for (window, props) in &state.props {
        if !props.minimized {
            continue;
        }
        let meta = state.window_meta(window);
        let key = if meta.app_id.is_empty() {
            meta.title
        } else {
            meta.app_id
        };
        items.push(DockItem::Focus {
            window: window.clone(),
            key,
        });
    }
    items
}

/// Number of dock items without building the (allocating) list. Used on the
/// pointer-motion path, which only needs the count for hit testing; must match
/// `dock_items(...).len()`.
pub fn dock_item_count(state: &State) -> usize {
    let minimized = state.props.values().filter(|p| p.minimized).count();
    let running = state
        .space
        .elements()
        .filter(|window| state.window_has_content(window))
        .count();
    1 + state.shell.pinned.len() + running + minimized
}

/// Keyboard events reach the handler through the same Smithay traits whether
/// they come from the winit or the libinput backend.
pub fn on_keyboard<B: InputBackend, E: KeyboardKeyEvent<B>>(state: &mut State, event: &E) {
    let serial = state.next_serial();
    let keyboard = state.keyboard.clone();
    let time = event.time() as u32;
    let pressed = event.state() == KeyState::Pressed;
    keyboard.input::<(), _>(
        state,
        event.key_code(),
        event.state(),
        serial,
        time,
        |state, mods, handle| on_key(state, mods, handle, pressed),
    );
}

fn on_key(
    state: &mut State,
    mods: &ModifiersState,
    handle: smithay::input::keyboard::KeysymHandle<'_>,
    pressed: bool,
) -> FilterResult<()> {
    let sym = handle.modified_sym();

    // Pair consumed presses with consumed releases even if the shell closes
    // between them (for example, Escape closing a popup or Super+P opening it).
    if !pressed {
        if state.swallowed_keys.remove(&sym) {
            return FilterResult::Intercept(());
        }
        return if state.shell.launcher_open {
            FilterResult::Intercept(())
        } else {
            FilterResult::Forward
        };
    }

    if state.shell.launcher_open {
        state.swallowed_keys.insert(sym);
        if sym == Keysym::Escape {
            close_launcher(state);
            return FilterResult::Intercept(());
        }
        if sym == Keysym::Return || sym == Keysym::KP_Enter {
            launch_first_filtered(state);
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

    if state.shell.quick_settings_open && sym == Keysym::Escape {
        state.swallowed_keys.insert(sym);
        state.shell.set_quick_settings_open(false);
        return FilterResult::Intercept(());
    }

    if state.shell.power_open && sym == Keysym::Escape {
        state.swallowed_keys.insert(sym);
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
        state.swallowed_keys.insert(sym);
        match action {
            config::BindingAction::Terminal => {
                let terminal = state.shell.terminal.clone();
                spawn_program(&terminal);
            }
            config::BindingAction::Launcher => {
                let w = state.output_size.w as f32;
                open_launcher(state, shell::topbar_button_rect(w, TopAction::Launcher));
            }
            config::BindingAction::ToggleTheme => {
                let w = state.output_size.w as f32;
                let rect = shell::topbar_button_rect(w, TopAction::Theme);
                state.toggle_theme(Point::from((
                    rect.x as f64 + rect.w as f64 / 2.0,
                    rect.y as f64 + rect.h as f64 / 2.0,
                )));
            }
            config::BindingAction::CloseWindow => {
                if let Some(w) = state.focused.clone() {
                    state.request_close_window(&w);
                }
            }
        }
        return FilterResult::Intercept(());
    }

    FilterResult::Forward
}

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
    let (w, h) = (state.output_size.w as f32, state.output_size.h as f32);
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

fn launch_first_filtered(state: &mut State) {
    let filtered = state.shell.filtered_apps();
    if let Some(&idx) = filtered.first() {
        let cmd = state.shell.apps[idx].command();
        spawn_program(&cmd);
    }
    close_launcher(state);
}

/// Relative pointer motion (mice and touchpads): accumulate the delta and keep
/// the cursor inside the output. Absolute devices (tablets, VMs) go through
/// [`on_pointer_motion`] directly.
pub fn on_pointer_motion_relative(state: &mut State, dx: f64, dy: f64, time: u32) {
    let max_x = (state.output_size.w as f64 - 1.0).max(0.0);
    let max_y = (state.output_size.h as f64 - 1.0).max(0.0);
    let location = Point::from((
        (state.cursor.x + dx).clamp(0.0, max_x),
        (state.cursor.y + dy).clamp(0.0, max_y),
    ));
    on_pointer_motion(state, location, time);
}

pub fn on_pointer_motion(state: &mut State, location: Point<f64, Logical>, time: u32) {
    state.cursor = location;

    // Track hovered chrome for M3 state layers.
    let (w, h) = (state.output_size.w as f32, state.output_size.h as f32);
    if state.shell.launcher_open {
        state.shell.set_hover_top(None);
        state.shell.set_hover_dock(None);
        state.shell.set_hover_power(None);
    } else {
        let top = shell::topbar_hit(w, location);
        let dock_hover = shell::dock_hit(w, h, dock_item_count(state), location);
        let power_hover = if state.shell.power_open {
            shell::power_row_at(w, location)
        } else {
            None
        };
        state.shell.set_hover_top(top);
        state.shell.set_hover_dock(dock_hover);
        state.shell.set_hover_power(power_hover);
    }

    match state.interaction.clone() {
        Interaction::Move {
            window,
            start_pointer,
            start_location,
        } => {
            let dx = (location.x - start_pointer.x).round() as i32;
            let dy = (location.y - start_pointer.y).round() as i32;
            let loc = Point::from((start_location.x + dx, start_location.y + dy));
            state.space.map_element(window, loc, false);
            state.space.refresh();
            return;
        }
        Interaction::Resize {
            window,
            edges,
            start_pointer,
            start_geometry,
        } => {
            let dx = (location.x - start_pointer.x).round() as i32;
            let dy = (location.y - start_pointer.y).round() as i32;
            let (new_geom, new_size) = compute_resize(start_geometry, edges, dx, dy);
            if let Some(toplevel) = window.toplevel() {
                toplevel.with_pending_state(|s| {
                    s.size = Some(new_size.into());
                });
                toplevel.send_configure();
            }
            state.space.map_element(window, new_geom.loc, false);
            state.space.refresh();
            return;
        }
        Interaction::Brightness => {
            if let Some(v) = shell::brightness_hit(state.output_size.w as f32, location) {
                state.brightness = v;
            }
            return;
        }
        Interaction::None => {}
    }

    let focus = state.surface_under_pointer();
    let event = MotionEvent {
        location,
        serial: state.next_serial(),
        time,
    };
    send_pointer_motion(state, focus, &event);
}

fn compute_resize(
    geom: Rectangle<i32, Logical>,
    edges: ResizeEdges,
    dx: i32,
    dy: i32,
) -> (Rectangle<i32, Logical>, (i32, i32)) {
    let min_w = 320;
    let min_h = 200;
    let mut x = geom.loc.x;
    let mut y = geom.loc.y;
    let mut w = geom.size.w;
    let mut h = geom.size.h;

    if edges.left {
        let nw = (geom.size.w - dx).max(min_w);
        x = geom.loc.x + (geom.size.w - nw);
        w = nw;
    }
    if edges.right {
        w = (geom.size.w + dx).max(min_w);
    }
    if edges.top {
        let nh = (geom.size.h - dy).max(min_h);
        y = geom.loc.y + (geom.size.h - nh);
        h = nh;
    }
    if edges.bottom {
        h = (geom.size.h + dy).max(min_h);
    }
    (Rectangle::new((x, y).into(), (w, h).into()), (w, h))
}

pub fn on_pointer_button<B: InputBackend, E: PointerButtonEvent<B>>(state: &mut State, event: &E) {
    let pressed = event.state() == ButtonState::Pressed;
    let time = event.time() as u32;
    let button = event.button_code();
    let pos = state.cursor;

    if !pressed && state.swallowed_buttons.remove(&button) {
        // A compositor-owned press must never leak an unmatched release to a
        // client. Activating on release lets every control show its press
        // animation (scale + ripple) before committing to the action.
        if button == BTN_LEFT {
            finish_compositor_press(state, pos);
        }
        return;
    }

    if button != BTN_LEFT {
        if pressed {
            state.last_interactive_press = None;
            let (w, h) = (state.output_size.w as f32, state.output_size.h as f32);
            let fullscreen = state.has_fullscreen_window();
            let in_launcher = state.shell.launcher_open;
            let in_quick_settings = !fullscreen
                && state.shell.quick_settings_open
                && shell::quick_settings_rect(w, h).contains(pos);
            if state.shell.quick_settings_open && !in_quick_settings {
                state.shell.set_quick_settings_open(false);
            }
            let in_popup = !fullscreen
                && state.shell.brightness_open
                && shell::brightness_hit(w, pos).is_some();
            let in_session_menu = !fullscreen
                && state.shell.power_open
                && shell::power_menu_rect(w).contains(pos);
            let in_topbar = !fullscreen && shell::topbar_hit(w, pos).is_some();
            let in_dock = !fullscreen && shell::dock_hit(w, h, dock_item_count(state), pos).is_some();
            let in_decoration = state.space.elements().any(|window| {
                if state.window_props(window).fullscreen || !state.window_server_decor(window) {
                    return false;
                }
                state
                    .space
                    .element_geometry(window)
                    .map(|geom| {
                        let layout = shell::decor_layout(geom);
                        layout.frame.contains(pos) && !layout.content.contains(pos)
                    })
                    .unwrap_or(false)
            });
            if in_launcher || in_quick_settings || in_popup || in_session_menu || in_topbar || in_dock || in_decoration
                || !matches!(state.interaction, Interaction::None)
            {
                state.swallowed_buttons.insert(button);
                return;
            }
        }
        let event = ButtonEvent {
            serial: state.next_serial(),
            time,
            button,
            state: if pressed { ButtonState::Pressed } else { ButtonState::Released },
        };
        send_pointer_button(state, &event);
        return;
    }

    if pressed {
        let serial = state.next_serial();
        state.last_interactive_press = None;

        let (w, h) = (state.output_size.w as f32, state.output_size.h as f32);
        let fullscreen = state.has_fullscreen_window();

        if state.shell.launcher_open {
            state.swallowed_buttons.insert(button);
            let filtered = state.shell.filtered_apps();
            if let LauncherHit::Tile(slot) = shell::launcher_hit(w, h, filtered.len(), pos) {
                // The ripple is drawn on the tile-sized press overlay, so its
                // centre is tile-local; screen coordinates would land outside
                // the overlay and clip the ripple away.
                let center = (
                    config::LAUNCHER_TILE / 2.0,
                    config::LAUNCHER_TILE / 2.0,
                );
                state.shell.press_begin(
                    PressKey::Launcher(slot),
                    center,
                    config::LAUNCHER_TILE * 0.72,
                    0.92,
                );
                state.shell.press_launch = Some(slot);
            }
            return;
        }

        if !fullscreen && state.shell.quick_settings_open {
            if shell::quick_settings_rect(w, h).contains(pos) {
                state.swallowed_buttons.insert(button);
                let rect = shell::quick_settings_rect(w, h);
                let pad = shell::SHADOW_PAD;
                if let Some(action) =
                    shell::quick_settings_hit(w, h, pos, state.shell.quick_settings_progress())
                {
                    let i = shell::quick_action_index(action);
                    let tile = shell::quick_setting_tile_rect(w, h, i);
                    let center = (
                        tile.x - rect.x + pad + tile.w / 2.0,
                        tile.y - rect.y + pad + tile.h / 2.0,
                    );
                    state.shell.press_begin(
                        PressKey::Tile(i),
                        center,
                        tile.w.max(tile.h) * 0.7,
                        0.94,
                    );
                    state.shell.press_tile = Some(action);
                } else if let Some(i) = shell::palette_chip_hit(w, h, pos) {
                    if let Some(chip) = shell::palette_chip_rect(w, h, i) {
                        let center = (
                            chip.x - rect.x + pad + chip.w / 2.0,
                            chip.y - rect.y + pad + chip.h / 2.0,
                        );
                        state.shell.press_begin(
                            PressKey::Chip(i),
                            center,
                            chip.w.max(chip.h) * 0.7,
                            0.96,
                        );
                        state.shell.press_chip = Some(i);
                    }
                }
                return;
            }
            if shell::topbar_hit(w, pos) != Some(TopAction::QuickSettings) {
                state.shell.set_quick_settings_open(false);
            }
        }

        // Session menu: a press previews a row, the action fires on release.
        if !fullscreen && state.shell.power_open {
            if let Some(i) = shell::power_row_at(w, pos) {
                state.swallowed_buttons.insert(button);
                state.shell.press_power = Some(i);
                let row = shell::power_row_rect(w, i);
                let card = shell::power_menu_rect(w);
                let pad = shell::SHADOW_PAD;
                let center = (
                    row.x - card.x + pad + 28.0,
                    row.y - card.y + pad + row.h / 2.0,
                );
                state
                    .shell
                    .press_begin(PressKey::Power(i), center, row.h, 0.94);
                return;
            }
            if shell::power_menu_rect(w).contains(pos) {
                // Clicks on the card's padding must not reach what is behind.
                state.swallowed_buttons.insert(button);
                return;
            }
            if shell::topbar_hit(w, pos) != Some(TopAction::Power) {
                state.shell.power_open = false;
                state.shell.set_power_open(false);
            }
        }

        // Brightness popup: a press on the slider starts a drag.
        if !fullscreen && state.shell.brightness_open {
            if let Some(v) = shell::brightness_hit(w, pos) {
                state.brightness = v;
                state.interaction = Interaction::Brightness;
                state.shell.set_slider_active(true);
                return;
            }
        }

        if !fullscreen {
            if let Some(action) = shell::topbar_hit(w, pos) {
                state.swallowed_buttons.insert(button);
                state.shell.press_top = Some(action);
                let rect = shell::topbar_button_rect(w, action);
                state.shell.press_begin(
                    PressKey::Top(action),
                    (rect.x + rect.w / 2.0, rect.y + rect.h / 2.0),
                    rect.w * 0.72,
                    0.86,
                );
                if action != TopAction::Brightness {
                    state.shell.brightness_open = false;
                    state.shell.set_brightness_open(false);
                }
                if action != TopAction::Power {
                    state.shell.power_open = false;
                    state.shell.set_power_open(false);
                }
                return;
            }
        }

        // A click anywhere else dismisses the popups.
        if !fullscreen {
            state.shell.brightness_open = false;
            state.shell.set_brightness_open(false);
            state.shell.power_open = false;
            state.shell.set_power_open(false);
        }

        let item_count = dock_item_count(state);
        if !fullscreen {
            if let Some(i) = shell::dock_hit(w, h, item_count, pos) {
                state.swallowed_buttons.insert(button);
                state.shell.press_dock = Some(i);
                let rect = shell::dock_item_rect(w, h, item_count, i);
                let dock = shell::dock_rect(w, h, item_count);
                state.shell.press_begin(
                    PressKey::Dock(i),
                    (rect.x + rect.w / 2.0, rect.y - dock.y + rect.h / 2.0),
                    rect.w * 0.75,
                    0.88,
                );
                return;
            }
        }

        // The decoration frame lives ABOVE the client content, so
        // `space.element_under` (content only) is not enough: walk every window
        // from the top of the stack and hit-test its full frame.
        let windows: Vec<_> = state.space.elements().cloned().collect();
        for window in windows.into_iter().rev() {
            // Toplevels can be announced before they attach their first buffer.
            // The renderer omits such windows, so they must not intercept input.
            if !state.window_has_content(&window) {
                continue;
            }
            let Some(geom) = state.space.element_geometry(&window) else {
                continue;
            };
            // A closing window is still visible for a few frames while it
            // animates out; it must not accept input any more.
            if state.shell.is_closing(&window) {
                continue;
            }
            // Fullscreen windows own the output, and CSD clients draw their own
            // title bar: neither has a frame of ours to hit-test, so only the
            // client content takes input.
            if state.window_props(&window).fullscreen || !state.window_server_decor(&window) {
                let inside = pos.x >= geom.loc.x as f64
                    && pos.y >= geom.loc.y as f64
                    && pos.x < (geom.loc.x + geom.size.w) as f64
                    && pos.y < (geom.loc.y + geom.size.h) as f64;
                if !inside {
                    continue;
                }
                let local = pos - state.window_surface_origin(&window).to_f64();
                let Some((surface, surface_loc)) =
                    window.surface_under(local, WindowSurfaceType::ALL)
                else {
                    // A transparent/input-excluded part of a normal window is
                    // not a hit. Keep searching lower windows instead of
                    // swallowing the click. Fullscreen still owns the output.
                    if fullscreen {
                        state.swallowed_buttons.insert(button);
                        return;
                    }
                    continue;
                };
                state.space.raise_element(&window, false);
                state.focus_window(Some(window.clone()), serial);
                state.last_interactive_press = Some((serial, window.clone()));
                send_client_press(
                    state,
                    surface,
                    surface_loc.to_f64(),
                    pos,
                    button,
                    serial,
                    time,
                );
                return;
            }
            let layout = shell::decor_layout(geom);
            if !layout.frame.contains(pos) {
                continue;
            }
            match shell::decor_hit(&layout, pos) {
                DecorHit::Close => {
                    state.swallowed_buttons.insert(button);
                    state.request_close_window(&window);
                }
                DecorHit::Minimize => {
                    state.swallowed_buttons.insert(button);
                    state.minimize_window(&window);
                }
                DecorHit::Maximize => {
                    state.swallowed_buttons.insert(button);
                    state.toggle_maximize(&window);
                }
                DecorHit::Move => {
                    state.space.raise_element(&window, false);
                    state.focus_window(Some(window.clone()), serial);
                    state.interaction = Interaction::Move {
                        window,
                        start_pointer: pos,
                        start_location: geom.loc,
                    };
                }
                DecorHit::Resize(edges) => {
                    state.space.raise_element(&window, false);
                    state.focus_window(Some(window.clone()), serial);
                    state.interaction = Interaction::Resize {
                        window,
                        edges,
                        start_pointer: pos,
                        start_geometry: geom,
                    };
                }
                DecorHit::None => {
                    // A visible frame can cover lower windows; its border is
                    // compositor-owned. Inside the content rectangle, however,
                    // an input-region miss must fall through to lower windows.
                    if !layout.content.contains(pos) {
                        state.space.raise_element(&window, false);
                        state.focus_window(Some(window.clone()), serial);
                        state.swallowed_buttons.insert(button);
                        return;
                    }
                    let local = pos - state.window_surface_origin(&window).to_f64();
                    if let Some((surface, surface_loc)) =
                        window.surface_under(local, WindowSurfaceType::ALL)
                    {
                        state.space.raise_element(&window, false);
                        state.focus_window(Some(window.clone()), serial);
                        state.last_interactive_press = Some((serial, window.clone()));
                        send_client_press(
                            state,
                            surface,
                            surface_loc.to_f64(),
                            pos,
                            button,
                            serial,
                            time,
                        );
                    } else {
                        continue;
                    }
                }
            }
            return;
        }

        state.swallowed_buttons.insert(button);
        close_launcher(state);
        state.focus_window(None, serial);
        return;
    }

    // Release. Requests carrying the serial are expected to have arrived by
    // the time the matching release is processed; do not leave it reusable.
    state.last_interactive_press = None;
    if !matches!(state.interaction, Interaction::None) {
        state.interaction = Interaction::None;
        state.shell.set_slider_active(false);
        return;
    }
    let event = ButtonEvent {
        serial: state.next_serial(),
        time,
        button,
        state: ButtonState::Released,
    };
    send_pointer_button(state, &event);
}

/// A release of a press the shell swallowed earlier: spring the visuals back
/// and, if the pointer is still over the same control, perform its action.
///
/// The launcher's scrim dismissal must only run for presses that were made
/// *while the launcher was already open*: opening it from the dock or the top
/// bar sets `consumed`, so the same release cannot instantly close it again.
pub(crate) fn finish_compositor_press(state: &mut State, pos: Point<f64, Logical>) {
    let (w, h) = (state.output_size.w as f32, state.output_size.h as f32);
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
                    let cmd = app.command();
                    spawn_program(&cmd);
                }
                Some(DockItem::Focus { window, .. }) => {
                    if state.window_props(&window).minimized {
                        state.restore_window(&window, serial);
                    } else {
                        // Raise without activating: `focus_window` owns
                        // activation and is the one that sends the configure.
                        state.space.raise_element(&window, false);
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
                    let cmd = state.shell.apps[idx].command();
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
    let w = state.output_size.w as f32;
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
            state.shell.set_qs_origin(shell::topbar_button_rect(w, TopAction::QuickSettings));
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

fn host_power_action_allowed(action: shell::PowerAction, session_mode: bool) -> bool {
    session_mode || action == shell::PowerAction::Logout
}

/// Run one of the session-menu actions.
pub(crate) fn perform_power_action(state: &mut State, action: shell::PowerAction) {
    state.shell.power_open = false;
    state.shell.set_power_open(false);
    let session_mode = SESSION_MODE.load(std::sync::atomic::Ordering::Relaxed);
    if !host_power_action_allowed(action, session_mode) {
        // Nested mode is only a development window. Never let its controls
        // shut down or reboot the host session that contains it.
        tracing::warn!(?action, "host power action ignored in nested mode");
        return;
    }
    match action {
        shell::PowerAction::PowerOff => spawn_system(&["systemctl", "poweroff"]),
        shell::PowerAction::Restart => spawn_system(&["systemctl", "reboot"]),
        shell::PowerAction::Logout => state.running = false,
    }
}

/// Spawn a host system command (session menu), detached from the compositor.
fn spawn_system(args: &[&str]) {
    let Some((program, rest)) = args.split_first() else {
        return;
    };
    let _ = std::process::Command::new(program)
        .args(rest)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn();
}

fn perform_quick_action(state: &mut State, action: QuickAction) {
    match action {
        QuickAction::Theme => {
            // Radial reveal grows out of the tile the user pressed.
            let (w, h) = (state.output_size.w as f32, state.output_size.h as f32);
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

#[cfg(test)]
mod tests {
    use super::*;

    fn geom(x: i32, y: i32, w: i32, h: i32) -> Rectangle<i32, Logical> {
        Rectangle::new((x, y).into(), (w, h).into())
    }

    fn edges(left: bool, right: bool, top: bool, bottom: bool) -> ResizeEdges {
        ResizeEdges {
            left,
            right,
            top,
            bottom,
        }
    }

    #[test]
    fn nested_mode_never_runs_host_power_commands() {
        assert!(!host_power_action_allowed(shell::PowerAction::PowerOff, false));
        assert!(!host_power_action_allowed(shell::PowerAction::Restart, false));
        assert!(host_power_action_allowed(shell::PowerAction::Logout, false));
    }

    #[test]
    fn real_session_allows_power_actions() {
        assert!(host_power_action_allowed(shell::PowerAction::PowerOff, true));
        assert!(host_power_action_allowed(shell::PowerAction::Restart, true));
    }

    #[test]
    fn resize_right_grows_width_keeps_origin() {
        let (g, size) =
            compute_resize(geom(100, 100, 400, 300), edges(false, true, false, false), 50, 0);
        assert_eq!(size, (450, 300));
        assert_eq!(g.loc, (100, 100).into());
    }

    #[test]
    fn resize_left_keeps_right_edge_fixed() {
        let (g, size) =
            compute_resize(geom(100, 100, 400, 300), edges(true, false, false, false), 50, 0);
        assert_eq!(size, (350, 300));
        assert_eq!(g.loc.x, 150);
        assert_eq!(g.loc.x + g.size.w, 500, "right edge must not move");
    }

    #[test]
    fn resize_top_keeps_bottom_edge_fixed() {
        let (g, size) =
            compute_resize(geom(100, 100, 400, 300), edges(false, false, true, false), 0, 30);
        assert_eq!(size, (400, 270));
        assert_eq!(g.loc.y, 130);
        assert_eq!(g.loc.y + g.size.h, 400, "bottom edge must not move");
    }

    #[test]
    fn resize_clamps_to_minimum_size() {
        let (g, size) =
            compute_resize(geom(100, 100, 400, 300), edges(true, false, false, false), 1000, 0);
        assert_eq!(size.0, 320);
        assert_eq!(g.loc.x + g.size.w, 500, "right edge stays fixed when clamped");

        let (_, size) =
            compute_resize(geom(100, 100, 400, 300), edges(false, false, false, true), 0, -1000);
        assert_eq!(size.1, 200);
    }

    #[test]
    fn spawn_ignores_empty_command() {
        // No program name: must be a no-op rather than a panic/index error.
        spawn_program(&[]);
    }
}
