//! Cached app icons, fixed search and spring-driven home pages.
use super::*;
use crate::shell::placement::CardPlacement;
impl Shell {
    pub fn render_home(
        &mut self,
        theme: &Theme,
        fonts: &mut Fonts,
        w: f32,
        h: f32,
    ) -> OverlayFrame {
        let design = crate::config::design();
        let layout = home_layout(w, h);
        self.home.last_size = (w, h);
        let key = KeyHash::new()
            .u64(theme.cache_key())
            .u32(w as u32)
            .u32(h as u32)
            .u32(self.home.page as u32)
            .u32(self.home_pages(w, h) as u32)
            .bool(self.dragged_identity.is_some())
            .u64(self.home.drop_hint.map_or(u64::MAX, |v| v as u64))
            .str(self.shortcut_error.as_deref().unwrap_or(""))
            .finish();
        if !self.home.cache.select(key) {
            let mut s = Surface::new(layout.area.w.ceil() as u32, layout.area.h.ceil() as u32);
            self.paint_home_widgets(&mut s, fonts, theme, layout);
            let pages = self.home_pages(w, h) + usize::from(self.dragged_identity.is_some());
            if pages > 1 {
                for (r, icon) in [
                    (layout.previous, "chevron_left"),
                    (layout.next, "chevron_right"),
                ] {
                    fonts.draw_icon_centered(
                        &mut s,
                        r.x - layout.area.x + r.w / 2.0,
                        r.y - layout.area.y + r.h / 2.0,
                        icon,
                        design.home.nav_icon_size,
                        theme.on_surface_variant,
                    );
                }
                let available =
                    (layout.next.x - layout.previous.right() - 2.0 * design.home.page_dot_gap)
                        .max(design.home.page_dot_gap);
                let visible =
                    pages.min((available / design.home.page_dot_gap).floor().max(1.0) as usize);
                let first = self
                    .home
                    .page
                    .saturating_sub(visible / 2)
                    .min(pages - visible);
                for i in 0..visible {
                    let active = first + i == self.home.page;
                    s.circle(
                        w / 2.0 - layout.area.x
                            + (i as f32 - (visible - 1) as f32 / 2.0) * design.home.page_dot_gap,
                        layout.previous.y - layout.area.y + design.home.nav_height / 2.0,
                        design.home.page_dot_radius,
                        if active {
                            theme.on_surface
                        } else {
                            theme.outline_variant
                        },
                    );
                }
            }
            if let Some(error) = &self.shortcut_error {
                let text = fit_text(fonts, TextStyle::body_small(), error, layout.area.w - 32.0);
                fonts.draw_text_role(
                    &mut s,
                    16.0,
                    layout.area.h - 24.0,
                    TextStyle::body_small(),
                    &text,
                    theme.error,
                );
            }
            let buffer = s.to_memory_buffer();
            self.home.cache.store(CachedLayer {
                key,
                surface: s,
                buffer,
            });
        }
        let layer = self.home.cache.as_ref().expect("home layer");
        let mut frame = OverlayFrame::single(
            layer.buffer.clone(),
            (f64::from(layout.area.x), f64::from(layout.area.y)).into(),
            None,
            1.0,
        );
        let placement = CardPlacement {
            source: layout.grid,
            target: layout.grid,
        };
        let cells = self.home.cells(layout);
        for i in 0..self.home.apps.len() {
            let app = self.home.apps[i].clone();
            if self.dragged_identity.as_deref() == Some(app.identity().as_str()) {
                continue;
            }
            let rect = self.home_tile_in(layout, &cells, i);
            if rect.right() <= layout.grid.x
                || rect.x >= layout.grid.right()
                || rect.bottom() <= layout.grid.y
                || rect.y >= layout.grid.bottom()
            {
                continue;
            }
            let buffer = self.app_tile(
                fonts,
                theme,
                &app,
                self.press_scale(PressKey::Home(i)),
                self.home.pressed == Some(HomeHit::App(i)),
            );
            if let Some(layer) = LayerFrame::clipped(buffer, rect, layout.grid, placement) {
                frame.pieces.push((layer, 1.0));
            }
        }
        frame
    }
}
