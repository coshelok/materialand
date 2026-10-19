//! Cursor surfaces are client-owned, with their own hotspot and buffer scale.
use super::scene::UiElement;
use crate::{desktop::State, theme::Theme};
use smithay::{
    backend::renderer::{
        element::{
            memory::MemoryRenderBufferRenderElement, surface::render_elements_from_surface_tree,
            Kind,
        },
        gles::GlesRenderer,
    },
    input::pointer::{CursorImageStatus, CursorImageSurfaceData},
    wayland::compositor::with_states,
};
pub(super) fn paint(
    state: &mut State,
    renderer: &mut GlesRenderer,
    theme: &Theme,
) -> Vec<UiElement> {
    match state.cursor_status.clone() {
        CursorImageStatus::Hidden => Vec::new(),
        CursorImageStatus::Surface(surface) => {
            let hotspot = with_states(&surface, |states| {
                states
                    .data_map
                    .get::<CursorImageSurfaceData>()
                    .and_then(|s| s.lock().ok())
                    .map(|s| s.hotspot)
                    .unwrap_or_default()
            });
            render_elements_from_surface_tree(
                renderer,
                &surface,
                (state.cursor - hotspot.to_f64())
                    .to_i32_round()
                    .to_physical(1),
                1.0,
                1.0,
                Kind::Cursor,
            )
        }
        CursorImageStatus::Named(icon) => {
            let (buffer, hotspot) = state.shell.render_cursor(theme, icon);
            MemoryRenderBufferRenderElement::from_buffer(
                renderer,
                (state.cursor - hotspot.to_f64()).to_physical(1.0),
                &buffer,
                None,
                None,
                None,
                Kind::Cursor,
            )
            .map(|element| vec![UiElement::Memory(element.into())])
            .unwrap_or_default()
        }
    }
}
