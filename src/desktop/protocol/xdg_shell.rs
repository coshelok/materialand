//! Smithay XdgShellHandler adapter.
use super::super::*;
impl XdgShellHandler for State {
    fn xdg_shell_state(&mut self) -> &mut XdgShellState {
        &mut self.xdg_shell_state
    }

    fn new_toplevel(&mut self, surface: ToplevelSurface) {
        let size = self.initial_window_size();
        surface.with_pending_state(|pending| pending.size = Some(size));
        let window = Window::new_wayland_window(surface);
        self.windows
            .meta
            .insert(window.clone(), default_meta(&window));
        self.windows.order.push(window.clone());
        self.windows.props.insert(window, WindowProps::default());
    }

    fn move_request(&mut self, surface: ToplevelSurface, _seat: wl_seat::WlSeat, serial: Serial) {
        let Some(window) = self.find_window(&surface) else {
            return;
        };
        let valid_press = self.last_interactive_press.as_ref().is_some_and(|(s, w)| {
            *s == serial && w == &window && self.windows.focused.as_ref() == Some(&window)
        });
        if !valid_press {
            return;
        }
        self.last_interactive_press = None;
        let props = self.window_props(&window);
        if props.fullscreen {
            return;
        }
        let geom = self
            .windows
            .space
            .element_geometry(&window)
            .unwrap_or_else(|| window.geometry());
        let serial = self.next_serial();
        self.windows.space.raise_element(&window, false);
        self.focus_window(Some(window.clone()), serial);
        self.interaction = Interaction::Move {
            button: self
                .pointer
                .grab_start_data()
                .map(|d| d.button)
                .unwrap_or(0x110),
            window,
            start_pointer: self.cursor,
            start_location: geom.loc,
        };
    }

    fn resize_request(
        &mut self,
        surface: ToplevelSurface,
        _seat: wl_seat::WlSeat,
        serial: Serial,
        edges: smithay::reexports::wayland_protocols::xdg::shell::server::xdg_toplevel::ResizeEdge,
    ) {
        use smithay::reexports::wayland_protocols::xdg::shell::server::xdg_toplevel::ResizeEdge;
        let Some(window) = self.find_window(&surface) else {
            return;
        };
        let valid_press = self.last_interactive_press.as_ref().is_some_and(|(s, w)| {
            *s == serial && w == &window && self.windows.focused.as_ref() == Some(&window)
        });
        if !valid_press {
            return;
        }
        self.last_interactive_press = None;
        let props = self.window_props(&window);
        if props.fullscreen || props.maximized {
            return;
        }
        let edges = ResizeEdges {
            left: matches!(
                edges,
                ResizeEdge::Left | ResizeEdge::TopLeft | ResizeEdge::BottomLeft
            ),
            right: matches!(
                edges,
                ResizeEdge::Right | ResizeEdge::TopRight | ResizeEdge::BottomRight
            ),
            top: matches!(
                edges,
                ResizeEdge::Top | ResizeEdge::TopLeft | ResizeEdge::TopRight
            ),
            bottom: matches!(
                edges,
                ResizeEdge::Bottom | ResizeEdge::BottomLeft | ResizeEdge::BottomRight
            ),
        };
        if edges == ResizeEdges::default() {
            return;
        }
        let geom = self
            .windows
            .space
            .element_geometry(&window)
            .unwrap_or_else(|| window.geometry());
        let serial = self.next_serial();
        self.windows.space.raise_element(&window, false);
        self.focus_window(Some(window.clone()), serial);
        let button = self
            .pointer
            .grab_start_data()
            .map(|d| d.button)
            .unwrap_or(0x110);
        self.begin_resize(window, edges, geom, button);
    }

    fn maximize_request(&mut self, surface: ToplevelSurface) {
        if let Some(window) = self.find_window(&surface) {
            self.set_maximized(&window, true);
        }
    }

    fn unmaximize_request(&mut self, surface: ToplevelSurface) {
        if let Some(window) = self.find_window(&surface) {
            self.set_maximized(&window, false);
        }
    }

    fn fullscreen_request(
        &mut self,
        surface: ToplevelSurface,
        _output: Option<smithay::reexports::wayland_server::protocol::wl_output::WlOutput>,
    ) {
        if let Some(window) = self.find_window(&surface) {
            self.set_fullscreen(&window, true);
        }
    }

    fn unfullscreen_request(&mut self, surface: ToplevelSurface) {
        if let Some(window) = self.find_window(&surface) {
            self.set_fullscreen(&window, false);
        }
    }

    fn minimize_request(&mut self, surface: ToplevelSurface) {
        if let Some(window) = self.find_window(&surface) {
            self.set_minimized(&window, true);
        }
    }

    fn new_popup(&mut self, surface: PopupSurface, _positioner: PositionerState) {
        if let Err(error) = self
            .popup_manager
            .track_popup(PopupKind::Xdg(surface.clone()))
        {
            tracing::warn!(?error, "could not register XDG popup surface");
            return;
        }
        // Clamp the initial placement to the output instead of accepting the
        // positioner's unconstrained geometry (menus at screen edges).
        self.unconstrain_popup(&surface);
    }

    fn grab(&mut self, surface: PopupSurface, seat: wl_seat::WlSeat, serial: Serial) {
        // `xdg_popup.grab`: hold the keyboard and pointer on the popup until
        // it is dismissed, so client menus keep input inside their surfaces.
        let Some(seat) = Seat::from_resource(&seat) else {
            return;
        };
        let kind = PopupKind::Xdg(surface);
        let Ok(root) = find_popup_root_surface(&kind) else {
            return;
        };
        match self.popup_manager.grab_popup(root, kind, &seat, serial) {
            Ok(mut grab) => {
                if let Some(keyboard) = seat.get_keyboard() {
                    // Never steal an unrelated grab that is already active.
                    if keyboard.is_grabbed()
                        && !(keyboard.has_grab(serial)
                            || keyboard.has_grab(grab.previous_serial().unwrap_or(serial)))
                    {
                        grab.ungrab(PopupUngrabStrategy::All);
                        return;
                    }
                    keyboard.set_focus(self, grab.current_grab(), serial);
                    keyboard.set_grab(self, PopupKeyboardGrab::new(&grab), serial);
                }
                if let Some(pointer) = seat.get_pointer() {
                    if pointer.is_grabbed()
                        && !(pointer.has_grab(serial)
                            || pointer
                                .has_grab(grab.previous_serial().unwrap_or_else(|| grab.serial())))
                    {
                        grab.ungrab(PopupUngrabStrategy::All);
                        return;
                    }
                    pointer.set_grab(self, PopupPointerGrab::new(&grab), serial, Focus::Keep);
                }
            }
            Err(error) => tracing::warn!(?error, "popup grab denied"),
        }
    }

    fn reposition_request(
        &mut self,
        surface: PopupSurface,
        positioner: PositionerState,
        token: u32,
    ) {
        // The client asked to move the popup (parent moved/resized or a new
        // positioner). Store the new geometry, clamp it to the output and
        // acknowledge with the token; the client commits the new position.
        let geometry = positioner.get_geometry();
        surface.with_pending_state(|state| {
            state.geometry = geometry;
            state.positioner = positioner;
        });
        self.unconstrain_popup(&surface);
        surface.send_repositioned(token);
    }

    fn title_changed(&mut self, surface: ToplevelSurface) {
        self.refresh_meta(&surface);
    }

    fn app_id_changed(&mut self, surface: ToplevelSurface) {
        self.refresh_meta(&surface);
    }

    fn toplevel_destroyed(&mut self, surface: ToplevelSurface) {
        let mut dead: Vec<Window> = self
            .windows
            .space
            .elements()
            .filter(|w| w.toplevel().map(|t| t == &surface).unwrap_or(false))
            .cloned()
            .collect();
        // Minimized windows are unmapped from the space but still tracked.
        for w in self.windows.props.keys() {
            if w.toplevel().map(|t| t == &surface).unwrap_or(false) && !dead.contains(w) {
                dead.push(w.clone());
            }
        }
        let mut focused_died = false;
        for w in &dead {
            // Drop any in-flight open/close animation for this window.
            self.shell.on_window_destroyed(w);
            self.windows.space.unmap_elem(w);
            self.windows.meta.remove(w);
            self.windows.props.remove(w);
            self.windows.order.retain(|item| item != w);
            self.windows.pending_resize.remove(w);
            if self.windows.focused.as_ref() == Some(w) {
                focused_died = true;
            }
            if self.interaction.window() == Some(w) {
                self.interaction = Interaction::None;
            }
        }
        self.windows.space.refresh();

        if focused_died {
            // Clear the dead reference first, then hand focus to the topmost
            // remaining window (`elements()` is back-to-front). `focus_window`
            // updates activation, the keyboard focus and, when nothing is left,
            // clears the focus entirely.
            let next = self.windows.space.elements().last().cloned();
            let serial = self.next_serial();
            self.focus_window(next, serial);
        }
    }
}
