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
            let (cropped, (ox, oy)) = surface.masked_circle_cropped(m.cx, m.cy, m.radius);
            (
                cropped.to_memory_buffer(),
                Point::from((loc.x + ox as f64, loc.y + oy as f64)),
            )
        }
    }
}

// ---- Layout -------------------------------------------------------------

pub(super) fn draw_m3_switch(s: &mut Surface, x: f32, y: f32, t: f32, theme: &Theme) {
    let t = t.max(0.0);
    let curve = t.clamp(0.0, 1.0);
    let track = Theme::mix(theme.surface_container_highest, theme.primary, curve);
    let handle_color = Theme::mix(theme.outline, theme.on_primary, curve);
    s.fill_round_rect(x, y, 52.0, 32.0, 16.0, track);
    if curve < 0.999 {
        s.stroke_round_rect(
            x + 0.5,
            y + 0.5,
            51.0,
            31.0,
            16.0,
            1.0,
            Theme::with_alpha(theme.outline, 1.0 - curve),
        );
    }
    let cx = x + 16.0 + 20.0 * t.clamp(0.0, 1.06);
    let radius = 8.0 + 4.0 * curve;
    s.circle(cx, y + 16.0, radius, handle_color);
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
    let opacity = if pressed {
        state::PRESSED
    } else if hovered {
        state::HOVER
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
