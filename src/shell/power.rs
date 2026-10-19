//! Power.
use super::*;

impl Shell {
    pub fn render_power_menu(
        &mut self,
        theme: &Theme,
        fonts: &mut Fonts,
        win_w: f32,
        progress: f32,
        mask: Option<CircleMask>,
    ) -> OverlayFrame {
        let p = progress.clamp(0.0, 1.0);
        let card = power_menu_rect(win_w);
        let pad = crate::config::design().shadow.padding;
        let layer_w = (card.w + 2.0 * pad).ceil() as u32;
        let layer_h = (card.h + 2.0 * pad).ceil() as u32;
        let loc_x = card.x as f64 - pad as f64;
        let loc_y = card.y as f64 - pad as f64;
        let (scale, scale_tx, scale_ty) =
            self.power_placement(win_w, progress).buffer_transform(pad);

        // Only hover and press visuals animate; settled frames are cached.
        let stable = !self.press.keys().any(|k| matches!(k, PressKey::Power(_)));
        let key = KeyHash::new()
            .u32(layer_w)
            .u32(layer_h)
            .u64(theme.cache_key())
            .i32(self.hover_power.map_or(-1, |i| i as i32))
            .finish();
        let cached_ok = mask.is_none() && stable && self.power_layer.select(key);
        let fresh = if cached_ok {
            None
        } else {
            let mut layer = Surface::new(layer_w, layer_h);
            self.paint_power_menu(&mut layer, fonts, theme, &card, win_w);
            let buffer = layer.to_memory_buffer();
            if mask.is_none() && stable {
                self.power_layer.store(CachedLayer {
                    key,
                    surface: layer,
                    buffer,
                });
                None
            } else {
                Some(CachedLayer {
                    key,
                    surface: layer,
                    buffer,
                })
            }
        };
        let cached = fresh
            .as_ref()
            .or(self.power_layer.as_ref())
            .expect("just built");

        if let Some(mask) = mask {
            let mut s = Surface::new(layer_w, layer_h);
            let transform =
                SkTransform::from_scale(scale, scale).post_translate(scale_tx, scale_ty);
            s.blit(&cached.surface, transform, 1.0, None);
            let (buffer, loc) = masked_from(
                &s,
                Some(mask),
                &s.to_memory_buffer(),
                Point::from((loc_x, loc_y)),
            );
            return OverlayFrame::single(buffer, loc, None, p);
        }

        OverlayFrame::single(
            cached.buffer.clone(),
            Point::from((loc_x + scale_tx as f64, loc_y + scale_ty as f64)),
            Some(Size::from((
                (layer_w as f32 * scale).round() as i32,
                (layer_h as f32 * scale).round() as i32,
            ))),
            p,
        )
    }
}

impl Shell {
    pub(super) fn paint_power_menu(
        &self,
        layer: &mut Surface,
        fonts: &mut Fonts,
        theme: &Theme,
        card: &RectF,
        win_w: f32,
    ) {
        let design = crate::config::design();
        let pad = design.shadow.padding;
        layer.shadow_round_rect(
            pad,
            pad,
            card.w,
            card.h,
            design.shape.extra_large,
            design.shadow.spread,
            theme.shadow,
        );
        layer.fill_round_rect(
            pad,
            pad,
            card.w,
            card.h,
            design.shape.extra_large,
            theme.surface_container_low,
        );
        for (i, action) in POWER_ACTIONS.iter().enumerate() {
            let row = power_row_rect(win_w, i);
            let rx = row.x - card.x + pad;
            let ry = row.y - card.y + pad;
            let key = PressKey::Power(i);
            // The press spring drives the state layer, so a release fades the
            // highlight out over the same frames as the squeeze.
            let hold = ((1.0 - self.press_scale(key))
                / (1.0 - design.motion.press_scale).max(0.001))
            .clamp(0.0, 1.0);
            let hovered = self.hover_power == Some(i);
            if hovered || hold > 0.001 {
                layer.fill_round_rect(
                    rx,
                    ry,
                    row.w,
                    row.h,
                    design.shape.medium,
                    theme.state_layer(
                        theme.on_surface,
                        design.state.hover.max(design.state.pressed * hold),
                    ),
                );
            }
            let clip = round_rect_path(rx, ry, row.w, row.h, design.shape.medium);
            self.draw_ripple(layer, key, &clip, theme.on_surface);

            let cy = ry + row.h / 2.0;
            fonts.draw_icon_centered(
                layer,
                rx + design.power.icon_x,
                cy,
                action.icon(),
                design.power.icon_size,
                theme.on_surface,
            );
            fonts.draw_text_role(
                layer,
                rx + design.power.label_x,
                cy - TextStyle::label_large().line_height / 2.0,
                TextStyle::label_large(),
                action.label(),
                theme.on_surface,
            );
        }
    }
}
