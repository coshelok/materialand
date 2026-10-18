use crate::graphics::text::FontKind;

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
