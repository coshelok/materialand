//! Typography roles are resolved from the process-wide design snapshot.
use crate::graphics::text::FontKind;
use serde::Deserialize;
#[derive(Clone, Copy, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TextStyle {
    pub size: f32,
    pub line_height: f32,
    pub tracking: f32,
    pub kind: FontKind,
}
impl TextStyle {
    pub fn title_medium() -> Self {
        crate::config::design().typography.title_medium
    }
    pub fn headline_small() -> Self {
        crate::config::design().typography.headline_small
    }
    pub fn title_small() -> Self {
        crate::config::design().typography.title_small
    }
    pub fn label_large() -> Self {
        crate::config::design().typography.label_large
    }
    pub fn label_medium() -> Self {
        crate::config::design().typography.label_medium
    }
    pub fn label_small() -> Self {
        crate::config::design().typography.label_small
    }
    pub fn body_large() -> Self {
        crate::config::design().typography.body_large
    }
    pub fn body_medium() -> Self {
        crate::config::design().typography.body_medium
    }
    pub fn body_small() -> Self {
        crate::config::design().typography.body_small
    }
    pub fn focused_title() -> Self {
        crate::config::design().typography.focused_title
    }
}
