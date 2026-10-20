//! Renderer-independent scene assembly. The returned list is front-to-back.
use super::{
    chrome,
    layers::Layers,
    uploads::{push_buffer, push_masked_buffer},
    windows,
};
use crate::{desktop::State, shell::CircleMask};
use smithay::backend::renderer::element::utils::{RelocateRenderElement, RescaleRenderElement};
use smithay::{
    backend::renderer::{element::surface::WaylandSurfaceRenderElement, gles::GlesRenderer},
    utils::{Physical, Size},
};
smithay::render_elements! { pub UiElement<=GlesRenderer>; Memory=super::uploads::ShellElement, Wl=WaylandSurfaceRenderElement<GlesRenderer>, Window=RelocateRenderElement<RescaleRenderElement<windows::WindowElement>>, Rounded=super::rounded::RoundedElement<RelocateRenderElement<RescaleRenderElement<windows::WindowElement>>>, }
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
        let mask = super::reveal::program(renderer)
            .is_some()
            .then_some(CircleMask {
                cx: origin.x as f32,
                cy: origin.y as f32,
                radius: t.max(0.0) * dx.hypot(dy) as f32,
            });
        second = Some((target, mask, if mask.is_some() { 1.0 } else { t }));
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
            .render_wallpaper(&next, size.w as u32, size.h as u32, None);
        push_masked_buffer(
            &mut background,
            renderer,
            &buffer,
            (loc.x, loc.y),
            Some(alpha),
            mask,
        );
    }
    for element in background {
        scene.single(element);
    }
    let (buffer, loc) =
        state
            .shell
            .render_home(&theme, &mut state.fonts, size.w as f32, size.h as f32, None);
    let mut home = Vec::new();
    let home_alpha = if state.overview.visible() {
        1.0 - state.overview.progress()
    } else {
        1.0
    };
    push_buffer(
        &mut home,
        renderer,
        &buffer,
        (loc.x, loc.y),
        Some(home_alpha),
    );
    if let Some((next, mask, alpha)) = second {
        let (buffer, loc) =
            state
                .shell
                .render_home(&next, &mut state.fonts, size.w as f32, size.h as f32, None);
        push_masked_buffer(
            &mut home,
            renderer,
            &buffer,
            (loc.x, loc.y),
            Some(alpha * home_alpha),
            mask,
        );
    }
    for element in home {
        scene.single(element);
    }
    let opening_windows = if state.overview.visible() {
        for element in super::overview::paint(state, renderer, size) {
            scene.single(element);
        }
        Vec::new()
    } else {
        windows::paint(state, renderer, &mut scene, theme, second)
    };
    for element in chrome::paint(state, renderer, size, theme, second, opening_windows) {
        scene.single(element);
    }
    if let Some(drag) = state.shortcut_drag.as_ref().filter(|drag| drag.active) {
        let (buffer, loc) =
            state
                .shell
                .render_shortcut_drag(&target, &mut state.fonts, &drag.app, state.cursor);
        let mut elements = Vec::new();
        push_buffer(&mut elements, renderer, &buffer, (loc.x, loc.y), Some(0.94));
        for element in elements {
            scene.single(element);
        }
    }
    scene.group(super::cursor::paint(state, renderer, &theme));
    scene.finish()
}
pub use super::presentation::send_frames_all;
