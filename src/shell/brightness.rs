//! Brightness.
use super::*;

impl Shell {
    pub fn render_brightness(
        &mut self,
        theme: &Theme,
        fonts: &mut Fonts,
        win_w: f32,
        brightness: f32,
        progress: f32,
        mask: Option<CircleMask>,
    ) -> OverlayFrame {
        let p = progress.clamp(0.0, 1.0);
        let card = brightness_popup(win_w);
        let pad = SHADOW_PAD;
        let layer_w = (card.w + 2.0 * pad).ceil() as u32;
        let layer_h = (card.h + 2.0 * pad).ceil() as u32;
        let loc_x = card.x as f64 - pad as f64;
        let loc_y = card.y as f64 - pad as f64;
        let (scale, scale_tx, scale_ty) = self
            .brightness_placement(win_w, progress)
            .buffer_transform(pad);

        // While the slider is animating its handle the content changes; such
        // frames are rebuilt, settled ones reuse the cached layer.
        let stable = self.slider.settled();
        let key = self.brightness_layer_key(layer_w, layer_h, theme, brightness);
        let cached_ok = mask.is_none()
            && stable
            && self.brightness_layer.as_ref().is_some_and(|l| l.key == key);
        let fresh = if cached_ok {
            None
        } else {
            let mut layer = Surface::new(layer_w, layer_h);
            self.paint_brightness(&mut layer, fonts, theme, &card, win_w, brightness);
            let buffer = layer.to_memory_buffer();
            if mask.is_none() && stable {
                self.brightness_layer = Some(CachedLayer {
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
            .or(self.brightness_layer.as_ref())
            .expect("just built");

        if let Some(mask) = mask {
            // Radial reveal: the circle clip is CPU work, so the scaled layer
            // is composited and masked here.
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
    pub(super) fn brightness_layer_key(
        &self,
        layer_w: u32,
        layer_h: u32,
        theme: &Theme,
        brightness: f32,
    ) -> u64 {
        KeyHash::new()
            .bool(self.brightness_available)
            .bool(self.brightness_error)
            .u32(layer_w)
            .u32(layer_h)
            .u64(theme.cache_key())
            .u32((brightness.clamp(0.0, 1.0) * 1000.0).round() as u32)
            .finish()
    }
}

impl Shell {
    pub(super) fn paint_brightness(
        &self,
        layer: &mut Surface,
        fonts: &mut Fonts,
        theme: &Theme,
        card: &RectF,
        win_w: f32,
        brightness: f32,
    ) {
        let pad = SHADOW_PAD;
        layer.shadow_round_rect(pad, pad, card.w, card.h, shape::EXTRA_LARGE, SHADOW_SPREAD);
        layer.fill_round_rect(
            0.0 + pad,
            pad,
            card.w,
            card.h,
            shape::EXTRA_LARGE,
            theme.surface_container_low,
        );
        let cy = pad + card.h / 2.0 + 12.0;
        fonts.draw_icon_centered(
            layer,
            pad + 28.0,
            cy,
            "brightness_6",
            22.0,
            theme.on_surface,
        );

        let label = if !self.brightness_available {
            "Brightness unavailable".to_string()
        } else if self.brightness_error {
            "Couldn’t change brightness".to_string()
        } else {
            format!("Brightness · {}%", (brightness * 100.0).round() as u32)
        };
        fonts.draw_text_role(
            layer,
            pad + 56.0,
            pad + 8.0,
            TextStyle::LABEL_MEDIUM,
            &label,
            theme.on_surface_variant,
        );
        if !self.brightness_available {
            return;
        }
        let track = brightness_track(win_w);
        let tx = track.x - card.x + pad;
        let t = ((brightness - BRIGHTNESS_MIN) / (BRIGHTNESS_MAX - BRIGHTNESS_MIN)).clamp(0.0, 1.0);
        layer.fill_round_rect(
            tx,
            cy - 16.0,
            track.w,
            32.0,
            16.0,
            theme.secondary_container,
        );
        layer.fill_round_rect(
            tx,
            cy - 16.0,
            (track.w * t).max(12.0),
            32.0,
            16.0,
            theme.primary_container,
        );
        // M3 slider: the handle grows while grabbed.
        let handle_h = 38.0 + 6.0 * self.slider.value().clamp(0.0, 1.06);
        layer.fill_round_rect(
            tx + track.w * t - 3.0,
            cy - handle_h / 2.0,
            6.0,
            handle_h,
            3.0,
            theme.primary,
        );
    }
}
