//! Recent-app previews use the clients' existing buffers, including minimized ones.
use super::{
    scene::UiElement,
    uploads::{masked_buffer_element, push_buffer, push_masked_buffer},
    window_transform::transform,
    windows::{window_group, WindowElement},
};
use crate::{
    desktop::State,
    shell::{RectF, WindowPose},
};
use smithay::{
    backend::renderer::gles::GlesRenderer,
    utils::{Physical, Rectangle, Size},
};
pub(super) fn paint(
    state: &mut State,
    renderer: &mut GlesRenderer,
    size: Size<i32, Physical>,
    theme: crate::theme::Theme,
    second: Option<(crate::theme::Theme, Option<crate::shell::CircleMask>, f32)>,
) -> Vec<UiElement> {
    let mut back_to_front = Vec::new();
    let t = state.overview.progress();
    let motion = state.overview.motion_progress();
    state.shell.begin_frame();
    let w = size.w as f32;
    let h = size.h as f32;
    let items = state.overview.items.clone();
    let card_size = state.overview.card_size(w, h);
    let design = &crate::config::design().overview;
    let rounded_shader = super::rounded::program(renderer);
    let layer = crate::shell::LayerFrame {
        source: None,
        buffer: state.shell.scrim_buffer(&theme),
        loc: (0.0, 0.0).into(),
        size: Some((size.w, size.h).into()),
    };
    super::uploads::push_layer(
        &mut back_to_front,
        renderer,
        &layer,
        Some(t * crate::config::design().overview.scrim_alpha),
        None,
    );
    state
        .shell
        .retain_overview_cards(&items.iter().map(|i| i.window.clone()).collect::<Vec<_>>());
    let label = state.overview.notice.clone().unwrap_or_else(|| {
        if state.overview.pending_split.is_some() {
            "Choose a second app".into()
        } else if !state.overview.has_cards() {
            "No recent apps".into()
        } else {
            "Recent apps".into()
        }
    });
    let header = state
        .shell
        .overview_header(&theme, &mut state.fonts, w, &label);
    push_buffer(&mut back_to_front, renderer, &header, (0.0, 0.0), Some(t));
    if let Some((next, mask, alpha)) = second {
        let header = state
            .shell
            .overview_header(&next, &mut state.fonts, w, &label);
        push_masked_buffer(
            &mut back_to_front,
            renderer,
            &header,
            (0.0, 0.0),
            Some(t * alpha),
            mask,
        );
    }

    // Keep desktop stacking during the morph; MRU order determines card positions,
    // not which overlapping full-size window suddenly appears in front.
    let stacking: Vec<_> = state.windows.space.elements().cloned().collect();
    let mut ordered: Vec<_> = items.iter().enumerate().collect();
    ordered.sort_by_key(|(_, item)| {
        stacking
            .iter()
            .position(|w| w == &item.window)
            .map(|i| i + 1)
            .unwrap_or(0)
    });
    for (i, item) in ordered {
        let offset = state.overview.card_offset(i);
        let mut card = state.overview.card(w, h, i);
        card.y -= offset;
        let meta = state.window_meta(&item.window);
        let selected = state.overview.pending_split.as_ref() == Some(&item.window);
        let buffer = state.shell.overview_card(
            &item.window,
            &theme,
            &mut state.fonts,
            &meta,
            card_size,
            selected,
            i == state.overview.selected,
        );
        let geom = item.window.geometry();
        let loc = state
            .windows
            .space
            .element_location(&item.window)
            .or(state.window_props(&item.window).minimized_location)
            .unwrap_or_default();
        let source = state.overview_frame(&item.window);
        let destination = state
            .split_pose(&item.window, source)
            .unwrap_or_else(|| state.shell.window_presentation(&item.window, source));
        let resting = if state.overview.open {
            item.origin
        } else {
            destination
        };
        // Freeze preview aspect at entry: configure/commit during a split must not
        // change the card's geometry midway through the return animation.
        let reference = item.origin.source;
        let scale = (card.w / reference.w).min(card.h / reference.h);
        let preview = RectF {
            x: card.x + (card.w - reference.w * scale) / 2.0,
            y: card.y + (card.h - reference.h * scale) / 2.0,
            w: reference.w * scale,
            h: reference.h * scale,
        };
        let minimized = state.window_props(&item.window).minimized;
        let from = if minimized { preview } else { resting.target };
        let mut target = interpolate_frame(from, preview, motion);
        target.y += offset;
        let depth_scale = card.w / card_size.0.max(1.0);
        let full_card = RectF {
            x: card.x,
            y: card.y - design.app_header_height * depth_scale,
            w: card.w,
            h: card.h + (design.app_header_height + design.footer_height) * depth_scale,
        };
        let chrome_source = RectF {
            x: full_card.x,
            y: full_card.y,
            w: card_size.0,
            h: card_size.1 + design.app_header_height + design.footer_height,
        };
        let mut chrome_target = interpolate_frame(from, full_card, motion);
        chrome_target.y += offset;
        for (card_theme, mask, alpha) in std::iter::once((theme, None, 1.0)).chain(second) {
            let buffer = if card_theme.cache_key() == theme.cache_key() {
                buffer.clone()
            } else {
                state.shell.overview_card(
                    &item.window,
                    &card_theme,
                    &mut state.fonts,
                    &meta,
                    card_size,
                    selected,
                    i == state.overview.selected,
                )
            };
            if let Some(element) = masked_buffer_element(
                renderer,
                &buffer,
                (chrome_source.x as f64, chrome_source.y as f64),
                Some(t * alpha),
                mask,
            ) {
                back_to_front.push(UiElement::Window(transform(
                    WindowElement::Memory(element),
                    WindowPose {
                        source: chrome_source,
                        target: chrome_target,
                        alpha: t * alpha,
                    },
                )));
            }
        }
        let alpha = if minimized {
            t
        } else {
            resting.alpha + (1.0 - resting.alpha) * t
        };
        let pose = WindowPose {
            source,
            target,
            alpha: if state.overview.open {
                alpha * state.shell.window_presentation(&item.window, source).alpha
            } else {
                alpha
            },
        };
        let group = window_group(
            state,
            renderer,
            &item.window,
            Rectangle::new(loc, geom.size),
            pose.alpha,
            theme,
            None,
        );
        for el in group.into_iter().rev() {
            let transformed = transform(el, pose);
            if t > 0.001 {
                back_to_front.push(UiElement::Rounded(super::rounded::RoundedElement::new(
                    transformed,
                    rounded_shader.clone(),
                    target,
                    design.card_radius * t * depth_scale,
                    size,
                )));
            } else {
                back_to_front.push(UiElement::Window(transformed));
            }
        }
    }
    state.shell.end_frame();
    back_to_front
}

/// Geometry can overshoot; opacity is independently clamped by Overview::progress.
fn interpolate_frame(from: RectF, to: RectF, t: f32) -> RectF {
    let lerp = |a: f32, b: f32| a + (b - a) * t;
    RectF {
        x: lerp(from.x, to.x),
        y: lerp(from.y, to.y),
        w: lerp(from.w, to.w).max(1.0),
        h: lerp(from.h, to.h).max(1.0),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn morph_keeps_native_geometry_aspect_ratio_and_the_spring_rebound() {
        let source = RectF {
            x: 80.0,
            y: 120.0,
            w: 1200.0,
            h: 800.0,
        };
        let preview = RectF {
            x: 500.0,
            y: 300.0,
            w: 420.0,
            h: 280.0,
        };
        let start = interpolate_frame(source, preview, 0.0);
        assert_eq!(
            (start.x, start.y, start.w, start.h),
            (80.0, 120.0, 1200.0, 800.0)
        );
        for t in [-0.03, 0.0, 0.2, 0.8, 1.0, 1.03] {
            let frame = interpolate_frame(source, preview, t);
            assert!((frame.w / frame.h - 1.5).abs() < 0.001);
        }
        let overshoot = interpolate_frame(source, preview, 1.03);
        assert!(overshoot.w < preview.w && overshoot.x > preview.x);
        let end = interpolate_frame(source, preview, 1.0);
        assert_eq!((end.x, end.y, end.w, end.h), (500.0, 300.0, 420.0, 280.0));
    }
}
