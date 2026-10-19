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
        let pad = crate::config::design().shadow.padding;
        let layer_w = (rect.w + 2.0 * pad).ceil() as u32;
        let layer_h = (rect.h + 2.0 * pad).ceil() as u32;
        let loc_x = rect.x as f64 - pad as f64;
        let loc_y = rect.y as f64 - pad as f64;
        let (scale, scale_tx, scale_ty) = self
            .quick_settings_placement(win_w, win_h, progress)
            .buffer_transform(pad);

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
            .bool(self.brightness_available)
            .bool(self.brightness_error)
            .bool(self.night_light_available)
            .u32((brightness.clamp(0.0, 1.0) * 100.0).round() as u32);
        for s in &self.switches {
            key = key.f32(s.value());
        }
        let key = key.finish();

        let cached_ok = mask.is_none() && stable && self.qs_layer.select(key);
        let fresh = if cached_ok {
            None
        } else {
            let mut layer = Surface::new(layer_w, layer_h);
            self.paint_quick_settings(&mut layer, fonts, theme, rect, brightness, variant);
            let buffer = layer.to_memory_buffer();
            if mask.is_none() && stable {
                self.qs_layer.store(CachedLayer {
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
        let design = crate::config::design();
        let pad = design.shadow.padding;
        layer.shadow_round_rect(
            pad,
            pad,
            rect.w,
            rect.h,
            design.shape.extra_large,
            design.shadow.spread,
            theme.shadow,
        );
        layer.fill_round_rect(
            pad,
            pad,
            rect.w,
            rect.h,
            design.shape.extra_large,
            theme.surface_container_low,
        );
        fonts.draw_text_role(
            layer,
            pad + design.quick_settings.padding,
            pad + design.quick_settings.title_y,
            TextStyle::headline_small(),
            "Quick settings",
            theme.on_surface,
        );
        fonts.draw_text_role(
            layer,
            pad + design.quick_settings.padding,
            pad + design.quick_settings.subtitle_y,
            TextStyle::body_small(),
            "Appearance & display",
            theme.on_surface_variant,
        );

        let tiles = [
            ("dark_mode", "Dark theme", QuickAction::Theme),
            ("nightlight", "Night light", QuickAction::NightLight),
            ("reduce_motion", "Reduce motion", QuickAction::ReduceMotion),
            ("brightness_6", "Brightness", QuickAction::Brightness),
        ];
        for (i, (icon, label, action)) in tiles.iter().enumerate() {
            let tile = quick_setting_tile(rect, i, 1.0);
            // The painter and input share the final tile geometry.
            let x = tile.x - rect.x + pad;
            let y = tile.y - rect.y + pad;
            let key = PressKey::Tile(i);
            let squeeze = self.press_scale(key);
            let tw = tile.w * squeeze;
            let th = tile.h * squeeze;
            let tx = x + (tile.w - tw) / 2.0;
            let ty = y + (tile.h - th) / 2.0;
            let t = self.switch_spring(*action).clamp(0.0, 1.0);
            let idle = Theme::mix(
                theme.surface_container,
                if i == 1 {
                    theme.tertiary_container
                } else {
                    theme.secondary_container
                },
                design.quick_settings.tile_tint,
            );
            let fill = Theme::mix(idle, theme.primary_container, t);
            let on = Theme::mix(theme.on_surface, theme.on_primary_container, t);
            // A pressed tile softens from a rounded rectangle into a pill.
            let radius = design.quick_settings.tile_radius
                + design.quick_settings.tile_active_radius_growth * t
                + design.quick_settings.tile_press_radius_growth * (1.0 - squeeze);
            layer.fill_round_rect(tx, ty, tw, th, radius, fill);
            layer.circle(
                x + design.quick_settings.icon_x,
                y + design.quick_settings.icon_y,
                design.quick_settings.icon_radius,
                Theme::with_alpha(on, design.quick_settings.icon_alpha),
            );
            fonts.draw_icon_centered(
                layer,
                x + design.quick_settings.icon_x,
                y + design.quick_settings.icon_y,
                icon,
                design.quick_settings.icon_size * squeeze,
                on,
            );
            let label = fit_text(
                fonts,
                TextStyle::label_large(),
                label,
                tile.w - 2.0 * design.quick_settings.label_x,
            );
            fonts.draw_text_role(
                layer,
                x + design.quick_settings.label_x,
                y + design.quick_settings.label_y,
                TextStyle::label_large(),
                &label,
                on,
            );
            let status = if (*action == QuickAction::Brightness && !self.brightness_available)
                || (*action == QuickAction::NightLight && !self.night_light_available)
            {
                "Unavailable".into()
            } else if *action == QuickAction::Brightness && self.brightness_error {
                "Couldn’t change".into()
            } else if *action == QuickAction::Brightness {
                format!("{}%", (brightness.clamp(0.0, 1.0) * 100.0).round() as u32)
            } else if t > 0.5 {
                "On".into()
            } else {
                "Off".into()
            };
            fonts.draw_text_role(
                layer,
                x + design.quick_settings.label_x,
                y + design.quick_settings.status_y,
                TextStyle::label_medium(),
                &status,
                on,
            );
            if *action != QuickAction::Brightness {
                draw_m3_switch(
                    layer,
                    x + tile.w - design.switch.width - design.quick_settings.label_x,
                    y + design.quick_settings.switch_y,
                    self.switch_spring(*action),
                    theme,
                );
            }
            if self.press_tile == Some(*action) {
                layer.fill_round_rect(
                    tx,
                    ty,
                    tw,
                    th,
                    radius,
                    theme.state_layer(on, design.state.pressed),
                );
            }
            let clip = round_rect_path(tx, ty, tw, th, radius);
            self.draw_ripple(layer, key, &clip, on);
        }

        fonts.draw_text_role(
            layer,
            pad + design.quick_settings.padding,
            pad + design.quick_settings.chips_y - design.quick_settings.palette_title_gap,
            TextStyle::title_medium(),
            "Color palette",
            theme.on_surface,
        );
        let swatches = *self.swatches(theme.is_dark);
        for (i, variant_i) in VARIANTS.iter().enumerate() {
            let chip = palette_chip_rect_in(rect, i);
            if chip.bottom() > rect.bottom() - design.quick_settings.chip_bottom_gap {
                break;
            }
            let key = PressKey::Chip(i);
            let scale = self.press_scale(key);
            let cw = chip.w * scale;
            let ch = chip.h * scale;
            let x = chip.x - rect.x + pad + (chip.w - cw) / 2.0;
            let y = chip.y - rect.y + pad + (chip.h - ch) / 2.0;
            let selected = *variant_i == variant;
            let (fill, on) = if selected {
                (theme.secondary_container, theme.on_secondary_container)
            } else {
                (theme.surface_container_high, theme.on_surface_variant)
            };
            layer.fill_round_rect(x, y, cw, ch, ch / 2.0, fill);
            if selected {
                layer.stroke_round_rect(
                    x + 1.0,
                    y + 1.0,
                    cw - 2.0,
                    ch - 2.0,
                    ch / 2.0,
                    design.quick_settings.chip_outline_width,
                    Theme::with_alpha(theme.primary, design.quick_settings.chip_outline_alpha),
                );
            }
            let (a, b, c) = swatches[i];
            let cy = y + ch / 2.0;
            layer.circle(
                x + design.quick_settings.swatch_first_x,
                cy,
                design.quick_settings.swatch_radius,
                a,
            );
            layer.circle(
                x + design.quick_settings.swatch_first_x + design.quick_settings.swatch_gap,
                cy,
                design.quick_settings.swatch_radius,
                b,
            );
            layer.circle(
                x + design.quick_settings.swatch_first_x + 2.0 * design.quick_settings.swatch_gap,
                cy,
                design.quick_settings.swatch_radius,
                c,
            );
            let name = fit_text(
                fonts,
                TextStyle::label_small(),
                variant_i.name(),
                cw - design.quick_settings.chip_label_x - design.quick_settings.chip_label_padding,
            );
            fonts.draw_text_role(
                layer,
                x + design.quick_settings.chip_label_x,
                cy - TextStyle::label_small().line_height / 2.0,
                TextStyle::label_small(),
                &name,
                on,
            );
            let clip = round_rect_path(x, y, cw, ch, ch / 2.0);
            self.draw_ripple(layer, key, &clip, on);
        }
    }
}
