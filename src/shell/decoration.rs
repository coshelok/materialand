//! Decoration.
use super::*;

impl Shell {
    pub fn render_decoration(
        &mut self,
        theme: &Theme,
        fonts: &mut Fonts,
        window: &Window,
        window_geom: Rectangle<i32, Logical>,
        params: DecorParams,
        mask: Option<CircleMask>,
    ) -> (MemoryRenderBuffer, Point<f64, Logical>) {
        let DecorParams {
            meta,
            focused,
            maximized,
        } = params;
        let l = decor_layout(window_geom);
        let loc = Point::from((
            l.frame.x as f64 - SHADOW_PAD as f64,
            l.frame.y as f64 - SHADOW_PAD as f64,
        ));
        // Only the *size* affects the painted pixels; the position is applied
        // by the caller through the returned `loc`. Including it here would
        // invalidate (and re-rasterise) the frame on every drag motion.
        let sig = KeyHash::new()
            .i32(window_geom.size.w)
            .i32(window_geom.size.h)
            .str(&meta.title)
            .str(&meta.app_id)
            .bool(focused)
            .bool(maximized)
            .u64(theme.cache_key())
            .finish();
        if let Some(entries) = self.decor_cache.get_mut(window) {
            if let Some(entry) = entries.iter_mut().find(|e| e.key == sig) {
                entry.epoch = self.frame_epoch;
                return masked_from(&entry.surface, mask, &entry.buffer, loc);
            }
        }
        let pw = l.frame.w.ceil() as u32 + 2 * SHADOW_PAD as u32;
        let ph = l.frame.h.ceil() as u32 + 2 * SHADOW_PAD as u32;
        let mut s = Surface::new(pw, ph);
        // Soft elevation shadow under the floating frame.
        s.shadow_round_rect(
            SHADOW_PAD,
            SHADOW_PAD,
            l.frame.w,
            l.frame.h,
            config::FRAME_RADIUS,
            SHADOW_SPREAD,
        );

        // Focus raises the frame to a higher Material elevation.
        let surface = theme.surface_for_elevation(if focused { 3 } else { 2 });
        let on = if focused {
            theme.on_surface
        } else {
            Theme::with_alpha(theme.on_surface, 0.7)
        };

        s.fill_round_rect(
            SHADOW_PAD,
            SHADOW_PAD,
            l.frame.w,
            l.frame.h,
            config::FRAME_RADIUS,
            surface,
        );
        s.stroke_round_rect(
            SHADOW_PAD + 0.5,
            SHADOW_PAD + 0.5,
            l.frame.w - 1.0,
            l.frame.h - 1.0,
            config::FRAME_RADIUS,
            1.0,
            Theme::with_alpha(theme.outline, if focused { 0.35 } else { 0.2 }),
        );

        // Window controls: minimize, maximize/restore, close.
        let cc = (
            l.close.0 - l.frame.x + SHADOW_PAD,
            l.close.1 - l.frame.y + SHADOW_PAD,
        );
        let mc = (
            l.maximize.0 - l.frame.x + SHADOW_PAD,
            l.maximize.1 - l.frame.y + SHADOW_PAD,
        );
        let nc = (
            l.minimize.0 - l.frame.x + SHADOW_PAD,
            l.minimize.1 - l.frame.y + SHADOW_PAD,
        );

        let neutral = theme.surface_container_highest;
        let neutral_on = theme.on_surface_variant;
        s.circle(nc.0, nc.1, 13.0, neutral);
        fonts.draw_icon_centered(&mut s, nc.0, nc.1, "remove", 16.0, neutral_on);
        s.circle(mc.0, mc.1, 13.0, neutral);
        fonts.draw_icon_centered(
            &mut s,
            mc.0,
            mc.1,
            if maximized {
                "filter_none"
            } else {
                "crop_square"
            },
            14.0,
            neutral_on,
        );
        s.circle(cc.0, cc.1, 13.0, Theme::with_alpha(theme.error, 0.9));
        fonts.draw_icon_centered(&mut s, cc.0, cc.1, "close", 16.0, theme.on_error);

        let key = if meta.app_id.is_empty() {
            meta.title.as_str()
        } else {
            meta.app_id.as_str()
        };
        let mid_y = SHADOW_PAD + l.titlebar.h / 2.0;
        fonts.draw_icon_centered(
            &mut s,
            SHADOW_PAD + 26.0,
            mid_y,
            config::icon_for_key(key),
            20.0,
            on,
        );
        // The focused window's title carries extra weight.
        let title_style = if focused {
            TextStyle {
                kind: FontKind::Bold,
                ..TextStyle::TITLE_SMALL
            }
        } else {
            TextStyle::TITLE_SMALL
        };
        fonts.draw_text_role(
            &mut s,
            SHADOW_PAD + 46.0,
            mid_y - title_style.line_height / 2.0,
            title_style,
            &truncate(&meta.title, 42),
            on,
        );

        let buffer = s.to_memory_buffer();
        let result = masked_from(&s, mask, &buffer, loc);
        let epoch = self.frame_epoch;
        let entries = self.decor_cache.entry(window.clone()).or_default();
        entries.push(DecorEntry {
            key: sig,
            surface: s,
            buffer,
            epoch,
        });
        if entries.len() > 4 {
            entries.remove(0);
        }
        result
    }
}
