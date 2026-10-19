//! Apply one uniform transform to each window's content, decorations and popups.
use super::{
    layers::Layers, scene::UiElement, uploads::buffer_element, window_transform::transform,
};
use crate::{
    desktop::State,
    shell::{CircleMask, DecorParams},
    theme::Theme,
};
use smithay::backend::renderer::{
    element::{
        memory::MemoryRenderBufferRenderElement, surface::WaylandSurfaceRenderElement,
        AsRenderElements,
    },
    gles::GlesRenderer,
};
smithay::render_elements! { pub WindowElement<=GlesRenderer>; Memory=MemoryRenderBufferRenderElement<GlesRenderer>, Wl=WaylandSurfaceRenderElement<GlesRenderer>, }

pub(super) fn paint(
    state: &mut State,
    renderer: &mut GlesRenderer,
    scene: &mut Layers<UiElement>,
    theme: Theme,
    second: Option<(Theme, Option<CircleMask>, f32)>,
) -> Vec<Vec<UiElement>> {
    let mut opening = Vec::new();
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
        let Some(frame) = state.window_frame_rect(&window) else {
            continue;
        };
        let meta = state.window_meta(&window);
        state.shell.bind_window_launch_origin(&window, &meta.app_id);
        let pose = state.shell.window_presentation(&window, frame);
        let origin = state.window_surface_origin(&window);
        let mut group: Vec<WindowElement> =
            window.render_elements(renderer, origin.to_physical(1), 1.0.into(), pose.alpha);
        if group.is_empty() {
            continue;
        }
        state.shell.window_frame_built(&window);
        let props = state.window_props(&window);
        if !props.fullscreen && state.window_server_decor(&window) {
            let params = DecorParams {
                meta: &meta,
                focused: state.windows.focused.as_ref() == Some(&window),
                maximized: props.maximized,
            };
            let (buffer, loc) = state.shell.render_decoration(
                &theme,
                &mut state.fonts,
                &window,
                geometry,
                params,
                None,
            );
            // A group's elements are front-to-back: the frame sits behind the
            // complete surface tree, preserving popup and subsurface ordering.
            let mut decorations = Vec::new();
            if let Some(el) = buffer_element(renderer, &buffer, (loc.x, loc.y), Some(pose.alpha)) {
                decorations.push(WindowElement::Memory(el));
            }
            if let Some((target, mask, fade)) = second {
                let (buffer, loc) = state.shell.render_decoration(
                    &target,
                    &mut state.fonts,
                    &window,
                    geometry,
                    params,
                    mask,
                );
                if let Some(el) =
                    buffer_element(renderer, &buffer, (loc.x, loc.y), Some(pose.alpha * fade))
                {
                    decorations.insert(0, WindowElement::Memory(el));
                }
            }
            group.extend(decorations);
        }
        let transformed = group
            .into_iter()
            .map(|el| UiElement::Window(transform(el, pose)))
            .collect();
        if state.shell.is_opening(&window)
            && state.windows.focused.as_ref() == Some(&window)
            && !state.shell.quick_settings_open
        {
            opening.push(transformed);
        } else {
            scene.group(transformed);
        }
    }
    state.shell.end_frame();
    opening
}
