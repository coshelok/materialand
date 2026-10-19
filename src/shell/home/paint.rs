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
        let layout = home_layout(w, h);
        let mut hash = KeyHash::new()
            .u64(theme.cache_key())
            .u32(w as u32)
            .u32(h as u32)
            .u32(self.home.page as u32)
            .str(&self.clock)
            .str(&self.calendar.now.date_text())
            .u32(self.home.position.value().to_bits())
            .u32(self.home.reveal.value().to_bits())
            .str(&format!("{:?}", self.home.pressed));
        for (i, app) in self.home.apps.iter().enumerate() {
            hash = hash
                .str(&app.identity())
                .str(&app.name)
                .str(&app.icon)
                .u32(self.press_scale(PressKey::Home(i)).to_bits());
        }
        let key = hash.finish();
        if self.home.cache.as_ref().is_none_or(|l| l.key != key) {
            let mut s = Surface::new(layout.area.w.ceil() as u32, layout.area.h.ceil() as u32);
            let x = 16.0;
            let y = self.home_offset();
            let large = TextStyle {
                size: if layout.area.h > 430.0 { 64.0 } else { 40.0 },
                line_height: 72.0,
                tracking: 0.0,
                kind: FontKind::Regular,
            };
            fonts.draw_text_role(&mut s, x, y, large, &self.clock, theme.on_surface);
            if layout.area.h > 430.0 {
                fonts.draw_text_role(
                    &mut s,
                    x,
                    y + 82.0,
                    TextStyle::TITLE_MEDIUM,
                    &self.calendar.now.date_text(),
                    theme.on_surface_variant,
                );
            }
            let mut grid = Surface::new(layout.grid.w.ceil() as u32, layout.grid.h.ceil() as u32);
            for (i, app) in self.home.apps.iter().enumerate() {
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
                        24.0,
                        Theme::with_alpha(theme.primary, 0.10),
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
                    s.fill_round_rect(
                        r.x - layout.area.x,
                        r.y - layout.area.y,
                        r.w,
                        r.h,
                        18.0,
                        theme.secondary_container,
                    );
                    fonts.draw_icon_centered(
                        &mut s,
                        r.x - layout.area.x + r.w / 2.0,
                        r.y - layout.area.y + r.h / 2.0,
                        icon,
                        22.0,
                        theme.on_secondary_container,
                    );
                }
                let label = format!("{} / {}", self.home.page + 1, pages);
                let tw = fonts.text_width_styled(TextStyle::LABEL_LARGE, &label);
                fonts.draw_text_role(
                    &mut s,
                    w / 2.0 - layout.area.x - tw / 2.0,
                    layout.previous.y - layout.area.y + 8.0,
                    TextStyle::LABEL_LARGE,
                    &label,
                    theme.on_surface,
                );
            }
            let buffer = s.to_memory_buffer();
            self.home.cache = Some(CachedLayer {
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
