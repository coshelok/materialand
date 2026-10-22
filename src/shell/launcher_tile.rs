//! Component painting.
use super::*;
impl Shell {
    #[allow(clippy::too_many_arguments)]
    pub(super) fn draw_launcher_tile(
        &self,
        s: &mut Surface,
        fonts: &mut Fonts,
        theme: &Theme,
        app: &AppEntry,
        x: f32,
        y: f32,
        scale: f32,
    ) {
        let design = crate::config::design();
        if let Some(icon) = self.app_icons.get(&app.icon) {
            let side = 2.0 * design.apps.medallion_radius * scale;
            let ratio = side / (icon.width().max(icon.pixmap.height()) as f32);
            let x = x + design.launcher.tile_size / 2.0 - icon.width() as f32 * ratio / 2.0;
            let y = y + design.apps.icon_center_y - icon.pixmap.height() as f32 * ratio / 2.0;
            s.blit(
                icon,
                SkTransform::from_scale(ratio, ratio).post_translate(x, y),
                1.0,
                None,
            );
        } else {
            let icon_name = config::icon_for_key(&app.icon);
            let (fill, on) = app_colors(theme, &app.icon);
            s.circle(
                x + design.launcher.tile_size / 2.0,
                y + design.apps.icon_center_y,
                design.apps.medallion_radius * scale,
                fill,
            );
            fonts.draw_icon_centered(
                s,
                x + design.launcher.tile_size / 2.0,
                y + design.apps.icon_center_y,
                icon_name,
                design.apps.glyph_size * scale,
                on,
            );
        }
        let label_style = TextStyle::body_medium();
        let caption = truncate(&app.name, design.apps.label_max_chars);
        let label = fit_text(fonts, label_style, &caption, design.launcher.tile_size);
        let lw = fonts.text_width_styled(label_style, &label);
        fonts.draw_text_role(
            s,
            x + design.launcher.tile_size / 2.0 - lw / 2.0,
            y + design.apps.label_y,
            label_style,
            &label,
            theme.on_surface,
        );
    }
}
