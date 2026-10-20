//! Apply one uniform transform to each window's content, decorations and popups.
use super::{
    layers::Layers,
    scene::UiElement,
    uploads::{buffer_element, masked_buffer_element, ShellElement},
    window_transform::transform,
};
use crate::{
    desktop::State,
    shell::{CircleMask, DecorParams},
    theme::Theme,
};
use smithay::backend::renderer::{
    element::{surface::WaylandSurfaceRenderElement, AsRenderElements},
    gles::GlesRenderer,
};
smithay::render_elements! { pub WindowElement<=GlesRenderer>; Memory=ShellElement, Wl=WaylandSurfaceRenderElement<GlesRenderer>, }

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
    let top_pair = windows
        .iter()
        .rev()
        .find(|window| {
            state
                .split
                .as_ref()
                .is_some_and(|split| *window == &split.left || *window == &split.right)
        })
        .cloned();
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
        let pose = state
            .split_pose(&window, frame)
            .unwrap_or_else(|| state.shell.window_presentation(&window, frame));
        let group = window_group(
            state, renderer, &window, geometry, pose.alpha, theme, second,
        );
        if group.is_empty() {
            continue;
        }
        state.shell.window_frame_built(&window);
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
        if top_pair.as_ref() == Some(&window) {
            if let Some(r) = state.split_divider() {
                let d = &crate::config::design().split;
                let buffer = state.shell.render_split_handle(&theme);
                let loc = (
                    f64::from(r.x + (r.w - d.handle_width) / 2.0),
                    f64::from(r.y + (r.h - d.handle_height) / 2.0),
                );
                if let Some(element) = buffer_element(renderer, &buffer, loc, None) {
                    scene.single(UiElement::Memory(element));
                }
            }
        }
    }
    state.shell.end_frame();
    opening
}

/// Preserve the same surface/decorations ordering when a window becomes a preview.
pub(super) fn window_group(
    state: &mut State,
    renderer: &mut GlesRenderer,
    window: &smithay::desktop::Window,
    geometry: smithay::utils::Rectangle<i32, smithay::utils::Logical>,
    alpha: f32,
    theme: Theme,
    second: Option<(Theme, Option<CircleMask>, f32)>,
) -> Vec<WindowElement> {
    let meta = state.window_meta(window);
    let loc = geometry.loc;
    let origin = if state.windows.space.element_location(window).is_some() {
        state.window_surface_origin(window)
    } else {
        loc - window.geometry().loc
    };
    let mut group: Vec<WindowElement> =
        window.render_elements(renderer, origin.to_physical(1), 1.0.into(), alpha);
    if group.is_empty() {
        return group;
    }
    let props = state.window_props(window);
    if !props.fullscreen && state.window_server_decor(window) {
        let params = DecorParams {
            meta: &meta,
            focused: state.windows.focused.as_ref() == Some(window),
            maximized: props.maximized,
        };
        let (buffer, loc) =
            state
                .shell
                .render_decoration(&theme, &mut state.fonts, window, geometry, params, None);
        // A group's elements are front-to-back: the frame sits behind the
        // complete surface tree, preserving popup and subsurface ordering.
        let mut decorations = Vec::new();
        if let Some(el) = buffer_element(renderer, &buffer, (loc.x, loc.y), Some(alpha)) {
            decorations.push(WindowElement::Memory(el));
        }
        if let Some((target, mask, fade)) = second {
            let (buffer, loc) = state.shell.render_decoration(
                &target,
                &mut state.fonts,
                window,
                geometry,
                params,
                None,
            );
            if let Some(el) =
                masked_buffer_element(renderer, &buffer, (loc.x, loc.y), Some(alpha * fade), mask)
            {
                decorations.insert(0, WindowElement::Memory(el));
            }
        }
        group.extend(decorations);
    }
    group
}
