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
        s.circle(
            x + config::LAUNCHER_TILE / 2.0,
            y + 36.0,
            32.0 * scale,
            theme.surface_container_highest,
        );
        fonts.draw_icon_centered(
            s,
            x + config::LAUNCHER_TILE / 2.0,
            y + 36.0,
            icon_name,
            36.0 * scale,
            theme.on_surface,
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
