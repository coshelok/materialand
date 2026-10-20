//! Presentation-only group transform; protocol coordinates and buffers stay native.
use crate::shell::WindowPose;
use smithay::{
    backend::renderer::element::{
        utils::{Relocate, RelocateRenderElement, RescaleRenderElement},
        Element,
    },
    utils::Point,
};
pub(super) fn transform<E: Element>(
    element: E,
    pose: WindowPose,
) -> RelocateRenderElement<RescaleRenderElement<E>> {
    let anchor = Point::from((pose.source.x.round() as i32, pose.source.y.round() as i32));
    let offset = (
        (pose.target.x - pose.source.x).round() as i32,
        (pose.target.y - pose.source.y).round() as i32,
    );
    RelocateRenderElement::from_element(
        RescaleRenderElement::from_element(
            element,
            anchor,
            (
                pose.scale(),
                f64::from(pose.target.h / pose.source.h.max(1.0)),
            ),
        ),
        offset,
        Relocate::Relative,
    )
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::shell::RectF;
    use smithay::{
        backend::renderer::{
            element::{solid::SolidColorRenderElement, Id, Kind},
            utils::CommitCounter,
            Color32F,
        },
        utils::{Physical, Rectangle},
    };
    #[test]
    fn window_frame_client_subsurface_and_popup_share_one_coordinate_transform() {
        let pose = WindowPose {
            source: RectF {
                x: 300.0,
                y: 200.0,
                w: 1000.0,
                h: 600.0,
            },
            target: RectF {
                x: 80.0,
                y: 500.0,
                w: 200.0,
                h: 120.0,
            },
            alpha: 1.0,
        };
        for (input, expected) in [
            ((300, 200, 1000, 600), (80, 500, 200, 120)),
            ((300, 240, 1000, 560), (80, 508, 200, 112)),
            ((400, 300, 200, 100), (100, 520, 40, 20)),
            ((250, 150, 200, 150), (70, 490, 40, 30)),
        ] {
            let rect = Rectangle::<i32, Physical>::new(
                (input.0, input.1).into(),
                (input.2, input.3).into(),
            );
            let element = SolidColorRenderElement::new(
                Id::new(),
                rect,
                CommitCounter::default(),
                Color32F::new(1.0, 1.0, 1.0, 1.0),
                Kind::Unspecified,
            );
            let output = transform(element, pose).geometry(1.0.into());
            assert_eq!(
                output,
                Rectangle::new(
                    (expected.0, expected.1).into(),
                    (expected.2, expected.3).into()
                )
            );
        }
    }
}
