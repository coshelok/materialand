//! Geometry.
use super::*;

impl State {
    pub(super) fn surface_space(&self, window: &Window) -> super::coordinates::SurfaceSpace {
        super::coordinates::SurfaceSpace::new(
            self.windows
                .space
                .element_location(window)
                .unwrap_or_default(),
            window.geometry().loc,
        )
    }
    pub fn window_surface_origin(&self, window: &Window) -> Point<i32, Logical> {
        self.surface_space(window).origin()
    }

    pub fn window_meta(&self, window: &Window) -> WindowMeta {
        self.windows
            .meta
            .get(window)
            .cloned()
            .unwrap_or_else(|| default_meta(window))
    }

    pub fn window_props(&self, window: &Window) -> WindowProps {
        self.windows.props.get(window).copied().unwrap_or_default()
    }

    pub fn window_has_content(&self, window: &Window) -> bool {
        let Some(toplevel) = window.toplevel() else {
            return true;
        };
        smithay::backend::renderer::utils::with_renderer_surface_state(
            toplevel.wl_surface(),
            |state| state.buffer().is_some(),
        )
        .unwrap_or(false)
    }

    pub fn window_server_decor(&self, window: &Window) -> bool {
        use smithay::reexports::wayland_protocols::xdg::decoration::zv1::server::zxdg_toplevel_decoration_v1::Mode;
        window
            .toplevel()
            .map(|toplevel| toplevel.current_state().decoration_mode == Some(Mode::ServerSide))
            .unwrap_or(false)
    }

    pub(super) fn props_mut(&mut self, window: &Window) -> &mut WindowProps {
        self.windows.props.entry(window.clone()).or_default()
    }

    pub fn has_fullscreen_window(&self) -> bool {
        self.windows
            .space
            .elements()
            .rev()
            .find(|window| self.window_has_content(window))
            .is_some_and(|window| self.window_props(window).fullscreen)
    }

    pub fn available_area(&self) -> Rectangle<i32, Logical> {
        let design = crate::config::design();
        let top = (design.topbar.height + 2.0 * design.topbar.margin) as i32;
        let bottom = (design.dock.height + design.dock.margin) as i32
            + design.window.workspace_bottom_gap as i32;
        let h = (self.display.size.h - top - bottom).max(1);
        Rectangle::new((0, top).into(), (self.display.size.w, h).into())
    }

    /// Leave room for our title bar outside the client's maximized geometry.
    pub fn window_available_area(&self, window: &Window) -> Rectangle<i32, Logical> {
        super::placement::content_area(self.available_area(), self.window_server_decor(window))
    }

    pub fn window_frame_rect(&self, window: &Window) -> Option<crate::shell::RectF> {
        let geometry = self.windows.space.element_geometry(window)?;
        Some(
            if !self.window_props(window).fullscreen && self.window_server_decor(window) {
                crate::shell::decor_layout(geometry).frame
            } else {
                crate::shell::RectF {
                    x: geometry.loc.x as f32,
                    y: geometry.loc.y as f32,
                    w: geometry.size.w as f32,
                    h: geometry.size.h as f32,
                }
            },
        )
    }
    pub fn output_rect(&self) -> Rectangle<i32, Logical> {
        Rectangle::new(
            (0, 0).into(),
            (self.display.size.w, self.display.size.h).into(),
        )
    }

    pub fn initial_window_size(&self) -> Size<i32, Logical> {
        let design = crate::config::design();
        let area = self.available_area();
        let (w, h) = (design.window.width as i32, design.window.height as i32);
        (
            w.min((area.size.w - design.window.initial_spare_width as i32).max(1)),
            h.min(
                (area.size.h
                    - design.window.titlebar_height as i32
                    - design.window.initial_spare_height as i32)
                    .max(1),
            ),
        )
            .into()
    }

    pub fn next_location(&mut self) -> Point<i32, Logical> {
        let design = crate::config::design();
        let n = self
            .windows
            .cascade
            .rem_euclid(design.window.cascade_count as i32);
        self.windows.cascade = self.windows.cascade.wrapping_add(1);
        let step = design.window.cascade_step as i32;
        let base_x = design.topbar.margin as i32 + design.window.spawn_x as i32;
        let base_y = (design.topbar.height
            + 2.0 * design.topbar.margin
            + design.window.titlebar_height
            + design.window.spawn_gap) as i32;
        let (sw, sh) = (self.display.size.w, self.display.size.h);
        let Size { w: ww, h: wh, .. } = self.initial_window_size();
        let dock = design.dock.height as i32 + design.dock.margin as i32;
        let max_x = (sw - ww - design.window.edge_margin as i32).max(base_x);
        let max_y = (sh - wh - dock - design.window.edge_margin as i32).max(base_y);
        Point::from((
            (base_x + n * step).min(max_x),
            (base_y + n * step).min(max_y),
        ))
    }
}
