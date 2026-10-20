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
        let radius = d.quick_settings.tile_radius
            + d.quick_settings.tile_active_radius_growth * t
            + d.quick_settings.tile_press_radius_growth * (1.0 - squeeze);
        s.fill_round_rect(x, y, w, h, radius, fill);
        fonts.draw_icon_centered(
            s,
            tile.x + d.quick_settings.icon_x,
            tile.y + tile.h / 2.0,
            glyph,
            d.quick_settings.icon_size * squeeze,
            on,
        );
        let details = matches!(action, QuickAction::Wifi | QuickAction::Bluetooth);
        let right = if details {
            d.shade.tile_label_x
        } else {
            d.quick_settings.label_x
        };
        let text_width = (tile.w - d.shade.tile_label_x - right).max(1.0);
        let label = fit_text(fonts, TextStyle::title_small(), label, text_width);
        fonts.draw_text_role(
            s,
            tile.x + d.shade.tile_label_x,
            tile.y + d.shade.tile_label_y,
            TextStyle::title_small(),
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
        let status = fit_text(fonts, TextStyle::body_small(), &status, text_width);
        fonts.draw_text_role(
            s,
            tile.x + d.shade.tile_label_x,
            tile.y + d.shade.tile_status_y,
            TextStyle::body_small(),
            &status,
            on,
        );
        if details {
            fonts.draw_icon_centered(
                s,
                tile.right() - d.shade.brightness_icon_x,
                tile.y + tile.h / 2.0,
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
