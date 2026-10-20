//! Android-style app identity strips and actions; live pixels are composed on the GPU.
use super::*;

pub(super) fn paint_recent_card(
    theme: &Theme,
    fonts: &mut Fonts,
    meta: &WindowMeta,
    size: (f32, f32),
    selected: bool,
) -> Surface {
    let d = &config::design().overview;
    let (w, h) = size;
    let mut s = Surface::new(
        w.ceil() as u32,
        (h + d.app_header_height + d.footer_height).ceil() as u32,
    );
    let badge = d.app_header_height * 0.7;
    s.circle(
        badge / 2.0,
        d.app_header_height / 2.0,
        badge / 2.0,
        theme.primary_fixed,
    );
    fonts.draw_icon_centered(
        &mut s,
        badge / 2.0,
        d.app_header_height / 2.0,
        config::icon_for_key(&meta.app_id),
        config::design().statusbar.icon_size,
        theme.on_primary_fixed,
    );
    let label = fit_text(
        fonts,
        TextStyle::title_small(),
        &meta.title,
        (w - badge - 24.0).max(1.0),
    );
    fonts.draw_text_role(
        &mut s,
        badge + 12.0,
        (d.app_header_height - TextStyle::title_small().line_height) / 2.0,
        TextStyle::title_small(),
        &label,
        theme.on_surface,
    );
    s.fill_round_rect(
        0.0,
        d.app_header_height,
        w,
        h,
        d.card_radius,
        theme.surface_container_low,
    );
    let footer_y = d.app_header_height + h;
    let button_h = (d.footer_height - 16.0).max(1.0);
    let open_width = (w - 120.0).clamp(1.0, 160.0);
    s.fill_round_rect(
        0.0,
        footer_y + 8.0,
        open_width,
        button_h,
        button_h / 2.0,
        theme.secondary_container,
    );
    let open = fit_text(
        fonts,
        TextStyle::label_large(),
        "Open app",
        open_width - 24.0,
    );
    fonts.draw_text_role(
        &mut s,
        12.0,
        footer_y + (d.footer_height - TextStyle::label_large().line_height) / 2.0,
        TextStyle::label_large(),
        &open,
        theme.on_secondary_container,
    );
    for (x, icon, active) in [
        (
            w - 84.0,
            if selected {
                "check_circle"
            } else {
                "splitscreen"
            },
            selected,
        ),
        (w - 28.0, "close", false),
    ] {
        s.circle(
            x,
            footer_y + d.footer_height / 2.0,
            button_h / 2.0,
            if active {
                theme.primary_container
            } else {
                theme.surface_container_high
            },
        );
        fonts.draw_icon_centered(
            &mut s,
            x,
            footer_y + d.footer_height / 2.0,
            icon,
            config::design().statusbar.icon_size,
            if active {
                theme.on_primary_container
            } else {
                theme.on_surface
            },
        );
    }
    s
}
impl Shell {
    pub fn overview_card(
        &mut self,
        window: &Window,
        theme: &Theme,
        fonts: &mut Fonts,
        meta: &WindowMeta,
        size: (f32, f32),
        selected: bool,
    ) -> MemoryRenderBuffer {
        let (w, h) = size;
        let key = KeyHash::new()
            .u64(theme.cache_key())
            .str(&meta.title)
            .str(&meta.app_id)
            .u32(w.to_bits())
            .u32(h.to_bits())
            .bool(selected)
            .finish();
        let cache = self.overview_cards.entry(window.clone()).or_default();
        if let Some(piece) = cache.iter().find(|p| p.key == key) {
            return piece.buffer.clone();
        }
        let surface = paint_recent_card(theme, fonts, meta, size, selected);
        let buffer = surface.to_memory_buffer();
        cache.push(Piece {
            key,
            surface,
            buffer: buffer.clone(),
        });
        if cache.len() > 2 {
            cache.remove(0);
        }
        buffer
    }
    pub fn overview_header(
        &mut self,
        theme: &Theme,
        fonts: &mut Fonts,
        w: f32,
        label: &str,
    ) -> MemoryRenderBuffer {
        let key = KeyHash::new()
            .u64(theme.cache_key())
            .str(label)
            .u32(w.to_bits())
            .finish();
        if let Some(piece) = &self.overview_header {
            if piece.key == key {
                return piece.buffer.clone();
            }
        }
        let d = &config::design().overview;
        let mut s = Surface::new(w.ceil() as u32, d.header_height.ceil() as u32);
        let label = fit_text(
            fonts,
            TextStyle::title_medium(),
            label,
            (w - 2.0 * d.margin).max(1.0),
        );
        let text_w = fonts.text_width_styled(TextStyle::title_medium(), &label);
        fonts.draw_text_role(
            &mut s,
            (w - text_w) / 2.0,
            top_inset() + 12.0,
            TextStyle::title_medium(),
            &label,
            theme.on_surface,
        );
        let buffer = s.to_memory_buffer();
        self.overview_header = Some(Piece {
            key,
            surface: s,
            buffer: buffer.clone(),
        });
        buffer
    }
    pub fn retain_overview_cards(&mut self, windows: &[Window]) {
        self.overview_cards.retain(|w, _| windows.contains(w));
    }
}
