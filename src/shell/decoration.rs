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
        let design = crate::config::design();
        let DecorParams {
            meta,
            focused,
            maximized,
        } = params;
        let l = decor_layout(window_geom);
        let loc = Point::from((
            l.frame.x as f64 - design.shadow.padding as f64,
            l.frame.y as f64 - design.shadow.padding as f64,
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
        let pw = l.frame.w.ceil() as u32 + 2 * design.shadow.padding as u32;
        let ph = l.frame.h.ceil() as u32 + 2 * design.shadow.padding as u32;
        let mut s = Surface::new(pw, ph);
        super::decoration_frame::paint_frame(&mut s, &l, theme, focused);
        let on = if focused {
            theme.on_surface
        } else {
            Theme::with_alpha(theme.on_surface, design.window.inactive_title_alpha)
        };

        // Window controls: minimize, maximize/restore, close.
        let cc = (
            l.close.0 - l.frame.x + design.shadow.padding,
            l.close.1 - l.frame.y + design.shadow.padding,
        );
        let mc = (
            l.maximize.0 - l.frame.x + design.shadow.padding,
            l.maximize.1 - l.frame.y + design.shadow.padding,
        );
        let nc = (
            l.minimize.0 - l.frame.x + design.shadow.padding,
            l.minimize.1 - l.frame.y + design.shadow.padding,
        );

        let neutral = theme.surface_container_highest;
        let neutral_on = theme.on_surface_variant;
        s.circle(nc.0, nc.1, design.window.control_radius, neutral);
        fonts.draw_icon_centered(
            &mut s,
            nc.0,
            nc.1,
            "remove",
            design.window.control_icon,
            neutral_on,
        );
        s.circle(mc.0, mc.1, design.window.control_radius, neutral);
        fonts.draw_icon_centered(
            &mut s,
            mc.0,
            mc.1,
            if maximized {
                "filter_none"
            } else {
                "crop_square"
            },
            design.window.maximize_icon,
            neutral_on,
        );
        s.circle(
            cc.0,
            cc.1,
            design.window.control_radius,
            theme.error_container,
        );
        fonts.draw_icon_centered(
            &mut s,
            cc.0,
            cc.1,
            "close",
            design.window.control_icon,
            theme.on_error_container,
        );

        let key = if meta.app_id.is_empty() {
            meta.title.as_str()
        } else {
            meta.app_id.as_str()
        };
        let mid_y = design.shadow.padding + l.titlebar.h / 2.0;
        fonts.draw_icon_centered(
            &mut s,
            design.shadow.padding + design.window.app_icon_x,
            mid_y,
            config::icon_for_key(key),
            design.window.app_icon_size,
            on,
        );
        // The focused window's title carries extra weight.
        let title_style = if focused {
            TextStyle::focused_title()
        } else {
            TextStyle::title_small()
        };
        let title = fit_text(
            fonts,
            title_style,
            &meta.title,
            (l.frame.w - design.window.title_x - design.window.title_right_padding).max(0.0),
        );
        fonts.draw_text_role(
            &mut s,
            design.shadow.padding + design.window.title_x,
            mid_y - title_style.line_height / 2.0,
            title_style,
            &title,
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
