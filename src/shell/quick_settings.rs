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
        if tablet_mode() && config::design().shade.fullscreen && mask.is_none() {
            return self
                .render_tablet_shade(theme, fonts, win_w, win_h, progress, brightness, variant);
        }
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
            .str(&self.clock)
            .str(&self.calendar.now.date_text())
            .str(self.appearance_error.as_deref().unwrap_or(""))
            .str(&format!("{:?}", self.system_status))
            .u32(self.shade_scroll.to_bits())
            .u64(self.notifications.revision)
            .bool(self.notifications_open)
            .bool(self.appearance_page)
            .u32(
                self.connection_page
                    .map_or(0, |page| if page == QuickAction::Wifi { 1 } else { 2 }),
            )
            .str(variant.name())
            .bool(self.brightness_available)
            .bool(self.brightness_error)
            .bool(self.night_light_available)
            .f32(brightness.clamp(0.0, 1.0));
        for s in &self.switches {
            key = key.f32(s.value());
        }
        let key = key.finish();

        let cached_ok = mask.is_none() && stable && self.qs_layer.select(key);
        let fresh = if cached_ok {
            None
        } else {
            let mut layer = Surface::new(layer_w, layer_h);
            self.paint_quick_settings(
                &mut layer,
                fonts,
                theme,
                rect,
                brightness,
                variant,
                shade_split(win_w),
            );
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

        let mut frame = OverlayFrame::single(
            cached.buffer.clone(),
            Point::from((loc_x + scale_tx as f64, loc_y + scale_ty as f64)),
            Some(Size::from((
                (layer_w as f32 * scale).round() as i32,
                (layer_h as f32 * scale).round() as i32,
            ))),
            p,
        );
        if tablet_mode() && config::design().shade.fullscreen {
            let key = theme.cache_key();
            if !self.shade_backdrop.select(key) {
                let mut surface = Surface::new(1, 1);
                surface.clear(theme.surface_container_low);
                let buffer = surface.to_memory_buffer();
                self.shade_backdrop.store(CachedLayer {
                    key,
                    surface,
                    buffer,
                });
            }
            frame.pieces.insert(
                0,
                (
                    LayerFrame {
                        rounded: None,
                        source: None,
                        buffer: self.shade_backdrop.as_ref().unwrap().buffer.clone(),
                        loc: (
                            0.0,
                            f64::from(
                                self.quick_settings_placement(win_w, win_h, progress)
                                    .target
                                    .y,
                            ),
                        )
                            .into(),
                        size: Some((win_w.ceil() as i32, rect.h.ceil() as i32).into()),
                    },
                    p,
                ),
            );
        }
        frame
    }
    #[allow(clippy::too_many_arguments)]
    pub(super) fn paint_quick_settings(
        &mut self,
        layer: &mut Surface,
        fonts: &mut Fonts,
        theme: &Theme,
        rect: RectF,
        brightness: f32,
        variant: ColorVariant,
        dual_column: bool,
    ) {
        let design = crate::config::design();
        let pad = design.shadow.padding;
        if !(tablet_mode() && design.shade.fullscreen) {
            layer.shadow_round_rect(
                pad,
                pad,
                rect.w,
                rect.h,
                design.shape.extra_large,
                design.shadow.spread,
                theme.shadow,
            );
        }
        layer.fill_round_rect(
            pad,
            pad,
            rect.w,
            rect.h,
            if tablet_mode() && design.shade.fullscreen {
                0.0
            } else {
                design.shape.extra_large
            },
            theme.surface_container_low,
        );
        let clock_style = if tablet_mode()
            && !self.notifications_open
            && self.connection_page.is_none()
            && !self.appearance_page
        {
            TextStyle {
                size: design.shade.clock_size,
                line_height: design.shade.clock_line_height,
                ..TextStyle::headline_small()
            }
        } else {
            TextStyle::headline_small()
        };
        let heading = fit_text(
            fonts,
            clock_style,
            if self.appearance_page {
                "Style"
            } else if self.notifications_open {
                "Notifications"
            } else if tablet_mode() && self.connection_page.is_none() {
                &self.clock
            } else if self.connection_page == Some(QuickAction::Wifi) {
                "Wi-Fi"
            } else if self.connection_page == Some(QuickAction::Bluetooth) {
                "Bluetooth"
            } else {
                "Quick settings"
            },
            rect.w - 2.0 * design.quick_settings.padding - 3.0 * design.shade.header_button_width,
        );
        fonts.draw_text_role(
            layer,
            pad + design.quick_settings.padding,
            pad + design.quick_settings.title_y,
            clock_style,
            &heading,
            theme.on_surface,
        );
        let date = self.calendar.now.date_text();
        let subtitle_y = self.appearance_status_rect(rect, dual_column).y - rect.y;
        let subtitle = self
            .appearance_error
            .as_deref()
            .unwrap_or(if self.appearance_page {
                "Wallpaper & style"
            } else if self.notifications_open {
                "App updates"
            } else if tablet_mode() && self.connection_page.is_none() {
                &date
            } else {
                "Connections, appearance & display"
            });
        let subtitle = fit_text(
            fonts,
            TextStyle::body_small(),
            subtitle,
            rect.w
                - 2.0 * design.quick_settings.padding
                - if self.appearance_error.is_some() {
                    64.0
                } else {
                    0.0
                },
        );
        fonts.draw_text_role(
            layer,
            pad + design.quick_settings.padding,
            pad + subtitle_y,
            TextStyle::body_small(),
            &subtitle,
            if self.appearance_error.is_some() {
                theme.error
            } else {
                theme.on_surface_variant
            },
        );
        if self.appearance_error.is_some() {
            fonts.draw_text_role(
                layer,
                pad + rect.w - design.quick_settings.padding - 48.0,
                pad + subtitle_y,
                TextStyle::body_small(),
                "Retry",
                theme.primary,
            );
        }

        if self.notifications_open || self.connection_page.is_some() || self.appearance_page {
            fonts.draw_icon_centered(
                layer,
                pad + rect.w - 2.5 * design.shade.header_button_width,
                pad + config::design().statusbar.height,
                "arrow_back",
                config::design().statusbar.icon_size,
                theme.on_surface,
            );
        }
        fonts.draw_icon_centered(
            layer,
            pad + rect.w - 0.5 * design.shade.header_button_width,
            pad + 32.0,
            "power_settings_new",
            22.0,
            theme.on_surface,
        );
        fonts.draw_icon_centered(
            layer,
            pad + rect.w - 1.5 * design.shade.header_button_width,
            pad + 32.0,
            "schedule",
            22.0,
            theme.on_surface,
        );

        let mut body = Surface::new(layer.pixmap.width(), layer.pixmap.height());
        if self.notifications_open {
            self.paint_notifications(layer, fonts, theme, rect, pad);
            return;
        }
        if self.connection_page.is_some() {
            self.paint_connections(layer, fonts, theme, rect, pad);
            return;
        }
        if tablet_mode() && !self.appearance_page {
            self.paint_shade_brightness(layer, fonts, theme, rect, pad, brightness);
        }
        if !self.appearance_page {
            let tiles = [
                ("dark_mode", "Dark theme", QuickAction::Theme),
                ("nightlight", "Night light", QuickAction::NightLight),
                ("reduce_motion", "Reduce motion", QuickAction::ReduceMotion),
                ("brightness_6", "Brightness", QuickAction::Brightness),
                ("wifi", "Wi-Fi", QuickAction::Wifi),
                ("bluetooth", "Bluetooth", QuickAction::Bluetooth),
            ];
            for (i, (icon, label, action)) in tiles.iter().enumerate() {
                let tile = quick_setting_tile(rect, i, 1.0);
                // The painter and input share the final tile geometry.
                let x = tile.x - rect.x + pad;
                let y = tile.y - rect.y + pad - self.shade_scroll;
                if tablet_mode() {
                    self.paint_android_tile(
                        &mut body,
                        fonts,
                        theme,
                        RectF {
                            x,
                            y,
                            w: tile.w,
                            h: tile.h,
                        },
                        *action,
                        label,
                        icon,
                        i,
                        brightness,
                    );
                    continue;
                }
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
                body.fill_round_rect(tx, ty, tw, th, radius, fill);
                body.circle(
                    x + design.quick_settings.icon_x,
                    y + design.quick_settings.icon_y,
                    design.quick_settings.icon_radius,
                    Theme::with_alpha(on, design.quick_settings.icon_alpha),
                );
                fonts.draw_icon_centered(
                    &mut body,
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
                    &mut body,
                    x + design.quick_settings.label_x,
                    y + design.quick_settings.label_y,
                    TextStyle::label_large(),
                    &label,
                    on,
                );
                let radio = match action {
                    QuickAction::Wifi => Some(&self.system_status.wifi),
                    QuickAction::Bluetooth => Some(&self.system_status.bluetooth),
                    _ => None,
                };
                let status: String = if let Some(radio) = radio {
                    if radio.busy {
                        "Working…".into()
                    } else if !radio.available {
                        "Unavailable".into()
                    } else {
                        radio.label.clone()
                    }
                } else if (*action == QuickAction::Brightness && !self.brightness_available)
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
                let status = fit_text(
                    fonts,
                    TextStyle::label_medium(),
                    &status,
                    tile.w - 2.0 * design.quick_settings.label_x,
                );
                fonts.draw_text_role(
                    &mut body,
                    x + design.quick_settings.label_x,
                    y + design.quick_settings.status_y,
                    TextStyle::label_medium(),
                    &status,
                    on,
                );
                if *action != QuickAction::Brightness {
                    draw_m3_switch(
                        &mut body,
                        x + tile.w - design.switch.width - design.quick_settings.label_x,
                        y + design.quick_settings.switch_y,
                        self.switch_spring(*action),
                        theme,
                    );
                }
                if self.press_tile == Some(*action) {
                    body.fill_round_rect(
                        tx,
                        ty,
                        tw,
                        th,
                        radius,
                        theme.state_layer(on, design.state.pressed),
                    );
                }
                if matches!(action, QuickAction::Wifi | QuickAction::Bluetooth) {
                    fonts.draw_icon_centered(
                        &mut body,
                        x + tile.w - 24.0,
                        y + design.quick_settings.status_y + 8.0,
                        "chevron_right",
                        20.0,
                        on,
                    );
                }
                let clip = round_rect_path(tx, ty, tw, th, radius);
                self.draw_ripple(&mut body, key, &clip, on);
            }

            let button = shade_style_button(rect);
            let x = button.x - rect.x + pad;
            let y = button.y - rect.y + pad - self.shade_scroll;
            body.fill_round_rect(
                x,
                y,
                button.w,
                button.h,
                button.h / 2.0,
                theme.surface_container_high,
            );
            fonts.draw_icon_centered(
                &mut body,
                x + 28.0,
                y + button.h / 2.0,
                "palette",
                22.0,
                theme.on_surface_variant,
            );
            let label = fit_text(
                fonts,
                TextStyle::title_small(),
                "Wallpaper & style",
                button.w - 96.0,
            );
            fonts.draw_text_role(
                &mut body,
                x + 56.0,
                y + (button.h - TextStyle::title_small().line_height) / 2.0,
                TextStyle::title_small(),
                &label,
                theme.on_surface,
            );
            fonts.draw_icon_centered(
                &mut body,
                x + button.w - 24.0,
                y + button.h / 2.0,
                "chevron_right",
                22.0,
                theme.on_surface_variant,
            );
        } else {
            if tablet_mode() {
                let description = fit_text(
                    fonts,
                    TextStyle::body_large(),
                    "Colors from your wallpaper",
                    rect.w - 2.0 * design.quick_settings.padding,
                );
                fonts.draw_text_role(
                    layer,
                    pad + design.quick_settings.padding,
                    pad + design.shade.brightness_y,
                    TextStyle::body_large(),
                    &description,
                    theme.on_surface_variant,
                );
            }
            fonts.draw_text_role(
                &mut body,
                pad + design.quick_settings.padding,
                pad + palette_top(rect.w)
                    - design.quick_settings.palette_title_gap
                    - self.shade_scroll,
                TextStyle::title_medium(),
                "Color palette",
                theme.on_surface,
            );
            let swatches = *self.swatches(theme.is_dark);
            for (i, variant_i) in VARIANTS.iter().enumerate() {
                let chip = palette_chip_rect_in(rect, i);

                let key = PressKey::Chip(i);
                let scale = self.press_scale(key);
                let cw = chip.w * scale;
                let ch = chip.h * scale;
                let x = chip.x - rect.x + pad + (chip.w - cw) / 2.0;
                let y = chip.y - rect.y + pad + (chip.h - ch) / 2.0 - self.shade_scroll;
                let selected = *variant_i == variant;
                let (fill, on) = if selected {
                    (theme.secondary_container, theme.on_secondary_container)
                } else {
                    (theme.surface_container_high, theme.on_surface_variant)
                };
                body.fill_round_rect(x, y, cw, ch, ch / 2.0, fill);
                if selected {
                    body.stroke_round_rect(
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
                body.circle(
                    x + design.quick_settings.swatch_first_x,
                    cy,
                    design.quick_settings.swatch_radius,
                    a,
                );
                body.circle(
                    x + design.quick_settings.swatch_first_x + design.quick_settings.swatch_gap,
                    cy,
                    design.quick_settings.swatch_radius,
                    b,
                );
                body.circle(
                    x + design.quick_settings.swatch_first_x
                        + 2.0 * design.quick_settings.swatch_gap,
                    cy,
                    design.quick_settings.swatch_radius,
                    c,
                );
                let name = fit_text(
                    fonts,
                    TextStyle::label_small(),
                    variant_i.name(),
                    cw - design.quick_settings.chip_label_x
                        - design.quick_settings.chip_label_padding,
                );
                fonts.draw_text_role(
                    &mut body,
                    x + design.quick_settings.chip_label_x,
                    cy - TextStyle::label_small().line_height / 2.0,
                    TextStyle::label_small(),
                    &name,
                    on,
                );
                let clip = round_rect_path(x, y, cw, ch, ch / 2.0);
                self.draw_ripple(&mut body, key, &clip, on);
            }
        }
        let viewport = round_rect_path(
            pad,
            pad + quick_tiles_top(),
            rect.w,
            (rect.h - quick_tiles_top()).max(1.0),
            0.0,
        );
        let rounded = round_rect_path(pad, pad, rect.w, rect.h, config::design().shape.extra_large);
        layer.blit_intersection(&body, &[&viewport, &rounded]);
        let battery = self
            .system_status
            .battery
            .as_ref()
            .map(|b| {
                format!(
                    "{}% · {}",
                    b.percent,
                    if b.charging {
                        "Charging"
                    } else if b.plugged {
                        "Plugged in"
                    } else {
                        "On battery"
                    }
                )
            })
            .unwrap_or_else(|| "Battery unavailable".into());
        if tablet_mode() && !self.appearance_page {
            fonts.draw_text_role(
                layer,
                pad + design.quick_settings.padding,
                pad + design.shade.brightness_y - TextStyle::label_medium().line_height - 4.0,
                TextStyle::label_medium(),
                &battery,
                theme.on_surface_variant,
            );
        }
        if let Some(error) = &self.system_status.error {
            let text = fit_text(
                fonts,
                TextStyle::body_small(),
                error,
                rect.w - 2.0 * design.quick_settings.padding,
            );
            fonts.draw_text_role(
                layer,
                pad + design.quick_settings.padding,
                pad + rect.h - 22.0,
                TextStyle::body_small(),
                &text,
                theme.error,
            );
        }
    }
}
