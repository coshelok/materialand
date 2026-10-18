//! Material color roles and interpolation.
use super::*;
impl Theme {
    /// Build a full Material 3 scheme for `spec`.
    pub fn from_spec(spec: ThemeSpec, dark: bool) -> Self {
        let built = ThemeBuilder::with_source(Argb::from_u32(spec.seed))
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
            self.primary,
            self.on_primary,
            self.primary_container,
            self.on_primary_container,
            self.inverse_primary,
            self.primary_fixed,
            self.primary_fixed_dim,
            self.on_primary_fixed,
            self.on_primary_fixed_variant,
            self.secondary,
            self.on_secondary,
            self.secondary_container,
            self.on_secondary_container,
            self.secondary_fixed,
            self.secondary_fixed_dim,
            self.on_secondary_fixed,
            self.on_secondary_fixed_variant,
            self.tertiary,
            self.on_tertiary,
            self.tertiary_container,
            self.on_tertiary_container,
            self.tertiary_fixed,
            self.tertiary_fixed_dim,
            self.on_tertiary_fixed,
            self.on_tertiary_fixed_variant,
            self.error,
            self.on_error,
            self.error_container,
            self.on_error_container,
            self.background,
            self.on_background,
            self.surface,
            self.on_surface,
            self.surface_dim,
            self.surface_bright,
            self.surface_container_lowest,
            self.surface_container_low,
            self.surface_container,
            self.surface_container_high,
            self.surface_container_highest,
            self.surface_variant,
            self.on_surface_variant,
            self.surface_tint,
            self.outline,
            self.outline_variant,
            self.shadow,
            self.scrim,
            self.inverse_surface,
            self.inverse_on_surface,
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
