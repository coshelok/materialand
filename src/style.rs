use crate::font::FontKind;

/// Material 3 type-scale roles used by the shell. `size`, `line_height` and
/// `tracking` follow the M3 spec; `kind` maps onto one of the embedded Roboto
/// weights.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TextStyle {
    pub size: f32,
    pub line_height: f32,
    pub tracking: f32,
    pub kind: FontKind,
}

impl TextStyle {
    const fn new(size: f32, line_height: f32, tracking: f32, kind: FontKind) -> Self {
        Self { size, line_height, tracking, kind }
    }

    // Material 3 type scale. Sizes/line heights are dp; tracking is in px at
    // scale 1. The compositor currently uses the roles listed below directly.
    /// Top-bar title.
    pub const TITLE_MEDIUM: Self = Self::new(16.0, 24.0, 0.15, FontKind::Medium);
    /// Window decoration title.
    pub const TITLE_SMALL: Self = Self::new(14.0, 20.0, 0.1, FontKind::Medium);
    /// Top-bar clock.
    pub const LABEL_LARGE: Self = Self::new(14.0, 20.0, 0.1, FontKind::Medium);
    pub const LABEL_MEDIUM: Self = Self::new(12.0, 16.0, 0.5, FontKind::Medium);
    pub const LABEL_SMALL: Self = Self::new(11.0, 16.0, 0.5, FontKind::Medium);
    /// Launcher search field.
    pub const BODY_LARGE: Self = Self::new(16.0, 24.0, 0.5, FontKind::Regular);
    pub const BODY_MEDIUM: Self = Self::new(14.0, 20.0, 0.25, FontKind::Regular);
    /// Launcher tile caption.
    pub const BODY_SMALL: Self = Self::new(12.0, 16.0, 0.4, FontKind::Regular);
}

/// Material 3 shape scale in logical dp.
pub mod shape {
    pub const MEDIUM: f32 = 12.0;
    /// Window frame and floating popups.
    pub const LARGE: f32 = 16.0;
    /// Launcher dialog.
    pub const EXTRA_LARGE: f32 = 28.0;
    pub const EXTRA_EXTRA_LARGE: f32 = 48.0;
}

/// Material 3 state-layer opacities, applied over the appropriate `on_*` role.
pub mod state {
    pub const HOVER: f32 = 0.08;
    pub const PRESSED: f32 = 0.10;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn label_and_body_styles_are_distinct_weights() {
        assert_eq!(TextStyle::LABEL_LARGE.kind, FontKind::Medium);
        assert_eq!(TextStyle::BODY_LARGE.kind, FontKind::Regular);
    }

    #[test]
    fn title_medium_matches_m3_spec() {
        assert_eq!(TextStyle::TITLE_MEDIUM.size, 16.0);
        assert_eq!(TextStyle::TITLE_MEDIUM.line_height, 24.0);
    }

    #[test]
    fn shape_scale_is_monotonic() {
        const { assert!(shape::EXTRA_LARGE > shape::LARGE) };
    }
}
