use std::path::{Path, PathBuf};

use material_colors::{color::Argb, dynamic_color::Variant, theme::ThemeBuilder};
use tiny_skia::Color;

/// A small deterministic hasher (FNV-1a) that turns cache inputs — palettes,
/// sizes, titles, queries — into compact `u64` keys.
///
/// The chrome caches used to compare multi-kilobyte hex strings rebuilt on
/// every frame; hashing the inputs costs a few hundred instructions and no
/// allocations.
#[derive(Clone, Copy, Debug, Default)]
pub struct KeyHash(u64);

impl KeyHash {
    pub fn new() -> Self {
        Self(0xcbf2_9ce4_8422_2325)
    }

    pub fn bytes(mut self, bytes: &[u8]) -> Self {
        for &b in bytes {
            self.0 ^= b as u64;
            self.0 = self.0.wrapping_mul(0x0000_0100_0000_01b3);
        }
        self
    }

    pub fn bool(self, value: bool) -> Self {
        self.bytes(&[value as u8])
    }

    pub fn u32(self, value: u32) -> Self {
        self.bytes(&value.to_le_bytes())
    }

    pub fn i32(self, value: i32) -> Self {
        self.u32(value as u32)
    }

    pub fn u64(self, value: u64) -> Self {
        self.bytes(&value.to_le_bytes())
    }

    pub fn f32(self, value: f32) -> Self {
        self.u32(value.to_bits())
    }

    pub fn str(self, value: &str) -> Self {
        self.bytes(value.as_bytes())
    }

    pub fn finish(self) -> u64 {
        self.0
    }
}

/// Seed color for the Material 3 dynamic palette (the Material baseline purple).
pub const SEED: u32 = 0xff67_50a4;

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
        let want = name.trim().to_ascii_lowercase().replace([' ', '-', '_'], "");
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
    pub const DEFAULT: ThemeSpec = ThemeSpec {
        seed: SEED,
        variant: ColorVariant::TonalSpot,
    };

    /// Read `MATERIALAND_SEED`, `MATERIALAND_VARIANT` and
    /// `MATERIALAND_WALLPAPER` (in that precedence) once at start-up.
    pub fn from_env() -> (Self, SeedSource) {
        let mut spec = Self::DEFAULT;
        let source = match std::env::var("MATERIALAND_SEED")
            .ok()
            .and_then(|raw| parse_seed(&raw))
        {
            Some(seed) => {
                spec.seed = seed;
                SeedSource::Env
            }
            None => match std::env::var_os("MATERIALAND_WALLPAPER").map(PathBuf::from) {
                Some(path) => match seed_from_wallpaper(&path) {
                    Some(seed) => {
                        spec.seed = seed;
                        SeedSource::Wallpaper(path)
                    }
                    None => SeedSource::Default,
                },
                None => SeedSource::Default,
            },
        };
        if let Some(variant) = std::env::var("MATERIALAND_VARIANT")
            .ok()
            .and_then(|raw| ColorVariant::from_name(&raw))
        {
            spec.variant = variant;
        }
        (spec, source)
    }

    pub fn dark(self) -> Theme {
        Theme::from_spec(self, true)
    }

    pub fn light(self) -> Theme {
        Theme::from_spec(self, false)
    }
}

/// Where the palette seed came from, for logging.
#[derive(Clone, Debug)]
pub enum SeedSource {
    Default,
    Env,
    Wallpaper(PathBuf),
}

impl SeedSource {
    pub fn describe(&self) -> String {
        match self {
            SeedSource::Default => "the default Material baseline seed".into(),
            SeedSource::Env => "MATERIALAND_SEED".into(),
            SeedSource::Wallpaper(path) => format!("the wallpaper {}", path.display()),
        }
    }
}

/// Parse `#RRGGBB`, `RRGGBB`, `0xAARRGGBB`-style seed colors.
pub fn parse_seed(raw: &str) -> Option<u32> {
    let trimmed = raw.trim().trim_start_matches('#');
    let digits = trimmed
        .strip_prefix("0x")
        .or_else(|| trimmed.strip_prefix("0X"))
        .unwrap_or(trimmed);
    if digits.is_empty() || digits.len() > 8 || !digits.chars().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }
    let value = u32::from_str_radix(digits, 16).ok()?;
    // A six-digit value carries no alpha; make it fully opaque.
    Some(if digits.len() <= 6 {
        0xff00_0000 | value
    } else {
        value
    })
}

fn argb_to_u32(argb: Argb) -> u32 {
    ((argb.alpha as u32) << 24)
        | ((argb.red as u32) << 16)
        | ((argb.green as u32) << 8)
        | argb.blue as u32
}

/// Quick check that the file starts with a known image magic number.
///
/// `material_colors`' reader *panics* on undecodable data, so filtering out
/// obvious non-images here keeps the common mistake quiet.
fn looks_like_image(bytes: &[u8]) -> bool {
    let starts = |magic: &[u8]| bytes.starts_with(magic);
    starts(&[0x89, b'P', b'N', b'G'])
        || starts(&[0xff, 0xd8, 0xff])
        || starts(b"GIF8")
        || starts(b"BM")
        || starts(b"II*\0")
        || starts(b"MM\0*")
        || starts(b"qoif")
        || (bytes.len() >= 12 && starts(b"RIFF") && &bytes[8..12] == b"WEBP")
}

/// Extract the Material You source color from a wallpaper image, if possible.
pub fn seed_from_wallpaper(path: &Path) -> Option<u32> {
    let bytes = std::fs::read(path).ok()?;
    if !looks_like_image(&bytes) {
        tracing::warn!(path = %path.display(), "wallpaper is not a recognised image");
        return None;
    }
    // The reader panics on corrupt data; silence the hook for this call and
    // recover the failure as a plain `None`.
    let previous_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(|_| {}));
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let mut image = material_colors::image::ImageReader::open(path).ok()?;
        image.resize(96, 96, material_colors::image::FilterType::Triangle);
        Some(argb_to_u32(material_colors::image::ImageReader::extract_color(
            &image,
        )))
    }));
    std::panic::set_hook(previous_hook);
    match result {
        Ok(seed) => seed,
        Err(_) => {
            tracing::warn!(path = %path.display(), "wallpaper could not be decoded; ignoring it");
            None
        }
    }
}

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

impl Theme {
    /// Build a full Material 3 scheme for `spec`.
    pub fn from_spec(spec: ThemeSpec, dark: bool) -> Self {
        let built =
            ThemeBuilder::with_source(Argb::from_u32(spec.seed))
                .variant(spec.variant.material())
                .build();
        let s = if dark {
            built.schemes.dark
        } else {
            built.schemes.light
        };
        Self {
            is_dark: dark,
            primary: role(s.primary),
            on_primary: role(s.on_primary),
            primary_container: role(s.primary_container),
            on_primary_container: role(s.on_primary_container),
            inverse_primary: role(s.inverse_primary),
            primary_fixed: role(s.primary_fixed),
            primary_fixed_dim: role(s.primary_fixed_dim),
            on_primary_fixed: role(s.on_primary_fixed),
            on_primary_fixed_variant: role(s.on_primary_fixed_variant),
            secondary: role(s.secondary),
            on_secondary: role(s.on_secondary),
            secondary_container: role(s.secondary_container),
            on_secondary_container: role(s.on_secondary_container),
            secondary_fixed: role(s.secondary_fixed),
            secondary_fixed_dim: role(s.secondary_fixed_dim),
            on_secondary_fixed: role(s.on_secondary_fixed),
            on_secondary_fixed_variant: role(s.on_secondary_fixed_variant),
            tertiary: role(s.tertiary),
            on_tertiary: role(s.on_tertiary),
            tertiary_container: role(s.tertiary_container),
            on_tertiary_container: role(s.on_tertiary_container),
            tertiary_fixed: role(s.tertiary_fixed),
            tertiary_fixed_dim: role(s.tertiary_fixed_dim),
            on_tertiary_fixed: role(s.on_tertiary_fixed),
            on_tertiary_fixed_variant: role(s.on_tertiary_fixed_variant),
            error: role(s.error),
            on_error: role(s.on_error),
            error_container: role(s.error_container),
            on_error_container: role(s.on_error_container),
            background: role(s.background),
            on_background: role(s.on_background),
            surface: role(s.surface),
            on_surface: role(s.on_surface),
            surface_dim: role(s.surface_dim),
            surface_bright: role(s.surface_bright),
            surface_container_lowest: role(s.surface_container_lowest),
            surface_container_low: role(s.surface_container_low),
            surface_container: role(s.surface_container),
            surface_container_high: role(s.surface_container_high),
            surface_container_highest: role(s.surface_container_highest),
            surface_variant: role(s.surface_variant),
            on_surface_variant: role(s.on_surface_variant),
            surface_tint: role(s.surface_tint),
            outline: role(s.outline),
            outline_variant: role(s.outline_variant),
            shadow: role(s.shadow),
            scrim: role(s.scrim),
            inverse_surface: role(s.inverse_surface),
            inverse_on_surface: role(s.inverse_on_surface),
        }
    }

    /// Surface tone for a Material 3 elevation level (0..=5).
    ///
    /// Level 0 is the plain surface; higher levels climb the surface-container
    /// ramp. Level 5 uses `surface_bright` in dark schemes to emulate the peak
    /// tonal elevation used by dialogs and menus.
    pub fn surface_for_elevation(&self, level: u8) -> Color {
        match level.min(5) {
            0 => self.surface,
            1 => self.surface_container_low,
            2 => self.surface_container,
            3 => self.surface_container_high,
            4 => self.surface_container_highest,
            _ => {
                if self.is_dark {
                    self.surface_bright
                } else {
                    self.surface_container_highest
                }
            }
        }
    }

    /// Stable cache key for every rendered color role. `is_dark` alone is not
    /// sufficient: during a theme transition the interpolated palette changes
    /// every frame while `is_dark` changes only once.
    pub fn cache_key(&self) -> u64 {
        let mut h = KeyHash::new().bool(self.is_dark);
        for color in [
            self.primary, self.on_primary, self.primary_container, self.on_primary_container,
            self.inverse_primary, self.primary_fixed, self.primary_fixed_dim,
            self.on_primary_fixed, self.on_primary_fixed_variant, self.secondary,
            self.on_secondary, self.secondary_container, self.on_secondary_container,
            self.secondary_fixed, self.secondary_fixed_dim, self.on_secondary_fixed,
            self.on_secondary_fixed_variant, self.tertiary, self.on_tertiary,
            self.tertiary_container, self.on_tertiary_container, self.tertiary_fixed,
            self.tertiary_fixed_dim, self.on_tertiary_fixed, self.on_tertiary_fixed_variant,
            self.error, self.on_error, self.error_container, self.on_error_container,
            self.background, self.on_background, self.surface, self.on_surface,
            self.surface_dim, self.surface_bright, self.surface_container_lowest,
            self.surface_container_low, self.surface_container, self.surface_container_high,
            self.surface_container_highest, self.surface_variant, self.on_surface_variant,
            self.surface_tint, self.outline, self.outline_variant, self.shadow, self.scrim,
            self.inverse_surface, self.inverse_on_surface,
        ] {
            h = h
                .f32(color.red())
                .f32(color.green())
                .f32(color.blue())
                .f32(color.alpha());
        }
        h.finish()
    }

    /// A translucent version of `c` (straight RGB, replaced alpha).
    pub fn with_alpha(c: Color, alpha: f32) -> Color {
        Color::from_rgba(c.red(), c.green(), c.blue(), alpha.clamp(0.0, 1.0)).unwrap_or(c)
    }

    /// Linear blend between two colors (straight RGBA, component-wise).
    pub fn mix(a: Color, b: Color, t: f32) -> Color {
        let t = t.clamp(0.0, 1.0);
        let lerp = |x: f32, y: f32| x + (y - x) * t;
        Color::from_rgba(
            lerp(a.red(), b.red()),
            lerp(a.green(), b.green()),
            lerp(a.blue(), b.blue()),
            lerp(a.alpha(), b.alpha()),
        )
        .unwrap_or(a)
    }

    /// A "state layer" color: the `on_*` color at `opacity`, used for M3 hover
    /// and press overlays. `opacity` is expected in the spec's 0.08..=0.12 range.
    pub fn state_layer(&self, on: Color, opacity: f32) -> Color {
        Self::with_alpha(on, opacity)
    }

    /// Interpolate every color role between two schemes; `is_dark` flips at the
    /// midpoint. Used for the animated light/dark transition.
    pub fn blend(a: &Theme, b: &Theme, t: f32) -> Theme {
        macro_rules! blend {
            ($($field:ident),+ $(,)?) => {
                Theme {
                    is_dark: if t < 0.5 { a.is_dark } else { b.is_dark },
                    $( $field: Self::mix(a.$field, b.$field, t), )+
                }
            };
        }
        blend!(
            primary,
            on_primary,
            primary_container,
            on_primary_container,
            inverse_primary,
            primary_fixed,
            primary_fixed_dim,
            on_primary_fixed,
            on_primary_fixed_variant,
            secondary,
            on_secondary,
            secondary_container,
            on_secondary_container,
            secondary_fixed,
            secondary_fixed_dim,
            on_secondary_fixed,
            on_secondary_fixed_variant,
            tertiary,
            on_tertiary,
            tertiary_container,
            on_tertiary_container,
            tertiary_fixed,
            tertiary_fixed_dim,
            on_tertiary_fixed,
            on_tertiary_fixed_variant,
            error,
            on_error,
            error_container,
            on_error_container,
            background,
            on_background,
            surface,
            on_surface,
            surface_dim,
            surface_bright,
            surface_container_lowest,
            surface_container_low,
            surface_container,
            surface_container_high,
            surface_container_highest,
            surface_variant,
            on_surface_variant,
            surface_tint,
            outline,
            outline_variant,
            shadow,
            scrim,
            inverse_surface,
            inverse_on_surface,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn schemes_are_opaque_and_tagged() {
        for t in [ThemeSpec::DEFAULT.dark(), ThemeSpec::DEFAULT.light()] {
            assert_eq!(t.background.alpha(), 1.0);
            assert_eq!(t.on_surface.alpha(), 1.0);
            assert_eq!(t.primary.alpha(), 1.0);
            assert_eq!(t.surface_container_highest.alpha(), 1.0);
            assert_eq!(t.primary_fixed.alpha(), 1.0);
            assert_eq!(t.tertiary_fixed_dim.alpha(), 1.0);
        }
        assert!(ThemeSpec::DEFAULT.dark().is_dark);
        assert!(!ThemeSpec::DEFAULT.light().is_dark);
    }

    #[test]
    fn with_alpha_keeps_rgb_changes_alpha() {
        let t = ThemeSpec::DEFAULT.dark();
        let c = Theme::with_alpha(t.primary, 0.5);
        assert_eq!(c.red(), t.primary.red());
        assert_eq!(c.green(), t.primary.green());
        assert_eq!(c.blue(), t.primary.blue());
        assert!((c.alpha() - 0.5).abs() < 1e-4);
    }

    #[test]
    fn elevation_levels_clamp() {
        let t = ThemeSpec::DEFAULT.dark();
        assert_eq!(t.surface_for_elevation(0), t.surface);
        assert_eq!(t.surface_for_elevation(4), t.surface_container_highest);
        assert_eq!(t.surface_for_elevation(9), t.surface_for_elevation(5));
    }

    #[test]
    fn seed_is_deterministic_and_changes_palette() {
        let a = ThemeSpec::DEFAULT.dark();
        let b = ThemeSpec::DEFAULT.dark();
        assert_eq!(a, b, "same seed must reproduce the same scheme");
        let other = Theme::from_spec(
            ThemeSpec {
                seed: 0xff00_ff00,
                variant: ColorVariant::TonalSpot,
            },
            true,
        );
        assert_ne!(a.primary, other.primary, "different seeds differ");
    }

    #[test]
    fn mix_endpoints_are_exact() {
        let t = ThemeSpec::DEFAULT.dark();
        assert_eq!(Theme::mix(t.primary, t.secondary, 0.0), t.primary);
        assert_eq!(Theme::mix(t.primary, t.secondary, 1.0), t.secondary);
    }

    #[test]
    fn cache_key_changes_when_palette_changes() {
        let dark = ThemeSpec::DEFAULT.dark();
        let light = ThemeSpec::DEFAULT.light();
        assert_ne!(dark.cache_key(), light.cache_key());
        assert_eq!(dark.cache_key(), dark.cache_key());
    }

    #[test]
    fn all_nine_variants_build_distinct_schemes() {
        let mut primaries = std::collections::HashSet::new();
        for variant in VARIANTS {
            let t = Theme::from_spec(ThemeSpec { seed: SEED, variant }, true);
            primaries.insert((
                (t.primary.red() * 255.0) as u8,
                (t.primary.green() * 255.0) as u8,
                (t.primary.blue() * 255.0) as u8,
            ));
            assert_eq!(t.primary.alpha(), 1.0);
        }
        assert!(
            primaries.len() >= 5,
            "variants must actually change the palette, got {} distinct primaries",
            primaries.len()
        );
    }

    #[test]
    fn variant_names_round_trip() {
        for variant in VARIANTS {
            assert_eq!(ColorVariant::from_name(variant.name()), Some(variant));
        }
        assert_eq!(ColorVariant::from_name("fruit_salad"), Some(ColorVariant::FruitSalad));
        assert_eq!(ColorVariant::from_name("nonsense"), None);
    }

    #[test]
    fn parse_seed_accepts_common_forms() {
        assert_eq!(parse_seed("#6750A4"), Some(0xff67_50a4));
        assert_eq!(parse_seed("6750a4"), Some(0xff67_50a4));
        assert_eq!(parse_seed("0x6750A4"), Some(0xff67_50a4));
        assert_eq!(parse_seed("FF6750A4"), Some(0xff67_50a4));
        assert_eq!(parse_seed(""), None);
        assert_eq!(parse_seed("zzz"), None);
        assert_eq!(parse_seed("#123456789"), None);
    }

    #[test]
    fn wallpaper_extraction_fails_softly() {
        assert_eq!(seed_from_wallpaper(Path::new("/nonexistent/wall.png")), None);
        // A readable file that is not an image must also be rejected.
        let path = std::env::temp_dir().join("materialand-not-an-image.txt");
        std::fs::write(&path, "definitely not an image").unwrap();
        assert_eq!(seed_from_wallpaper(&path), None);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn fixed_roles_differ_between_light_and_dark() {
        let (light, dark) = (ThemeSpec::DEFAULT.light(), ThemeSpec::DEFAULT.dark());
        // Fixed roles are *fixed*: the same tonal value in both schemes.
        assert_eq!(light.primary_fixed, dark.primary_fixed);
        assert_eq!(light.on_primary_fixed, dark.on_primary_fixed);
        assert_eq!(light.secondary_fixed, dark.secondary_fixed);
        assert_eq!(light.tertiary_fixed, dark.tertiary_fixed);
        // ...but each container family differs from its neighbours.
        assert_ne!(light.primary_fixed, light.secondary_fixed);
    }
}
