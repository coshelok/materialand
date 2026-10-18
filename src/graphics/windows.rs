//! Client surfaces keep their protocol geometry throughout animations.
use super::{layers::Layers, scene::UiElement, uploads::push_buffer};
use crate::{
    desktop::State,
    shell::{CircleMask, DecorParams},
    theme::Theme,
};
use smithay::backend::renderer::{element::AsRenderElements, gles::GlesRenderer};
pub(super) fn paint(
    state: &mut State,
    renderer: &mut GlesRenderer,
    scene: &mut Layers<UiElement>,
    theme: Theme,
    second: Option<(Theme, Option<CircleMask>, f32)>,
) {
    state.shell.begin_frame();
    let windows: Vec<_> = state
        .windows
        .space
        .elements_for_output(&state.display.handle)
        .cloned()
        .collect();
    for window in windows {
        if !state.window_has_content(&window) {
            continue;
        }
        let Some(geometry) = state.windows.space.element_geometry(&window) else {
            continue;
        };
        let origin = state.window_surface_origin(&window);
        let alpha = state
            .shell
            .window_anim(&window)
            .map(|(_, alpha)| alpha)
            .unwrap_or(1.0);
        let content: Vec<UiElement> =
            window.render_elements(renderer, origin.to_physical(1), 1.0.into(), alpha);
        if content.is_empty() {
            continue;
        }
        let props = state.window_props(&window);
        if !props.fullscreen && state.window_server_decor(&window) {
            let meta = state.window_meta(&window);
            let params = DecorParams {
                meta: &meta,
                focused: state.windows.focused.as_ref() == Some(&window),
                maximized: props.maximized,
            };
            let mut decorations = Vec::new();
            let (buffer, loc) = state.shell.render_decoration(
                &theme,
                &mut state.fonts,
                &window,
                geometry,
                params,
                None,
            );
            push_buffer(
                &mut decorations,
                renderer,
                &buffer,
                (loc.x, loc.y),
                Some(alpha),
            );
            if let Some((target, mask, fade)) = second {
                let (buffer, loc) = state.shell.render_decoration(
                    &target,
                    &mut state.fonts,
                    &window,
                    geometry,
                    params,
                    mask,
                );
                push_buffer(
                    &mut decorations,
                    renderer,
                    &buffer,
                    (loc.x, loc.y),
                    Some(alpha * fade),
                );
            }
            for element in decorations {
                scene.single(element);
            }
        }
        scene.group(content);
    }
    state.shell.end_frame();
}
