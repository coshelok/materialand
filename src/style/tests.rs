use crate::graphics::text::FontKind;

use super::*;

#[test]
fn label_and_body_styles_are_distinct_weights() {
    assert_eq!(TextStyle::label_large().kind, FontKind::Medium);
    assert_eq!(TextStyle::body_large().kind, FontKind::Regular);
}

#[test]
fn title_medium_matches_m3_spec() {
    assert_eq!(TextStyle::title_medium().size, 16.0);
    assert_eq!(TextStyle::title_medium().line_height, 24.0);
}

#[test]
fn shape_scale_is_monotonic() {
    assert!(crate::config::design().shape.extra_large > crate::config::design().shape.large);
}
