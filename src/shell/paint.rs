//! Shared clipping and control painting.
use super::*;
pub(super) fn masked_from(
    surface: &Surface,
    mask: Option<CircleMask>,
    cached: &MemoryRenderBuffer,
    loc: Point<f64, Logical>,
) -> (MemoryRenderBuffer, Point<f64, Logical>) {
    match mask {
        None => (cached.clone(), loc),
        Some(m) => {
            // Once the reveal circle covers the whole layer the mask is a
            // no-op; skipping it avoids cloning the full pixmap and building a
            // full-size coverage mask on the final frames of the animation.
            let w = surface.width() as f32;
            let h = surface.pixmap.height() as f32;
            let cx = m.cx - loc.x as f32;
            let cy = m.cy - loc.y as f32;
            let dx = cx.max(w - cx).max(0.0);
            let dy = cy.max(h - cy).max(0.0);
            if dx * dx + dy * dy <= m.radius * m.radius {
                return (cached.clone(), loc);
            }
            let (cropped, (ox, oy)) = surface.masked_circle_cropped(cx, cy, m.radius);
            (
                cropped.to_memory_buffer(),
                Point::from((loc.x + ox as f64, loc.y + oy as f64)),
            )
        }
    }
}

// ---- Layout -------------------------------------------------------------

pub(super) fn draw_m3_switch(s: &mut Surface, x: f32, y: f32, t: f32, theme: &Theme) {
    let design = crate::config::design();
    let t = t.max(0.0);
    let curve = t.clamp(0.0, 1.0);
    let track = Theme::mix(theme.surface_container_highest, theme.primary, curve);
    let handle_color = Theme::mix(theme.outline, theme.on_primary, curve);
    s.fill_round_rect(
        x,
        y,
        design.switch.width,
        design.switch.height,
        design.switch.height / 2.0,
        track,
    );
    if curve < 0.999 {
        s.stroke_round_rect(
            x + design.switch.stroke / 2.0,
            y + design.switch.stroke / 2.0,
            design.switch.width - design.switch.stroke,
            design.switch.height - design.switch.stroke,
            design.switch.height / 2.0,
            design.switch.stroke,
            Theme::with_alpha(theme.outline, 1.0 - curve),
        );
    }
    let cx = x
        + design.switch.height / 2.0
        + (design.switch.width - design.switch.height)
            * t.clamp(0.0, design.motion.overshoot_limit);
    let radius = design.switch.handle_off_radius
        + (design.switch.handle_on_radius - design.switch.handle_off_radius) * curve;
    s.circle(cx, y + design.switch.height / 2.0, radius, handle_color);
}
pub(super) fn state_layer_overlay(
    s: &mut Surface,
    theme: &Theme,
    cx: f32,
    cy: f32,
    r: f32,
    hovered: bool,
    pressed: bool,
) {
    let design = crate::config::design();
    let opacity = if pressed {
        design.state.pressed
    } else if hovered {
        design.state.hover
    } else {
        return;
    };
    s.circle(cx, cy, r, theme.state_layer(theme.on_surface, opacity));
}

/// Shorten `s` to at most `max` characters, adding an ellipsis when clipped.
pub(super) fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        return s.to_string();
    }
    if max == 0 {
        return String::new();
    }
    let mut out: String = s.chars().take(max - 1).collect();
    out.push('…');
    out
}

/// Pastel app medallions use complete Material foreground/background pairs.
/// Fixed container tones keep their soft appearance in both brightness modes.
pub(super) fn app_colors(theme: &Theme, key: &str) -> (Color, Color) {
    let icon = config::icon_for_key(key);
    let apps = &crate::config::design().apps;
    if apps.tertiary_icons.iter().any(|value| value == icon) {
        (theme.tertiary_fixed, theme.on_tertiary_fixed)
    } else if apps.secondary_icons.iter().any(|value| value == icon) {
        (theme.secondary_fixed, theme.on_secondary_fixed)
    } else {
        (theme.primary_fixed, theme.on_primary_fixed)
    }
}

/// Fit labels to the available width rather than relying on character counts.
pub(super) fn fit_text(fonts: &mut Fonts, style: TextStyle, text: &str, width: f32) -> String {
    if width <= 0.0 {
        return String::new();
    }
    let mut count = text.chars().count();
    let mut label = text.to_owned();
    while fonts.text_width_styled(style, &label) > width && count > 0 {
        count -= 1;
        label = truncate(text, count);
    }
    label
}
