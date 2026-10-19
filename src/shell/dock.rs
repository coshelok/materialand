//! Dock.
use super::*;

impl Shell {
    #[allow(clippy::too_many_arguments)]
    pub fn render_dock(
        &mut self,
        theme: &Theme,
        fonts: &mut Fonts,
        win_w: f32,
        win_h: f32,
        items: &[DockItem],
        focused: Option<&Window>,
        mask: Option<CircleMask>,
    ) -> (MemoryRenderBuffer, Point<f64, Logical>) {
        let design = crate::config::design();
        let rect = self.dock_rect(win_w, win_h, items.len());
        let loc = Point::from((rect.x as f64, rect.y as f64));
        let mut key = KeyHash::new()
            .i32(win_w.round() as i32)
            .i32(win_h.round() as i32)
            .u64(theme.cache_key())
            .i32(self.hover_dock.map_or(-1, |i| i as i32));
        for item in items {
            match item {
                DockItem::Launcher => key = key.u32(1),
                DockItem::Launch(app) => key = key.u32(2).str(&app.icon),
                DockItem::Focus { window, key: k, .. } => {
                    key = key.u32(3).str(k).bool(focused == Some(window));
                }
            }
        }
        let cache_key = key.finish();
        let animating =
            self.dock_animating() || self.press.keys().any(|k| matches!(k, PressKey::Dock(_)));
        if !animating {
            if let Some(piece) = self.dock_cache.iter().find(|p| p.key == cache_key) {
                return masked_from(&piece.surface, mask, &piece.buffer, loc);
            }
        }
        let mut s = Surface::new(rect.w as u32, rect.h as u32);
        // A floating Android-style taskbar, with generous outer rounding.
        s.fill_round_rect(
            0.0,
            0.0,
            rect.w,
            rect.h,
            rect.h / 2.0,
            theme.surface_container,
        );

        for (i, item) in items.iter().enumerate() {
            let cy = rect.h / 2.0;
            let item_rect = self.dock_item_rect(win_w, win_h, items.len(), i);
            let icon_size = item_rect.w;
            if icon_size < 0.5 {
                continue;
            }
            let unit = icon_size / design.dock.icon_size;
            let cx = item_rect.x - rect.x + icon_size / 2.0;
            let key = PressKey::Dock(i);
            let scale = self.press_scale(key);

            match item {
                DockItem::Launcher => {
                    s.circle(cx, cy, icon_size / 2.0 * scale, theme.primary_container);
                    fonts.draw_icon_centered(
                        &mut s,
                        cx,
                        cy,
                        "apps",
                        design.dock.glyph_size * scale * unit,
                        theme.on_primary_container,
                    );
                }
                DockItem::Launch(app) => {
                    let icon = config::icon_for_key(&app.icon);
                    let (fill, on) = app_colors(theme, &app.icon);
                    s.circle(cx, cy, icon_size / 2.0 * scale, fill);
                    fonts.draw_icon_centered(
                        &mut s,
                        cx,
                        cy,
                        icon,
                        design.dock.glyph_size * scale * unit,
                        on,
                    );
                }
                DockItem::Focus { window, key: k, .. } => {
                    let icon = config::icon_for_key(k);
                    let active = focused == Some(window);
                    let (fill, on) = app_colors(theme, k);
                    s.circle(cx, cy, icon_size / 2.0 * scale, fill);
                    if active {
                        let w = icon_size * scale;
                        let h = icon_size * scale;
                        s.fill_round_rect(
                            cx - w / 2.0,
                            cy - h / 2.0,
                            w,
                            h,
                            design.shape.large,
                            theme.primary_container,
                        );
                    }
                    fonts.draw_icon_centered(
                        &mut s,
                        cx,
                        cy,
                        icon,
                        design.dock.glyph_size * scale * unit,
                        if active {
                            theme.on_primary_container
                        } else {
                            on
                        },
                    );
                    // Indicator dot for running apps.
                    s.fill_round_rect(
                        cx - if active {
                            design.dock.indicator_active_width / 2.0
                        } else {
                            design.dock.indicator_width / 2.0
                        },
                        rect.h - design.dock.indicator_bottom,
                        if active {
                            design.dock.indicator_active_width
                        } else {
                            design.dock.indicator_width
                        },
                        design.dock.indicator_height,
                        design.dock.indicator_radius,
                        theme.primary,
                    );
                }
            }

            let hovered = self.hover_dock == Some(i);
            let pressed = self.press_dock == Some(i);
            if hovered || pressed {
                let opacity = if pressed {
                    design.state.pressed
                } else {
                    design.state.hover
                };
                s.circle(
                    cx,
                    cy,
                    icon_size / 2.0 + design.dock.feedback_padding,
                    theme.state_layer(theme.on_surface, opacity),
                );
            }
            let mut cb = PathBuilder::new();
            cb.push_circle(cx, cy, icon_size / 2.0 + design.dock.feedback_padding);
            if let Some(clip) = cb.finish() {
                self.draw_ripple(&mut s, key, &clip, theme.on_surface);
            }
        }

        let buffer = s.to_memory_buffer();
        let result = masked_from(&s, mask, &buffer, loc);
        if !animating {
            self.dock_cache.insert(
                0,
                Piece {
                    key: cache_key,
                    surface: s,
                    buffer,
                },
            );
            self.dock_cache.truncate(4);
        }
        result
    }
}
