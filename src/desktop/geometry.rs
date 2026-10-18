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
            .any(|window| self.window_props(window).fullscreen)
    }

    pub fn available_area(&self) -> Rectangle<i32, Logical> {
        let top = (config::TOPBAR_HEIGHT + 2.0 * config::TOPBAR_MARGIN) as i32;
        let bottom = (config::DOCK_HEIGHT + config::DOCK_MARGIN) as i32 + 8;
        let h = (self.display.size.h - top - bottom).max(200);
        Rectangle::new((0, top).into(), (self.display.size.w, h).into())
    }

    pub fn output_rect(&self) -> Rectangle<i32, Logical> {
        Rectangle::new(
            (0, 0).into(),
            (self.display.size.w, self.display.size.h).into(),
        )
    }

    pub fn next_location(&mut self) -> Point<i32, Logical> {
        let n = self.windows.cascade.rem_euclid(6);
        self.windows.cascade = self.windows.cascade.wrapping_add(1);
        let step = config::CASCADE_STEP;
        let base_x = config::TOPBAR_MARGIN as i32 + 48;
        let base_y = config::TOPBAR_HEIGHT as i32 + config::TOPBAR_MARGIN as i32 + 8;
        let (sw, sh) = (self.display.size.w, self.display.size.h);
        let (ww, wh) = config::DEFAULT_WINDOW_SIZE;
        let dock = config::DOCK_HEIGHT as i32 + config::DOCK_MARGIN as i32;
        let max_x = (sw - ww - 24).max(base_x);
        let max_y = (sh - wh - dock - 24).max(base_y);
        Point::from((
            (base_x + n * step).min(max_x),
            (base_y + n * step).min(max_y),
        ))
    }
}
