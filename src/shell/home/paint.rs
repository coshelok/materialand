//! Material You clock widget, tonal app medallions and spring-driven pages.
use super::*;
impl Shell {
    pub fn render_home(
        &mut self,
        theme: &Theme,
        fonts: &mut Fonts,
        w: f32,
        h: f32,
        mask: Option<CircleMask>,
    ) -> (MemoryRenderBuffer, Point<f64, Logical>) {
        let design = crate::config::design();
        let layout = home_layout(w, h);
        let mut hash = KeyHash::new()
            .u64(theme.cache_key())
            .u32(w as u32)
            .u32(h as u32)
            .u32(self.home.page as u32)
            .str(&self.clock)
            .str(self.dragged_identity.as_deref().unwrap_or(""))
            .str(self.shortcut_error.as_deref().unwrap_or(""))
            .str(&self.calendar.now.date_text())
            .u32(self.home.position.value().to_bits())
            .u32(self.home.reveal.value().to_bits())
            .str(&format!("{:?}", self.home.pressed));
        for (i, app) in self.home.apps.iter().enumerate() {
            let rect = self.home_tile_rect(w, h, i);
            hash = hash.u32(rect.x.to_bits()).u32(rect.y.to_bits());
            hash = hash
                .str(&app.identity())
                .str(&app.name)
                .str(&app.icon)
                .u32(self.press_scale(PressKey::Home(i)).to_bits());
        }
        let key = hash.finish();
        if !self.home.cache.select(key) {
            let mut s = Surface::new(layout.area.w.ceil() as u32, layout.area.h.ceil() as u32);
            self.paint_home_widgets(&mut s, fonts, theme, layout);
            let mut grid = Surface::new(layout.grid.w.ceil() as u32, layout.grid.h.ceil() as u32);
            for (i, app) in self.home.apps.iter().enumerate() {
                if self.dragged_identity.as_deref() == Some(app.identity().as_str()) {
                    continue;
                }
                let rect = self.home_tile_rect(w, h, i);
                if rect.right() <= layout.grid.x || rect.x >= layout.grid.right() {
                    continue;
                }
                let (tx, ty) = (rect.x - layout.grid.x, rect.y - layout.grid.y);
                if self.home.pressed == Some(HomeHit::App(i)) {
                    grid.fill_round_rect(
                        tx,
                        ty,
                        rect.w,
                        rect.h,
                        design.shape.large,
                        Theme::with_alpha(theme.primary, design.state.pressed),
                    );
                }
                self.draw_launcher_tile(
                    &mut grid,
                    fonts,
                    theme,
                    app,
                    tx,
                    ty,
                    self.press_scale(PressKey::Home(i)),
                );
            }
            s.blit(
                &grid,
                SkTransform::from_translate(
                    layout.grid.x - layout.area.x,
                    layout.grid.y - layout.area.y,
                ),
                1.0,
                None,
            );
            let pages = self.home_pages(w, h);
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
        masked_from(
            &layer.surface,
            mask,
            &layer.buffer,
            (f64::from(layout.area.x), f64::from(layout.area.y)).into(),
        )
    }
}
