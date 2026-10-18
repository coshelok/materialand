//! Topbar.
use super::*;

impl Shell {
    pub fn render_topbar(
        &mut self,
        theme: &Theme,
        fonts: &mut Fonts,
        win_w: f32,
        _title: &str,
        mask: Option<CircleMask>,
    ) -> (MemoryRenderBuffer, Point<f64, Logical>) {
        let rect = topbar_rect(win_w);
        let loc = Point::from((rect.x as f64, rect.y as f64));
        let key = KeyHash::new()
            .i32(win_w.round() as i32)
            .u64(theme.cache_key())
            .str(&self.clock)
            .i32(self.hover_top.map_or(-1, |a| a as i32))
            .i32(self.press_top.map_or(-1, |a| a as i32))
            .bool(self.quick_settings_open)
            .finish();
        let animating = self.press.keys().any(|k| matches!(k, PressKey::Top(_)));
        if !animating {
            if let Some(piece) = self.topbar_cache.iter().find(|p| p.key == key) {
                return masked_from(&piece.surface, mask, &piece.buffer, loc);
            }
        }
        let mut s = Surface::new(rect.w as u32, rect.h as u32);
        let cy = rect.h / 2.0;

        // Left side: launcher button, then the clock (classic Android phone
        // style). The launcher occupies the leading full-height square.
        self.draw_icon_button(
            &mut s,
            fonts,
            theme,
            topbar_button_rect(win_w, TopAction::Launcher),
            TopAction::Launcher,
            "apps",
            20.0,
            theme.on_surface,
            None,
        );
        let clock_style = TextStyle::LABEL_LARGE;
        fonts.draw_text_role(
            &mut s,
            16.0 + config::TOPBAR_HEIGHT + 4.0,
            cy - clock_style.line_height / 2.0,
            clock_style,
            &self.clock,
            theme.on_surface,
        );

        // Right side: status icons, right to left.
        self.draw_icon_button(
            &mut s,
            fonts,
            theme,
            topbar_button_rect(win_w, TopAction::Power),
            TopAction::Power,
            "account_circle",
            18.0,
            theme.on_surface,
            Some(theme.surface_container_high),
        );
        let theme_icon = if theme.is_dark {
            "light_mode"
        } else {
            "dark_mode"
        };
        self.draw_icon_button(
            &mut s,
            fonts,
            theme,
            topbar_button_rect(win_w, TopAction::Theme),
            TopAction::Theme,
            theme_icon,
            18.0,
            theme.on_surface_variant,
            None,
        );
        self.draw_icon_button(
            &mut s,
            fonts,
            theme,
            topbar_button_rect(win_w, TopAction::Brightness),
            TopAction::Brightness,
            "brightness_medium",
            18.0,
            theme.on_surface_variant,
            None,
        );
        let qs_bg = self
            .quick_settings_open
            .then_some(theme.secondary_container);
        self.draw_icon_button(
            &mut s,
            fonts,
            theme,
            topbar_button_rect(win_w, TopAction::QuickSettings),
            TopAction::QuickSettings,
            "settings",
            18.0,
            if self.quick_settings_open {
                theme.on_secondary_container
            } else {
                theme.on_surface_variant
            },
            qs_bg,
        );

        let buffer = s.to_memory_buffer();
        // During a radial reveal even a fresh piece must come back masked, or
        // its first frame would flash the full new scheme.
        let result = masked_from(&s, mask, &buffer, loc);
        if !animating {
            self.topbar_cache.insert(
                0,
                Piece {
                    key,
                    surface: s,
                    buffer,
                },
            );
            self.topbar_cache.truncate(4);
        }
        result
    }
}
