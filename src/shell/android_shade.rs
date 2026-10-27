//! Android-style shade components; painting and pointer routing share geometry.
use super::*;

pub fn shade_brightness_rect(w: f32, h: f32) -> RectF {
    let r = quick_settings_rect(w, h);
    let d = config::design();
    RectF {
        x: r.x + d.quick_settings.padding,
        y: r.y + d.shade.brightness_y,
        w: (r.w - 2.0 * d.quick_settings.padding).max(1.0),
        h: d.shade.brightness_height,
    }
}
pub fn shade_brightness_hit(w: f32, h: f32, pos: Point<f64, Logical>) -> Option<f32> {
    let r = shade_brightness_rect(w, h);
    r.contains(pos)
        .then(|| shade_brightness_value(w, h, pos.x as f32))
}
pub fn shade_brightness_value(w: f32, h: f32, x: f32) -> f32 {
    let r = shade_brightness_rect(w, h);
    let edge = (2.0 * config::design().shade.brightness_icon_x).min(r.w / 3.0);
    let t = ((x - r.x - edge) / (r.w - edge - r.h / 2.0).max(1.0)).clamp(0.0, 1.0);
    BRIGHTNESS_MIN + t * (BRIGHTNESS_MAX - BRIGHTNESS_MIN)
}
impl Shell {
    /// Fixed header row: saving failures remain visible while the body scrolls.
    pub fn appearance_status_rect(&self, rect: RectF, split: bool) -> RectF {
        let d = config::design();
        let line_height = if tablet_mode()
            && (!self.notifications_open || split)
            && self.connection_page.is_none()
            && !self.appearance_page
        {
            d.shade.clock_line_height
        } else {
            TextStyle::headline_small().line_height
        };
        RectF {
            x: rect.x + d.quick_settings.padding,
            y: rect.y
                + d.quick_settings
                    .subtitle_y
                    .max(d.quick_settings.title_y + line_height + 4.0),
            w: (rect.w - 2.0 * d.quick_settings.padding).max(1.0),
            h: TextStyle::body_small().line_height,
        }
    }
    #[allow(clippy::too_many_arguments)]
    pub(super) fn paint_shade_brightness(
        &self,
        s: &mut Surface,
        fonts: &mut Fonts,
        theme: &Theme,
        rect: RectF,
        pad: f32,
        brightness: f32,
    ) {
        let d = config::design();
        let x = pad + d.quick_settings.padding;
        let y = pad + d.shade.brightness_y;
        let w = (rect.w - 2.0 * d.quick_settings.padding).max(1.0);
        let h = d.shade.brightness_height;
        s.fill_round_rect(x, y, w, h, h / 2.0, theme.surface_container_highest);
        let fill = if self.brightness_available {
            theme.primary
        } else {
            theme.surface_container_high
        };
        let t = ((brightness - BRIGHTNESS_MIN) / (BRIGHTNESS_MAX - BRIGHTNESS_MIN)).clamp(0.0, 1.0);
        let edge = (2.0 * d.shade.brightness_icon_x).min(w / 3.0);
        let filled = (edge + t * (w - edge - h / 2.0)).clamp(h.min(w), w);
        s.fill_round_rect(x, y, filled, h, h / 2.0, fill);
        fonts.draw_icon_centered(
            s,
            x + d.shade.brightness_icon_x,
            y + h / 2.0,
            "brightness_6",
            d.statusbar.icon_size,
            if self.brightness_available {
                theme.on_primary
            } else {
                theme.on_surface_variant
            },
        );
        if !self.brightness_available {
            let label = fit_text(
                fonts,
                TextStyle::label_large(),
                "Brightness unavailable",
                w - edge - h / 2.0,
            );
            fonts.draw_text_role(
                s,
                x + edge,
                y + (h - TextStyle::label_large().line_height) / 2.0,
                TextStyle::label_large(),
                &label,
                theme.on_surface_variant,
            );
        }
    }
    #[allow(clippy::too_many_arguments)]
    pub(super) fn paint_android_tile(
        &self,
        s: &mut Surface,
        fonts: &mut Fonts,
        theme: &Theme,
        tile: RectF,
        action: QuickAction,
        label: &str,
        glyph: &str,
        index: usize,
        brightness: f32,
    ) {
        let d = config::design();
        let squeeze = self.press_scale(PressKey::Tile(index));
        let t = self.switch_spring(action).clamp(0.0, 1.0);
        let fill = Theme::mix(theme.surface_container_highest, theme.primary, t);
        let on = Theme::mix(theme.on_surface, theme.on_primary, t);
        let w = tile.w * squeeze;
        let h = tile.h * squeeze;
        let x = tile.x + (tile.w - w) / 2.0;
        let y = tile.y + (tile.h - h) / 2.0;
        // Classic Pixel controls retain a capsule silhouette in both states.
        let radius = h.min(w) / 2.0;
        s.fill_round_rect(x, y, w, h, radius, fill);
        let compact = tile.w < 196.0;
        let icon_y = if compact { 23.0 } else { tile.h / 2.0 };
        fonts.draw_icon_centered(
            s,
            tile.x + d.quick_settings.icon_x,
            tile.y + icon_y,
            glyph,
            d.quick_settings.icon_size * squeeze,
            on,
        );
        let details = matches!(action, QuickAction::Wifi | QuickAction::Bluetooth);
        let label_x = if compact { 22.0 } else { d.shade.tile_label_x };
        let right = if details && !compact { 40.0 } else { 18.0 };
        let text_width = (tile.w - label_x - right).max(1.0);
        let label_y = if compact {
            tile.h - 49.0
        } else {
            tile.h / 2.0 - 22.0
        };
        let status_y = if compact {
            tile.h - 28.0
        } else {
            tile.h / 2.0 + 3.0
        };
        let label_style = if compact {
            TextStyle::title_small()
        } else {
            TextStyle::title_medium()
        };
        let status_style = if compact {
            TextStyle::body_small()
        } else {
            TextStyle::body_medium()
        };
        let label = fit_text(fonts, label_style, label, text_width);
        fonts.draw_text_role(
            s,
            tile.x + label_x,
            tile.y + label_y,
            label_style,
            &label,
            on,
        );
        let status = match action {
            QuickAction::Wifi => self.system_status.wifi.label.clone(),
            QuickAction::Bluetooth => self.system_status.bluetooth.label.clone(),
            QuickAction::Brightness if !self.brightness_available => "Unavailable".into(),
            QuickAction::Brightness => format!("{}%", (brightness * 100.0).round() as u32),
            QuickAction::NightLight if !self.night_light_available => "Unavailable".into(),
            _ => {
                if t > 0.5 {
                    "On".into()
                } else {
                    "Off".into()
                }
            }
        };
        let status = fit_text(fonts, status_style, &status, text_width);
        fonts.draw_text_role(
            s,
            tile.x + label_x,
            tile.y + status_y,
            status_style,
            &status,
            on,
        );
        if details {
            fonts.draw_icon_centered(
                s,
                tile.right() - d.shade.brightness_icon_x,
                tile.y + icon_y,
                "chevron_right",
                d.statusbar.icon_size,
                on,
            );
        }
        if self.press_tile == Some(action) {
            s.fill_round_rect(x, y, w, h, radius, theme.state_layer(on, d.state.pressed));
        }
        let clip = round_rect_path(x, y, w, h, radius);
        self.draw_ripple(s, PressKey::Tile(index), &clip, on);
    }
}

impl Shell {
    pub fn reset_shade_scroll(&mut self) {
        self.shade_scroll = 0.0;
        self.shade_scroll_target = 0.0;
        self.shade_scroll_motion.snap(0.0);
    }
    pub fn scroll_shade(&mut self, delta: f32, max: f32) {
        if !delta.is_finite() {
            return;
        }
        self.shade_scroll_target = (self.shade_scroll_target + delta).clamp(0.0, max.max(0.0));
        self.shade_scroll_motion
            .set_target(self.shade_scroll_target);
        if self.reduce_motion {
            self.shade_scroll_motion.finish();
            self.shade_scroll = self.shade_scroll_target;
        }
    }
}

pub fn shade_page_button(w: f32, h: f32) -> RectF {
    let rect = quick_settings_rect(w, h);
    let size = 2.0 * config::design().statusbar.height;
    RectF {
        x: rect.right() - 3.0 * config::design().shade.header_button_width,
        y: rect.y,
        w: config::design().shade.header_button_width,
        h: size,
    }
}

impl Shell {
    pub fn shade_scroll_limit(&self, w: f32, h: f32) -> f32 {
        if self.notifications_open && !shade_split(w) {
            self.notification_scroll_max(w, h)
        } else if self.appearance_page {
            let rect = quick_settings_rect(w, h);
            let d = &config::design().quick_settings;
            (palette_top(rect.w)
                + VARIANTS.len().div_ceil(palette_columns(rect.w)) as f32
                    * (d.chip_height + d.chip_gap)
                + d.padding
                - rect.h)
                .max(0.0)
        } else if let Some(page) = self.connection_page {
            let count = if page == QuickAction::Wifi {
                self.system_status.networks.len()
            } else {
                self.system_status.devices.len()
            };
            (quick_tiles_top()
                + 48.0
                + count as f32 * config::design().power.row_height
                + config::design().quick_settings.padding
                - quick_settings_rect(w, h).h)
                .max(0.0)
        } else {
            shade_scroll_max(w, h)
        }
    }
    pub fn clamp_shade_scroll(&mut self, w: f32, h: f32) -> bool {
        let notice_max = self.notification_scroll_max(w, h);
        self.notice_scroll_target = self.notice_scroll_target.clamp(0.0, notice_max);
        let notice_clamped = self.notice_scroll.value().clamp(0.0, notice_max);
        let notice_changed = notice_clamped != self.notice_scroll.value();
        if notice_changed {
            self.notice_scroll.snap(notice_clamped);
        }
        self.notice_scroll.set_target(self.notice_scroll_target);
        let max = self.shade_scroll_limit(w, h);
        self.shade_scroll_target = self.shade_scroll_target.clamp(0.0, max);
        let clamped = self.shade_scroll.clamp(0.0, max);
        let changed = clamped != self.shade_scroll;
        if changed {
            self.shade_scroll = clamped;
            self.shade_scroll_motion.snap(clamped);
        }
        self.shade_scroll_motion
            .set_target(self.shade_scroll_target);
        changed || notice_changed
    }
    pub fn scroll_notifications(&mut self, delta: f32, w: f32, h: f32) {
        if !delta.is_finite() {
            return;
        }
        self.notice_scroll_target =
            (self.notice_scroll_target + delta).clamp(0.0, self.notification_scroll_max(w, h));
        self.notice_scroll.set_target(self.notice_scroll_target);
        if self.reduce_motion {
            self.notice_scroll.finish();
        }
    }
}
