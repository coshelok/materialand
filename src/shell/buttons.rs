//! Component painting.
use super::*;
impl Shell {
    #[allow(clippy::too_many_arguments)]
    pub(super) fn draw_icon_button(
        &self,
        s: &mut Surface,
        fonts: &mut Fonts,
        theme: &Theme,
        rect: RectF,
        action: TopAction,
        icon: &str,
        icon_px: f32,
        icon_color: Color,
        bg: Option<Color>,
    ) {
        let cx = rect.x + rect.w / 2.0;
        let cy = rect.y + rect.h / 2.0;
        let key = PressKey::Top(action);
        let scale = self.press_scale(key);
        if let Some(bg) = bg {
            s.circle(cx, cy, rect.w / 2.0 * 0.84 * scale, bg);
        }
        fonts.draw_icon_centered(s, cx, cy, icon, icon_px * scale, icon_color);
        state_layer_overlay(
            s,
            theme,
            cx,
            cy,
            rect.w / 2.0,
            self.hover_top == Some(action),
            self.press_top == Some(action),
        );
        let mut cb = PathBuilder::new();
        cb.push_circle(cx, cy, rect.w / 2.0);
        if let Some(clip) = cb.finish() {
            self.draw_ripple(s, key, &clip, theme.on_surface);
        }
    }
}
