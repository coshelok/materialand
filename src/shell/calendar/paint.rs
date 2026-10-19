//! Cached date, time and calendar raster layer.
use super::*;
impl Shell {
    pub fn render_calendar(
        &mut self,
        theme: &Theme,
        fonts: &mut Fonts,
        w: f32,
        h: f32,
        mask: Option<CircleMask>,
    ) -> OverlayFrame {
        let card = calendar_rect(w, h);
        let pad = SHADOW_PAD;
        let key = KeyHash::new()
            .u64(theme.cache_key())
            .str(&format!("{:?}", self.calendar.now))
            .i32(self.calendar.year)
            .u32(self.calendar.month)
            .u32(card.w as u32)
            .u32(card.h as u32)
            .i32(self.calendar.pressed.map_or(-1, |a| a as i32))
            .finish();
        if self.calendar_layer.as_ref().is_none_or(|l| l.key != key) {
            let mut s = Surface::new(
                (card.w + 2.0 * pad).ceil() as u32,
                (card.h + 2.0 * pad).ceil() as u32,
            );
            s.shadow_round_rect(pad, pad, card.w, card.h, shape::EXTRA_LARGE, SHADOW_SPREAD);
            s.fill_round_rect(
                pad,
                pad,
                card.w,
                card.h,
                shape::EXTRA_LARGE,
                theme.surface_container_low,
            );
            let x = pad + 24.0;
            let now = &self.calendar.now;
            fonts.draw_text_role(
                &mut s,
                x,
                pad + 18.0,
                TextStyle::LABEL_LARGE,
                "Date & time",
                theme.on_surface_variant,
            );
            let large = TextStyle {
                size: 42.0,
                line_height: 52.0,
                tracking: 0.0,
                kind: FontKind::Regular,
            };
            fonts.draw_text_role(
                &mut s,
                x,
                pad + 44.0,
                large,
                &now.seconds_text(),
                theme.on_surface,
            );
            let date = fit_text(
                fonts,
                TextStyle::BODY_MEDIUM,
                &now.date_text(),
                card.w - 48.0,
            );
            fonts.draw_text_role(
                &mut s,
                x,
                pad + 102.0,
                TextStyle::BODY_MEDIUM,
                &date,
                theme.on_surface,
            );
            fonts.draw_text_role(
                &mut s,
                x,
                pad + 126.0,
                TextStyle::LABEL_MEDIUM,
                &now.zone,
                theme.on_surface_variant,
            );
            let month = format!(
                "{} {}",
                MONTHS[(self.calendar.month - 1) as usize],
                self.calendar.year
            );
            fonts.draw_text_role(
                &mut s,
                x,
                pad + 158.0,
                TextStyle::TITLE_SMALL,
                &month,
                theme.on_surface,
            );
            for (action, icon) in [
                (CalendarAction::Previous, "chevron_left"),
                (CalendarAction::Next, "chevron_right"),
            ] {
                let Some(r) = nav_rect(card, action) else {
                    continue;
                };
                let (rx, ry) = (r.x - card.x + pad, r.y - card.y + pad);
                let color = if self.calendar.pressed == Some(action) {
                    theme.primary_container
                } else {
                    theme.secondary_container
                };
                s.fill_round_rect(rx, ry, r.w, r.h, r.h / 2.0, color);
                fonts.draw_icon_centered(
                    &mut s,
                    rx + r.w / 2.0,
                    ry + r.h / 2.0,
                    icon,
                    22.0,
                    theme.on_surface,
                );
            }

            let cell = (card.w - 40.0) / 7.0;
            let row_h = ((card.h - 256.0) / 6.0).clamp(10.0, 32.0);
            for (i, label) in ["M", "T", "W", "T", "F", "S", "S"].iter().enumerate() {
                let tw = fonts.text_width_styled(TextStyle::LABEL_MEDIUM, label);
                fonts.draw_text_role(
                    &mut s,
                    pad + 20.0 + (i as f32 + 0.5) * cell - tw / 2.0,
                    pad + 196.0,
                    TextStyle::LABEL_MEDIUM,
                    label,
                    theme.on_surface_variant,
                );
            }
            let first = first_weekday(self.calendar.year, self.calendar.month);
            for day in 1..=days_in_month(self.calendar.year, self.calendar.month) {
                let index = first + day as usize - 1;
                let cx = pad + 20.0 + (index % 7) as f32 * cell + cell / 2.0;
                let cy = pad + 226.0 + (index / 7) as f32 * row_h;
                let today = self.calendar.year == now.year
                    && self.calendar.month == now.month
                    && day == now.day;
                if today {
                    s.circle(cx, cy, (row_h * 0.48).min(16.0), theme.primary);
                }
                let text = day.to_string();
                let tw = fonts.text_width_styled(TextStyle::LABEL_LARGE, &text);
                fonts.draw_text_role(
                    &mut s,
                    cx - tw / 2.0,
                    cy - 10.0,
                    TextStyle::LABEL_LARGE,
                    &text,
                    if today {
                        theme.on_primary
                    } else {
                        theme.on_surface
                    },
                );
            }
            let buffer = s.to_memory_buffer();
            self.calendar_layer = Some(CachedLayer {
                key,
                surface: s,
                buffer,
            });
        }
        let cached = self.calendar_layer.as_ref().expect("calendar layer");
        let (scale, tx, ty) = self.calendar_placement(w, h).buffer_transform(pad);
        let loc = Point::from(((card.x - pad + tx) as f64, (card.y - pad + ty) as f64));
        let p = self.calendar_progress().clamp(0.0, 1.0);
        if let Some(mask) = mask {
            // Match other panels during the palette reveal.
            let mut surface = Surface::new(
                cached.surface.pixmap.width(),
                cached.surface.pixmap.height(),
            );
            surface.blit(
                &cached.surface,
                SkTransform::from_scale(scale, scale).post_translate(tx, ty),
                1.0,
                None,
            );
            let origin = Point::from(((card.x - pad) as f64, (card.y - pad) as f64));
            let (buffer, loc) =
                masked_from(&surface, Some(mask), &surface.to_memory_buffer(), origin);
            return OverlayFrame::single(buffer, loc, None, p);
        }
        OverlayFrame::single(
            cached.buffer.clone(),
            loc,
            Some(Size::from((
                (cached.surface.pixmap.width() as f32 * scale).round() as i32,
                (cached.surface.pixmap.height() as f32 * scale).round() as i32,
            ))),
            p,
        )
    }
}
