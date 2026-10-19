use material_colors::{color::Argb, dynamic_color::Variant, theme::ThemeBuilder};
use std::path::{Path, PathBuf};
use tiny_skia::Color;
mod cache_key;
pub use cache_key::KeyHash;
mod seed;
pub use seed::*;
mod overrides;
mod roles;
pub use overrides::{parse_role_color, valid_role};
/// The nine Material You colour variants exposed by the palette picker.
///
/// This mirrors `material_colors::dynamic_color::Variant`, re-declared as a
/// small `Copy` enum so the UI can index it and cycle through it freely.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ColorVariant {
    TonalSpot,
    Vibrant,
    Expressive,
    Content,
    Fidelity,
    Rainbow,
    FruitSalad,
    Neutral,
    Monochrome,
}

/// Palette picker order: the pastel baseline first, then the expressive ones.
pub const VARIANTS: [ColorVariant; 9] = [
    ColorVariant::TonalSpot,
    ColorVariant::Vibrant,
    ColorVariant::Expressive,
    ColorVariant::Content,
    ColorVariant::Fidelity,
    ColorVariant::Rainbow,
    ColorVariant::FruitSalad,
    ColorVariant::Neutral,
    ColorVariant::Monochrome,
];

impl ColorVariant {
    pub fn material(self) -> Variant {
        match self {
            ColorVariant::TonalSpot => Variant::TonalSpot,
            ColorVariant::Vibrant => Variant::Vibrant,
            ColorVariant::Expressive => Variant::Expressive,
            ColorVariant::Content => Variant::Content,
            ColorVariant::Fidelity => Variant::Fidelity,
            ColorVariant::Rainbow => Variant::Rainbow,
            ColorVariant::FruitSalad => Variant::FruitSalad,
            ColorVariant::Neutral => Variant::Neutral,
            ColorVariant::Monochrome => Variant::Monochrome,
        }
    }

    /// Human-readable name shown in the quick-settings palette grid.
    pub fn name(self) -> &'static str {
        match self {
            ColorVariant::TonalSpot => "Tonal spot",
            ColorVariant::Vibrant => "Vibrant",
            ColorVariant::Expressive => "Expressive",
            ColorVariant::Content => "Content",
            ColorVariant::Fidelity => "Fidelity",
            ColorVariant::Rainbow => "Rainbow",
            ColorVariant::FruitSalad => "Fruit salad",
            ColorVariant::Neutral => "Neutral",
            ColorVariant::Monochrome => "Monochrome",
        }
    }

    pub fn from_name(name: &str) -> Option<Self> {
        let want = name
            .trim()
            .to_ascii_lowercase()
            .replace([' ', '-', '_'], "");
        VARIANTS
            .into_iter()
            .find(|v| v.name().to_ascii_lowercase().replace([' ', '-', '_'], "") == want)
    }
}

/// A full palette description: the seed color plus the Material You variant.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ThemeSpec {
    pub seed: u32,
    pub variant: ColorVariant,
}

impl ThemeSpec {
    /// Resolve seed extraction once after the central design file is loaded.
    pub fn from_config() -> (Self, SeedSource) {
        let mut spec = Self::default();
        let appearance = &crate::config::design().appearance;
        if appearance.seed.is_empty() && !appearance.wallpaper.is_empty() {
            let path = PathBuf::from(&appearance.wallpaper);
            if let Some(seed) = seed_from_wallpaper(&path) {
                spec.seed = seed;
                return (spec, SeedSource::Wallpaper(path));
            }
            return (spec, SeedSource::Default);
        }
        (spec, SeedSource::Configuration)
    }

    pub fn dark(self) -> Theme {
        Theme::from_spec(self, true)
    }

    pub fn light(self) -> Theme {
        Theme::from_spec(self, false)
    }
}

impl Default for ThemeSpec {
    fn default() -> Self {
        let appearance = &crate::config::design().appearance;
        Self {
            seed: parse_seed(&appearance.seed).unwrap_or_else(|| {
                parse_seed(&crate::config::built_in_design().appearance.seed)
                    .expect("built-in seed")
            }),
            variant: ColorVariant::from_name(&appearance.variant).expect("validated palette"),
        }
    }
}

/// Where the palette seed came from, for logging.
#[derive(Clone, Debug)]
pub enum SeedSource {
    Default,
    Configuration,
    Wallpaper(PathBuf),
}

impl SeedSource {
    pub fn describe(&self) -> String {
        match self {
            SeedSource::Default => "the default Material baseline seed".into(),
            SeedSource::Configuration => "the central design configuration".into(),
            SeedSource::Wallpaper(path) => format!("the wallpaper {}", path.display()),
        }
    }
}

/// Parse `#RRGGBB`, `RRGGBB`, `0xAARRGGBB`-style seed colors.
/// The complete set of Material 3 color roles, resolved for one scheme.
///
/// Every field is an opaque `tiny_skia::Color`; translucency is derived at draw
/// time through [`Theme::with_alpha`] so the roles stay faithful to the spec.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Theme {
    pub is_dark: bool,

    pub primary: Color,
    pub on_primary: Color,
    pub primary_container: Color,
    pub on_primary_container: Color,
    pub inverse_primary: Color,
    pub primary_fixed: Color,
    pub primary_fixed_dim: Color,
    pub on_primary_fixed: Color,
    pub on_primary_fixed_variant: Color,

    pub secondary: Color,
    pub on_secondary: Color,
    pub secondary_container: Color,
    pub on_secondary_container: Color,
    pub secondary_fixed: Color,
    pub secondary_fixed_dim: Color,
    pub on_secondary_fixed: Color,
    pub on_secondary_fixed_variant: Color,

    pub tertiary: Color,
    pub on_tertiary: Color,
    pub tertiary_container: Color,
    pub on_tertiary_container: Color,
    pub tertiary_fixed: Color,
    pub tertiary_fixed_dim: Color,
    pub on_tertiary_fixed: Color,
    pub on_tertiary_fixed_variant: Color,

    pub error: Color,
    pub on_error: Color,
    pub error_container: Color,
    pub on_error_container: Color,

    pub background: Color,
    pub on_background: Color,

    pub surface: Color,
    pub on_surface: Color,
    pub surface_dim: Color,
    pub surface_bright: Color,
    pub surface_container_lowest: Color,
    pub surface_container_low: Color,
    pub surface_container: Color,
    pub surface_container_high: Color,
    pub surface_container_highest: Color,
    pub surface_variant: Color,
    pub on_surface_variant: Color,
    pub surface_tint: Color,

    pub outline: Color,
    pub outline_variant: Color,

    pub shadow: Color,
    pub scrim: Color,

    pub inverse_surface: Color,
    pub inverse_on_surface: Color,
}

fn role(argb: Argb) -> Color {
    Color::from_rgba8(argb.red, argb.green, argb.blue, argb.alpha)
}

/// The three accent tones a palette variant contributes, as used by the
/// palette-picker swatches.
pub fn variant_swatch(spec: ThemeSpec, dark: bool) -> (Color, Color, Color) {
    let t = Theme::from_spec(spec, dark);
    (t.primary, t.secondary, t.tertiary)
}

#[cfg(test)]
mod tests;
