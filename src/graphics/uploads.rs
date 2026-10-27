//! Uploads for cached shell layers.
use super::reveal::{self, RevealElement};
use super::scene::UiElement;
use crate::shell;
smithay::render_elements! { pub ShellElement<=GlesRenderer>;
    Memory=MemoryRenderBufferRenderElement<GlesRenderer>,
    Reveal=RevealElement<MemoryRenderBufferRenderElement<GlesRenderer>>,
}
use smithay::{
    backend::renderer::{
        element::{
            memory::{MemoryRenderBuffer, MemoryRenderBufferRenderElement},
            Element, Kind, RenderElement, UnderlyingStorage,
        },
        gles::GlesRenderer,
    },
    utils::{Point, Rectangle, Transform},
};
pub(super) fn push_buffer(
    elements: &mut Vec<UiElement>,
    renderer: &mut GlesRenderer,
    buffer: &MemoryRenderBuffer,
    loc: (f64, f64),
    alpha: Option<f32>,
) {
    push_masked_buffer(elements, renderer, buffer, loc, alpha, None);
}
pub(super) fn push_masked_buffer(
    elements: &mut Vec<UiElement>,
    renderer: &mut GlesRenderer,
    buffer: &MemoryRenderBuffer,
    loc: (f64, f64),
    alpha: Option<f32>,
    mask: Option<shell::CircleMask>,
) {
    if let Some(el) = masked_buffer_element(renderer, buffer, loc, alpha, mask) {
        elements.push(UiElement::Memory(el));
    }
}
pub(super) fn buffer_element(
    renderer: &mut GlesRenderer,
    buffer: &MemoryRenderBuffer,
    loc: (f64, f64),
    alpha: Option<f32>,
) -> Option<ShellElement> {
    masked_buffer_element(renderer, buffer, loc, alpha, None)
}
pub(super) fn masked_buffer_element(
    renderer: &mut GlesRenderer,
    buffer: &MemoryRenderBuffer,
    loc: (f64, f64),
    alpha: Option<f32>,
    mask: Option<shell::CircleMask>,
) -> Option<ShellElement> {
    let element = MemoryRenderBufferRenderElement::from_buffer(
        renderer,
        Point::from(loc),
        buffer,
        alpha,
        None,
        None,
        Kind::Unspecified,
    )
    .ok()?;
    Some(apply_mask(renderer, element, mask, None))
}
fn apply_mask(
    renderer: &mut GlesRenderer,
    element: MemoryRenderBufferRenderElement<GlesRenderer>,
    mask: Option<shell::CircleMask>,
    rounded: Option<shell::RoundedClip>,
) -> ShellElement {
    if mask.is_some() || rounded.is_some() {
        if let (Some(program), Some(UnderlyingStorage::Memory(memory))) = (
            reveal::program(renderer),
            element.underlying_storage(renderer),
        ) {
            let size = memory.size();
            return ShellElement::Reveal(RevealElement::new(element, program, mask, rounded, size));
        }
    }
    ShellElement::Memory(element)
}

/// Push one overlay layer, letting the GPU scale it when the layer carries a
/// size override (container transforms of the launcher, sheets and menus).
pub(super) fn push_layer(
    elements: &mut Vec<UiElement>,
    renderer: &mut GlesRenderer,
    layer: &shell::LayerFrame,
    alpha: Option<f32>,
    mask: Option<shell::CircleMask>,
) {
    let Ok(native) = MemoryRenderBufferRenderElement::from_buffer(
        renderer,
        layer.loc.to_physical(1.0),
        &layer.buffer,
        alpha,
        None,
        None,
        Kind::Unspecified,
    ) else {
        return;
    };
    let source = layer
        .source
        .map(|r| {
            Rectangle::new(
                (f64::from(r.x), f64::from(r.y)).into(),
                (f64::from(r.w), f64::from(r.h)).into(),
            )
        })
        .unwrap_or_else(|| {
            native
                .src()
                .to_logical(1.0, Transform::Normal, &native.src().size)
        });
    let Ok(el) = MemoryRenderBufferRenderElement::from_buffer(
        renderer,
        layer.loc.to_physical(1.0),
        &layer.buffer,
        alpha,
        Some(source),
        layer.size,
        Kind::Unspecified,
    ) else {
        return;
    };
    elements.push(UiElement::Memory(apply_mask(
        renderer,
        el,
        mask,
        layer.rounded,
    )));
}

/// Push every piece of an overlay frame (scrim, card, press feedback) in order,
/// scaled by `fade` — the light/dark transition opacity.
pub(super) fn push_overlay(
    elements: &mut Vec<UiElement>,
    renderer: &mut GlesRenderer,
    frame: &shell::OverlayFrame,
    fade: f32,
) {
    push_masked_overlay(elements, renderer, frame, fade, None);
}
pub(super) fn push_masked_overlay(
    elements: &mut Vec<UiElement>,
    renderer: &mut GlesRenderer,
    frame: &shell::OverlayFrame,
    fade: f32,
    mask: Option<shell::CircleMask>,
) {
    for (layer, alpha) in &frame.pieces {
        push_layer(elements, renderer, layer, Some(alpha * fade), mask);
    }
}
