//! Uploads for cached shell layers.
use super::scene::UiElement;
use crate::shell;
use smithay::{
    backend::renderer::{
        element::{
            memory::{MemoryRenderBuffer, MemoryRenderBufferRenderElement},
            Kind,
        },
        gles::GlesRenderer,
    },
    utils::Point,
};
pub(super) fn push_buffer(
    elements: &mut Vec<UiElement>,
    renderer: &mut GlesRenderer,
    buffer: &MemoryRenderBuffer,
    loc: (f64, f64),
    alpha: Option<f32>,
) {
    if let Some(el) = buffer_element(renderer, buffer, loc, alpha) {
        elements.push(UiElement::Memory(el));
    }
}
pub(super) fn buffer_element(
    renderer: &mut GlesRenderer,
    buffer: &MemoryRenderBuffer,
    loc: (f64, f64),
    alpha: Option<f32>,
) -> Option<MemoryRenderBufferRenderElement<GlesRenderer>> {
    MemoryRenderBufferRenderElement::from_buffer(
        renderer,
        Point::from(loc),
        buffer,
        alpha,
        None,
        None,
        Kind::Unspecified,
    )
    .ok()
}

/// Push one overlay layer, letting the GPU scale it when the layer carries a
/// size override (container transforms of the launcher, sheets and menus).
pub(super) fn push_layer(
    elements: &mut Vec<UiElement>,
    renderer: &mut GlesRenderer,
    layer: &shell::LayerFrame,
    alpha: Option<f32>,
) {
    if let Ok(el) = MemoryRenderBufferRenderElement::from_buffer(
        renderer,
        layer.loc.to_physical(1.0),
        &layer.buffer,
        alpha,
        None,
        layer.size,
        Kind::Unspecified,
    ) {
        elements.push(UiElement::Memory(el));
    }
}

/// Push every piece of an overlay frame (scrim, card, press feedback) in order,
/// scaled by `fade` — the light/dark transition opacity.
pub(super) fn push_overlay(
    elements: &mut Vec<UiElement>,
    renderer: &mut GlesRenderer,
    frame: &shell::OverlayFrame,
    fade: f32,
) {
    for (layer, alpha) in &frame.pieces {
        push_layer(elements, renderer, layer, Some(alpha * fade));
    }
}
