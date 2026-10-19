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
        let icon_name = config::icon_for_key(&app.icon);
        let (fill, on) = app_colors(theme, &app.icon);
        s.circle(
            x + config::LAUNCHER_TILE / 2.0,
            y + 36.0,
            34.0 * scale,
            fill,
        );
        fonts.draw_icon_centered(
            s,
            x + config::LAUNCHER_TILE / 2.0,
            y + 36.0,
            icon_name,
            30.0 * scale,
            on,
        );
        let label = truncate(&app.name, 14);
        let label_style = TextStyle::BODY_MEDIUM;
        let lw = fonts.text_width_styled(label_style, &label);
        fonts.draw_text_role(
            s,
            x + config::LAUNCHER_TILE / 2.0 - lw / 2.0,
            y + 76.0,
            label_style,
            &label,
            theme.on_surface,
        );
    }
}
