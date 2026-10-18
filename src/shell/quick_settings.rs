//! Quick-settings layer.
use super::*;
impl Shell {
    #[allow(clippy::too_many_arguments)]
    pub fn render_quick_settings(
        &mut self,
        theme: &Theme,
        fonts: &mut Fonts,
        win_w: f32,
        win_h: f32,
        progress: f32,
        brightness: f32,
        variant: ColorVariant,
        mask: Option<CircleMask>,
    ) -> OverlayFrame {
        let p = progress.clamp(0.0, 1.0);
        let rect = quick_settings_rect(win_w, win_h);
        let ease = CubicBezier::EMPHASIZED_DECELERATE.eval(p);
        let origin = self
            .qs_origin
            .unwrap_or_else(|| topbar_button_rect(win_w, TopAction::QuickSettings));
        let dy = (origin.y + origin.h / 2.0 - (rect.y + rect.h / 2.0)) * (1.0 - ease);
        let scale = 0.92 + 0.08 * ease;
        let pad = SHADOW_PAD;
        let layer_w = (rect.w + 2.0 * pad).ceil() as u32;
        let layer_h = (rect.h + 2.0 * pad).ceil() as u32;
        let loc_x = rect.x as f64 - pad as f64;
        let loc_y = rect.y as f64 - pad as f64;
        let cx = pad + rect.w / 2.0;
        let cy = pad + rect.h / 2.0;
        let scale_tx = cx * (1.0 - scale);
        let scale_ty = cy * (1.0 - scale) + dy;

        // The sheet only changes while a switch, tile or chip is in motion;
        // such frames are rebuilt directly, settled ones reuse the cache.
        let stable = self.switches.iter().all(|s| s.settled())
            && !self
                .press
                .keys()
                .any(|k| matches!(k, PressKey::Tile(_) | PressKey::Chip(_)));
        let mut key = KeyHash::new()
            .u32(layer_w)
            .u32(layer_h)
            .u64(theme.cache_key())
            .str(variant.name())
            .u32((brightness.clamp(0.0, 1.0) * 100.0).round() as u32);
        for s in &self.switches {
            key = key.f32(s.value());
        }
        let key = key.finish();

        let cached_ok =
            mask.is_none() && stable && self.qs_layer.as_ref().is_some_and(|l| l.key == key);
        let fresh = if cached_ok {
            None
        } else {
            let mut layer = Surface::new(layer_w, layer_h);
            self.paint_quick_settings(&mut layer, fonts, theme, rect, brightness, variant);
            let buffer = layer.to_memory_buffer();
            if mask.is_none() && stable {
                self.qs_layer = Some(CachedLayer {
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
            .or(self.qs_layer.as_ref())
            .expect("just built");

        if let Some(mask) = mask {
            // Radial reveal: CPU composite + circle clip.
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
    pub(super) fn paint_quick_settings(
        &mut self,
        layer: &mut Surface,
        fonts: &mut Fonts,
        theme: &Theme,
        rect: RectF,
        brightness: f32,
        variant: ColorVariant,
    ) {
        let pad = SHADOW_PAD;
        layer.shadow_round_rect(pad, pad, rect.w, rect.h, shape::EXTRA_LARGE, SHADOW_SPREAD);
        layer.fill_round_rect(
            pad,
            pad,
            rect.w,
            rect.h,
            shape::EXTRA_LARGE,
            theme.surface_container_high,
        );
        layer.stroke_round_rect(
            pad + 0.5,
            pad + 0.5,
            rect.w - 1.0,
            rect.h - 1.0,
            shape::EXTRA_LARGE,
            1.0,
            Theme::with_alpha(theme.outline, 0.2),
        );

        let title_style = TextStyle::TITLE_MEDIUM;
        fonts.draw_text_role(
            layer,
            pad + 16.0,
            pad + 14.0,
            title_style,
            "Quick settings",
            theme.on_surface,
        );

        let tile_w = ((rect.w - 44.0) / 2.0).max(56.0);
        let tile_h = 72.0;
        let x1 = pad + 16.0;
        let x2 = x1 + tile_w + 12.0;
        let y1 = pad + 48.0;
        let y2 = y1 + tile_h + 12.0;
        let tiles = [
            (x1, y1, "dark_mode", "Dark theme", QuickAction::Theme),
            (x2, y1, "nightlight", "Night light", QuickAction::NightLight),
            (
                x1,
                y2,
                "reduce_motion",
                "Reduce motion",
                QuickAction::ReduceMotion,
            ),
            (
                x2,
                y2,
                "brightness_6",
                "Brightness",
                QuickAction::Brightness,
            ),
        ];

        for (i, (x, y, icon, label, action)) in tiles.iter().enumerate() {
            let action = *action;
            let key = PressKey::Tile(i);
            let scale_p = self.press_scale(key);
            let tw = tile_w * scale_p;
            let th = tile_h * scale_p;
            let tx = x + (tile_w - tw) / 2.0;
            let ty = y + (tile_h - th) / 2.0;

            let t = self.switch_spring(action).clamp(0.0, 1.0);
            let active = t > 0.5;
            let fill = Theme::mix(theme.surface_container_low, theme.secondary_container, t);
            layer.fill_round_rect(tx, ty, tw, th, shape::LARGE, fill);
            layer.stroke_round_rect(
                tx + 0.5,
                ty + 0.5,
                tw - 1.0,
                th - 1.0,
                shape::LARGE,
                1.0,
                Theme::with_alpha(
                    if active {
                        theme.primary
                    } else {
                        theme.outline_variant
                    },
                    0.35,
                ),
            );
            let on_color = Theme::mix(theme.on_surface_variant, theme.on_secondary_container, t);
            fonts.draw_icon_centered(layer, x + 18.0, y + 22.0, icon, 18.0, on_color);
            fonts.draw_text_role(
                layer,
                x + 34.0,
                y + 13.0,
                TextStyle::BODY_SMALL,
                label,
                on_color,
            );

            if action == QuickAction::Brightness {
                let value = format!("{}%", (brightness.clamp(0.0, 1.0) * 100.0).round() as u32);
                fonts.draw_text_role(
                    layer,
                    x + 34.0,
                    y + 34.0,
                    TextStyle::LABEL_MEDIUM,
                    &value,
                    theme.on_surface_variant,
                );
            } else {
                draw_m3_switch(
                    layer,
                    x + tile_w - 60.0,
                    y + 38.0,
                    self.switch_spring(action),
                    theme,
                );
            }

            if self.press_tile == Some(action) {
                layer.fill_round_rect(
                    tx,
                    ty,
                    tw,
                    th,
                    shape::LARGE,
                    theme.state_layer(theme.on_surface, state::PRESSED),
                );
            }
            let clip = round_rect_path(tx, ty, tw, th, shape::LARGE);
            self.draw_ripple(layer, key, &clip, theme.on_surface);
        }

        // ---- Palette picker --------------------------------------------
        fonts.draw_text_role(
            layer,
            pad + 16.0,
            pad + QS_CHIPS_Y - 24.0,
            TextStyle::TITLE_SMALL,
            "Palette",
            theme.on_surface,
        );
        let current = variant.name();
        let current_w = fonts.text_width_styled(TextStyle::LABEL_MEDIUM, current);
        fonts.draw_text_role(
            layer,
            pad + rect.w - 16.0 - current_w,
            pad + QS_CHIPS_Y - 22.0,
            TextStyle::LABEL_MEDIUM,
            current,
            theme.on_surface_variant,
        );

        let swatches = *self.swatches(theme.is_dark);
        for (i, &variant_i) in VARIANTS.iter().enumerate() {
            let chip = palette_chip_rect_in(rect, i);
            if chip.bottom() > rect.bottom() - 6.0 {
                break;
            }
            let key = PressKey::Chip(i);
            let scale_p = self.press_scale(key);
            let cw = chip.w * scale_p;
            let ch = chip.h * scale_p;
            let cx = chip.x - rect.x + pad;
            let cy = chip.y - rect.y + pad;
            let x = cx + (chip.w - cw) / 2.0;
            let y = cy + (chip.h - ch) / 2.0;

            let selected = variant_i == variant;
            let fill = if selected {
                theme.secondary_container
            } else {
                theme.surface_container_low
            };
            let outline = if selected {
                Theme::with_alpha(theme.primary, 0.5)
            } else {
                Theme::with_alpha(theme.outline_variant, 0.5)
            };
            layer.fill_round_rect(x, y, cw, ch, ch / 2.0, fill);
            layer.stroke_round_rect(x + 0.5, y + 0.5, cw - 1.0, ch - 1.0, ch / 2.0, 1.0, outline);

            let (p1, p2, p3) = swatches[i];
            let dot_y = y + ch / 2.0;
            layer.circle(x + 14.0, dot_y, 4.0, p1);
            layer.circle(x + 25.0, dot_y, 4.0, p2);
            layer.circle(x + 36.0, dot_y, 4.0, p3);

            let name = truncate(variant_i.name(), 10);
            fonts.draw_text_role(
                layer,
                x + 48.0,
                dot_y - TextStyle::LABEL_SMALL.line_height / 2.0,
                TextStyle::LABEL_SMALL,
                &name,
                if selected {
                    theme.on_secondary_container
                } else {
                    theme.on_surface_variant
                },
            );

            if self.press_chip == Some(i) {
                layer.fill_round_rect(
                    x,
                    y,
                    cw,
                    ch,
                    ch / 2.0,
                    theme.state_layer(theme.on_surface, state::PRESSED),
                );
            }
            let clip = round_rect_path(x, y, cw, ch, ch / 2.0);
            self.draw_ripple(layer, key, &clip, theme.on_surface);

            // Keep the borrow checker happy: `swatches` is a copy.
        }
    }
}
