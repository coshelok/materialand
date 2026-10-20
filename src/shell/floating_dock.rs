//! A shared elevated pastel capsule; exterior shadow pixels never receive input.
use super::*;

pub fn floating_dock_padding() -> f32 {
    config::design().floating_dock.shadow_padding.ceil()
}
pub(super) fn foreground(theme: &Theme) -> Color {
    theme.on_secondary_fixed
}
pub(super) fn surface(theme: &Theme, w: f32, h: f32) -> Surface {
    let d = &config::design().floating_dock;
    let pad = floating_dock_padding();
    let mut surface = Surface::new((w.ceil() + 2.0 * pad) as u32, (h.ceil() + 2.0 * pad) as u32);
    surface.shadow_round_rect(pad, pad, w, h, h / 2.0, d.shadow_spread, theme.shadow);
    let fill = Theme::mix(theme.secondary_fixed, theme.primary_fixed, d.tint_mix);
    surface.fill_round_rect(
        pad,
        pad,
        w,
        h,
        h / 2.0,
        Theme::with_alpha(fill, d.background_alpha),
    );
    surface.stroke_round_rect(
        pad,
        pad,
        w,
        h,
        h / 2.0,
        d.outline_width,
        Theme::with_alpha(foreground(theme), d.outline_alpha),
    );
    surface
}
