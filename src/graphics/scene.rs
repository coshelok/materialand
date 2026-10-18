//! Renderer-independent scene assembly. The returned list is front-to-back.
use super::{chrome, layers::Layers, uploads::push_buffer, windows};
use crate::{desktop::State, shell::CircleMask};
use smithay::backend::renderer::element::memory::MemoryRenderBufferRenderElement;
use smithay::{
    backend::renderer::{
        element::{solid::SolidColorRenderElement, surface::WaylandSurfaceRenderElement, Id, Kind},
        gles::GlesRenderer,
        utils::CommitCounter,
        Color32F,
    },
    utils::{Physical, Rectangle, Size},
};
smithay::render_elements! { pub UiElement<=GlesRenderer>; Memory=MemoryRenderBufferRenderElement<GlesRenderer>, Wl=WaylandSurfaceRenderElement<GlesRenderer>, Solid=SolidColorRenderElement, }
pub fn build_ui(
    state: &mut State,
    renderer: &mut GlesRenderer,
    size: Size<i32, Physical>,
) -> Vec<UiElement> {
    let target = state.theme;
    let mut second = None;
    let theme = if let Some((from, origin, t)) = state.shell.reveal_frame() {
        let dx = origin.x.max(f64::from(size.w) - origin.x).max(0.0);
        let dy = origin.y.max(f64::from(size.h) - origin.y).max(0.0);
        second = Some((
            target,
            Some(CircleMask {
                cx: origin.x as f32,
                cy: origin.y as f32,
                radius: t.max(0.0) * dx.hypot(dy) as f32,
            }),
            1.0,
        ));
        from
    } else if let Some((from, alpha)) = state.shell.theme_fade() {
        second = Some((target, None, alpha));
        from
    } else {
        target
    };
    let mut scene = Layers::new();
    let mut background = Vec::new();
    let (buffer, loc) = state
        .shell
        .render_wallpaper(&theme, size.w as u32, size.h as u32, None);
    push_buffer(&mut background, renderer, &buffer, (loc.x, loc.y), None);
    if let Some((next, mask, alpha)) = second {
        let (buffer, loc) = state
            .shell
            .render_wallpaper(&next, size.w as u32, size.h as u32, mask);
        push_buffer(
            &mut background,
            renderer,
            &buffer,
            (loc.x, loc.y),
            Some(alpha),
        );
    }
    for element in background {
        scene.single(element);
    }
    windows::paint(state, renderer, &mut scene, theme, second);
    for element in chrome::paint(state, renderer, size, theme, second) {
        scene.single(element);
    }
    scene.group(super::cursor::paint(state, renderer, &theme));
    let full = Rectangle::from_size(size);
    for color in [
        Color32F::new(0.0, 0.0, 0.0, (1.0 - state.brightness).clamp(0.0, 0.8)),
        Color32F::new(
            1.0,
            0.52,
            0.20,
            0.14 * state.shell.night_light_progress().clamp(0.0, 1.0),
        ),
    ] {
        if color.a() > 0.001 {
            scene.single(UiElement::Solid(SolidColorRenderElement::new(
                Id::new(),
                full,
                CommitCounter::default(),
                color,
                Kind::Unspecified,
            )));
        }
    }
    scene.finish()
}
pub use super::presentation::send_frames_all;
