//! Window modes.
use super::*;

impl State {
    pub(super) fn set_maximized(&mut self, window: &Window, maximized: bool) {
        use smithay::reexports::wayland_protocols::xdg::shell::server::xdg_toplevel::State as XdgState;

        let props = self.window_props(window);
        if props.maximized == maximized && (!maximized || !props.fullscreen) {
            return;
        }
        let area = self.window_available_area(window);

        if maximized {
            let current = self
                .windows
                .space
                .element_geometry(window)
                .unwrap_or_else(|| window.geometry());
            let normal = props.saved.unwrap_or(current);
            {
                let p = self.props_mut(window);
                p.maximized = true;
                p.fullscreen = false;
                p.saved = Some(normal);
                p.restore_maximized = false;
            }
            if let Some(t) = window.toplevel() {
                t.with_pending_state(|s| {
                    s.states.set(XdgState::Maximized);
                    s.states.unset(XdgState::Fullscreen);
                    s.size = Some((area.size.w, area.size.h).into());
                });
                t.send_configure();
            }
            self.windows
                .space
                .map_element(window.clone(), area.loc, false);
        } else {
            if props.fullscreen {
                return;
            }
            let restore = props.saved.unwrap_or_else(|| {
                Rectangle::new((0, 0).into(), config::DEFAULT_WINDOW_SIZE.into())
            });
            {
                let p = self.props_mut(window);
                p.maximized = false;
                p.saved = None;
                p.restore_maximized = false;
            }
            if let Some(t) = window.toplevel() {
                t.with_pending_state(|s| {
                    s.states.unset(XdgState::Maximized);
                    s.states.unset(XdgState::Fullscreen);
                    s.size = Some((restore.size.w, restore.size.h).into());
                });
                t.send_configure();
            }
            self.windows
                .space
                .map_element(window.clone(), restore.loc, false);
        }
        self.windows.space.refresh();
    }

    pub(super) fn set_fullscreen(&mut self, window: &Window, fullscreen: bool) {
        use smithay::reexports::wayland_protocols::xdg::shell::server::xdg_toplevel::State as XdgState;

        let props = self.window_props(window);
        if props.fullscreen == fullscreen {
            return;
        }
        let output = self.output_rect();

        if fullscreen {
            self.shell.set_calendar_open(false);
            self.shell.close_app_menu();
            self.shell.brightness_open = false;
            self.shell.set_brightness_open(false);
            self.shell.set_quick_settings_open(false);
            self.shell.launcher_open = false;
            self.shell.set_launcher_open(false);
            let current = self
                .windows
                .space
                .element_geometry(window)
                .unwrap_or_else(|| window.geometry());
            let normal = props.saved.unwrap_or(current);
            {
                let p = self.props_mut(window);
                p.restore_maximized = props.maximized || props.restore_maximized;
                p.saved = Some(normal);
                p.fullscreen = true;
                p.maximized = false;
            }
            if let Some(t) = window.toplevel() {
                t.with_pending_state(|s| {
                    s.states.set(XdgState::Fullscreen);
                    s.states.unset(XdgState::Maximized);
                    s.size = Some((output.size.w, output.size.h).into());
                });
                t.send_configure();
            }
            self.windows
                .space
                .map_element(window.clone(), output.loc, false);
        } else if props.restore_maximized {
            let normal = props.saved.unwrap_or_else(|| {
                Rectangle::new((0, 0).into(), config::DEFAULT_WINDOW_SIZE.into())
            });
            let area = self.window_available_area(window);
            {
                let p = self.props_mut(window);
                p.fullscreen = false;
                p.maximized = true;
                p.restore_maximized = false;
                p.saved = Some(normal);
            }
            if let Some(t) = window.toplevel() {
                t.with_pending_state(|s| {
                    s.states.unset(XdgState::Fullscreen);
                    s.states.set(XdgState::Maximized);
                    s.size = Some((area.size.w, area.size.h).into());
                });
                t.send_configure();
            }
            self.windows
                .space
                .map_element(window.clone(), area.loc, false);
        } else {
            let restore = props.saved.unwrap_or_else(|| {
                Rectangle::new((0, 0).into(), config::DEFAULT_WINDOW_SIZE.into())
            });
            {
                let p = self.props_mut(window);
                p.fullscreen = false;
                p.maximized = false;
                p.restore_maximized = false;
                p.saved = None;
            }
            if let Some(t) = window.toplevel() {
                t.with_pending_state(|s| {
                    s.states.unset(XdgState::Fullscreen);
                    s.states.unset(XdgState::Maximized);
                    s.size = Some((restore.size.w, restore.size.h).into());
                });
                t.send_configure();
            }
            self.windows
                .space
                .map_element(window.clone(), restore.loc, false);
        }
        self.windows.space.refresh();
    }

    pub(super) fn set_minimized(&mut self, window: &Window, minimized: bool) {
        use smithay::reexports::wayland_protocols::xdg::shell::server::xdg_toplevel::State as XdgState;

        let props = self.window_props(window);
        if props.minimized == minimized {
            return;
        }
        let loc = if minimized {
            self.windows
                .space
                .element_location(window)
                .or(props.minimized_location)
                .unwrap_or_else(|| self.next_location())
        } else {
            props
                .minimized_location
                .or_else(|| self.windows.space.element_location(window))
                .unwrap_or_else(|| self.next_location())
        };

        if let Some(t) = window.toplevel() {
            t.with_pending_state(|s| {
                if minimized {
                    s.states.set(XdgState::Suspended);
                } else {
                    s.states.unset(XdgState::Suspended);
                }
            });
            t.send_configure();
        }
        {
            let p = self.props_mut(window);
            p.minimized = minimized;
            p.minimized_location = if minimized { Some(loc) } else { None };
        }

        if minimized {
            self.windows.space.unmap_elem(window);
            if self.windows.focused.as_ref() == Some(window) {
                // Keep `focused` intact until `focus_window` deactivates the old
                // live surface; clearing it first skips the XDG configure.
                let next = self.windows.space.elements().last().cloned();
                let serial = self.next_serial();
                self.focus_window(next, serial);
            }
        } else {
            self.windows.space.map_element(window.clone(), loc, false);
            let serial = self.next_serial();
            self.windows.space.raise_element(window, false);
            self.focus_window(Some(window.clone()), serial);
        }
        self.windows.space.refresh();
    }

    pub fn restore_window(&mut self, window: &Window, _serial: Serial) {
        self.set_minimized(window, false);
    }

    pub fn toggle_maximize(&mut self, window: &Window) {
        let maximized = self.window_props(window).maximized;
        self.set_maximized(window, !maximized);
    }

    pub fn minimize_window(&mut self, window: &Window) {
        self.set_minimized(window, true);
    }
}
