//! Optional per-scheme Material color roles from the central configuration.
use super::*;
pub fn parse_role_color(raw: &str) -> Option<Color> {
    let raw = raw.trim();
    let digits = raw.strip_prefix("#").unwrap_or(raw);
    if digits.len() != 6 {
        return None;
    }
    let rgb = u32::from_str_radix(digits, 16).ok()?;
    Some(Color::from_rgba8(
        (rgb >> 16) as u8,
        (rgb >> 8) as u8,
        rgb as u8,
        255,
    ))
}
pub fn valid_role(name: &str) -> bool {
    matches!(
        name,
        "primary"
            | "on_primary"
            | "primary_container"
            | "on_primary_container"
            | "inverse_primary"
            | "primary_fixed"
            | "primary_fixed_dim"
            | "on_primary_fixed"
            | "on_primary_fixed_variant"
            | "secondary"
            | "on_secondary"
            | "secondary_container"
            | "on_secondary_container"
            | "secondary_fixed"
            | "secondary_fixed_dim"
            | "on_secondary_fixed"
            | "on_secondary_fixed_variant"
            | "tertiary"
            | "on_tertiary"
            | "tertiary_container"
            | "on_tertiary_container"
            | "tertiary_fixed"
            | "tertiary_fixed_dim"
            | "on_tertiary_fixed"
            | "on_tertiary_fixed_variant"
            | "error"
            | "on_error"
            | "error_container"
            | "on_error_container"
            | "background"
            | "on_background"
            | "surface"
            | "on_surface"
            | "surface_dim"
            | "surface_bright"
            | "surface_container_lowest"
            | "surface_container_low"
            | "surface_container"
            | "surface_container_high"
            | "surface_container_highest"
            | "surface_variant"
            | "on_surface_variant"
            | "surface_tint"
            | "outline"
            | "outline_variant"
            | "shadow"
            | "scrim"
            | "inverse_surface"
            | "inverse_on_surface"
    )
}
impl Theme {
    pub(super) fn apply_configured_colors(&mut self) {
        let colors = &crate::config::design().colors;
        let overrides = if self.is_dark {
            &colors.dark
        } else {
            &colors.light
        };
        for (name, value) in overrides {
            let Some(color) = parse_role_color(value) else {
                continue;
            };
            match name.as_str() {
                "primary" => self.primary = color,
                "on_primary" => self.on_primary = color,
                "primary_container" => self.primary_container = color,
                "on_primary_container" => self.on_primary_container = color,
                "inverse_primary" => self.inverse_primary = color,
                "primary_fixed" => self.primary_fixed = color,
                "primary_fixed_dim" => self.primary_fixed_dim = color,
                "on_primary_fixed" => self.on_primary_fixed = color,
                "on_primary_fixed_variant" => self.on_primary_fixed_variant = color,
                "secondary" => self.secondary = color,
                "on_secondary" => self.on_secondary = color,
                "secondary_container" => self.secondary_container = color,
                "on_secondary_container" => self.on_secondary_container = color,
                "secondary_fixed" => self.secondary_fixed = color,
                "secondary_fixed_dim" => self.secondary_fixed_dim = color,
                "on_secondary_fixed" => self.on_secondary_fixed = color,
                "on_secondary_fixed_variant" => self.on_secondary_fixed_variant = color,
                "tertiary" => self.tertiary = color,
                "on_tertiary" => self.on_tertiary = color,
                "tertiary_container" => self.tertiary_container = color,
                "on_tertiary_container" => self.on_tertiary_container = color,
                "tertiary_fixed" => self.tertiary_fixed = color,
                "tertiary_fixed_dim" => self.tertiary_fixed_dim = color,
                "on_tertiary_fixed" => self.on_tertiary_fixed = color,
                "on_tertiary_fixed_variant" => self.on_tertiary_fixed_variant = color,
                "error" => self.error = color,
                "on_error" => self.on_error = color,
                "error_container" => self.error_container = color,
                "on_error_container" => self.on_error_container = color,
                "background" => self.background = color,
                "on_background" => self.on_background = color,
                "surface" => self.surface = color,
                "on_surface" => self.on_surface = color,
                "surface_dim" => self.surface_dim = color,
                "surface_bright" => self.surface_bright = color,
                "surface_container_lowest" => self.surface_container_lowest = color,
                "surface_container_low" => self.surface_container_low = color,
                "surface_container" => self.surface_container = color,
                "surface_container_high" => self.surface_container_high = color,
                "surface_container_highest" => self.surface_container_highest = color,
                "surface_variant" => self.surface_variant = color,
                "on_surface_variant" => self.on_surface_variant = color,
                "surface_tint" => self.surface_tint = color,
                "outline" => self.outline = color,
                "outline_variant" => self.outline_variant = color,
                "shadow" => self.shadow = color,
                "scrim" => self.scrim = color,
                "inverse_surface" => self.inverse_surface = color,
                "inverse_on_surface" => self.inverse_on_surface = color,
                _ => {}
            }
        }
    }
}
