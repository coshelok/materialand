use crate::graphics::text::FontKind;

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
        Self {
            size,
            line_height,
            tracking,
            kind,
        }
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
