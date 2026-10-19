//! A single entry point applies display changes and reflows window policy.
use super::*;
impl State {
    pub fn output_identity_matches(&self, name: &str, properties: &PhysicalProperties) -> bool {
        let current = self.display.handle.physical_properties();
        self.display.handle.name() == name
            && current.size == properties.size
            && current.subpixel == properties.subpixel
            && current.make == properties.make
            && current.model == properties.model
    }

    /// Physical properties are immutable in Smithay: a changed monitor gets a
    /// new wl_output, while returning to the same monitor retains its identity.
    pub fn set_output_identity(&mut self, name: String, properties: PhysicalProperties) {
        if self.output_identity_matches(&name, &properties) {
            return;
        }
        let previous = self.display.handle.clone();
        let next = Output::new(name, properties);
        next.change_current_state(
            previous.current_mode(),
            Some(previous.current_transform()),
            Some(previous.current_scale()),
            Some(previous.current_location()),
        );
        if let Some(mode) = previous.preferred_mode() {
            next.set_preferred(mode);
        }
        let global = next.create_global::<Self>(&self.display_handle);
        self.windows.space.unmap_output(&previous);
        self.windows.space.map_output(&next, (0, 0));
        self.display.handle = next;
        let old_global = std::mem::replace(&mut self._output_global, global);
        self.display_handle
            .disable_global::<Self>(old_global.clone());
        self.retired_output_globals.push(old_global);
        self.windows.space.refresh();
    }

    pub fn set_output_mode(&mut self, mode: Mode) {
        if mode.size.w <= 0 || mode.size.h <= 0 || self.display.handle.current_mode() == Some(mode)
        {
            return;
        }
        self.display.size = mode.size.to_logical(1);
        self.display.handle.change_current_state(
            Some(mode),
            Some(Transform::Normal),
            Some(Scale::Integer(1)),
            Some((0, 0).into()),
        );
        self.display.handle.set_preferred(mode);
        self.cursor.x = self
            .cursor
            .x
            .clamp(0.0, f64::from((self.display.size.w - 1).max(0)));
        self.cursor.y = self
            .cursor
            .y
            .clamp(0.0, f64::from((self.display.size.h - 1).max(0)));
        self.windows.pending_resize.clear();
        let windows: Vec<_> = self.windows.props.keys().cloned().collect();
        for window in windows {
            let props = self.window_props(&window);
            let area = if props.fullscreen {
                Some(self.output_rect())
            } else if props.maximized {
                Some(self.window_available_area(&window))
            } else {
                None
            };
            if let Some(area) = area {
                if let Some(top) = window.toplevel() {
                    top.with_pending_state(|s| s.size = Some(area.size));
                    top.send_configure();
                }
                if props.minimized {
                    self.props_mut(&window).minimized_location = Some(area.loc);
                } else if self.windows.space.element_location(&window).is_some() {
                    self.windows.space.map_element(window, area.loc, false);
                }
            } else if let Some(location) = self.windows.space.element_location(&window) {
                let constrained = Point::from((
                    location.x.clamp(0, (self.display.size.w - 64).max(0)),
                    location.y.clamp(40, (self.display.size.h - 48).max(40)),
                ));
                self.windows.space.map_element(window, constrained, false);
            }
        }
        self.windows.space.refresh();
        self.shell.mark_dirty();
    }
}
