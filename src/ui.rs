//! Backend-agnostic frame building.
//!
//! Both the nested (winit) and the KMS (DRM) backends feed the same element
//! list built here, so the shell looks and behaves identically no matter how
//! the compositor is started. The KMS path passes the list straight to the DRM
//! compositor, the nested path draws it into the winit framebuffer.

use std::time::Instant;

use smithay::{
    backend::renderer::{
        element::{
            memory::{MemoryRenderBuffer, MemoryRenderBufferRenderElement},
            solid::SolidColorRenderElement,
            surface::WaylandSurfaceRenderElement,
            AsRenderElements, Id, Kind,
        },
        gles::GlesRenderer,
        utils::CommitCounter,
        Color32F,
    },
    reexports::wayland_server::protocol::wl_surface::WlSurface,
    utils::{Logical, Physical, Point, Rectangle, Size},
    wayland::seat::WaylandFocus,
};

use crate::{
    clock::Clock,
    input,
    shell::{self, CircleMask},
    state::{Interaction, State},
    theme::Theme,
};

smithay::render_elements! {
    pub UiElement<=GlesRenderer>;
    Memory=MemoryRenderBufferRenderElement<GlesRenderer>,
    Wl=WaylandSurfaceRenderElement<GlesRenderer>,
    Solid=SolidColorRenderElement,
}

/// Everything the shell paints for one frame, in front-to-back order.
///
/// `draw_render_elements` (nested) and the DRM compositor expect front-to-back
/// order; the list is assembled back-to-front and flipped at the end.
pub fn build_ui(
    state: &mut State,
    renderer: &mut GlesRenderer,
    size: Size<i32, Physical>,
) -> Vec<UiElement> {
    let win_w = size.w as f32;
    let win_h = size.h as f32;

    // The scheme painted this frame. A change animates over the *previous*
    // scheme: a palette switch fades the new one in with a GPU alpha blend,
    // a light/dark switch reveals it through a growing circle. Either way the
    // two endpoint schemes are constants, so every theme-keyed cache stays
    // valid for the whole transition and no frame re-rasterises anything.
    let target_theme = state.theme;
    let mut second: Option<(Theme, Option<CircleMask>, f32)> = None;
    let theme = if let Some((from, origin, t)) = state.shell.reveal_frame() {
        let radius = t.max(0.0) * max_reveal_radius(origin, size);
        second = Some((
            target_theme,
            Some(CircleMask {
                cx: origin.x as f32,
                cy: origin.y as f32,
                radius,
            }),
            1.0,
        ));
        from
    } else if let Some((from, alpha)) = state.shell.theme_fade() {
        second = Some((target_theme, None, alpha));
        from
    } else {
        target_theme
    };

    let mut elements: Vec<UiElement> = Vec::new();

    // Wallpaper.
    let (wallpaper, wloc) = state
        .shell
        .render_wallpaper(&theme, size.w as u32, size.h as u32, None);
    push_buffer(&mut elements, renderer, &wallpaper, (wloc.x, wloc.y), None);
    if let Some((target, mask, alpha)) = second {
        let (wallpaper, wloc) =
            state
                .shell
                .render_wallpaper(&target, size.w as u32, size.h as u32, mask);
        push_buffer(&mut elements, renderer, &wallpaper, (wloc.x, wloc.y), Some(alpha));
    }

    // Windows and their decorations, back to front.
    state.shell.begin_frame();
    let windows: Vec<_> = state.space.elements_for_output(&state.output).cloned().collect();
    for window in windows {
        let geom = state
            .space
            .element_geometry(&window)
            .unwrap_or_else(|| window.geometry());
        let meta = state.window_meta(&window);
        let focused = state.focused.as_ref() == Some(&window);
        let props = state.window_props(&window);

        // Window open/close plays as a scale about the window centre plus a
        // fade. The decoration and the client content share the transform so
        // the window moves as one object.
        let (wscale, walpha) = state
            .shell
            .window_anim(&window)
            .unwrap_or((1.0, 1.0));
        let center = (
            geom.loc.x as f64 + geom.size.w as f64 / 2.0,
            geom.loc.y as f64 + geom.size.h as f64 / 2.0,
        );

        // `Window::render_elements` draws the surface tree from its origin,
        // while the space stores the position of the window *geometry* — the
        // rect our decoration frames. CSD clients (GTK) inset that rect inside
        // their buffer, so the surface starts `geometry().loc` above-left.
        let origin = state.window_surface_origin(&window).to_physical(1);
        let scaled_loc = Point::from((
            (center.0 + (origin.x as f64 - center.0) * wscale).round() as i32,
            (center.1 + (origin.y as f64 - center.1) * wscale).round() as i32,
        ));
        // Window rendering includes all tracked XDG popups (menus, combo boxes,
        // tooltips) and their surface trees in the correct relative position.
        let content: Vec<UiElement> = window.render_elements::<UiElement>(
            renderer,
            scaled_loc,
            smithay::utils::Scale::from(
                state.output.current_scale().fractional_scale() * wscale,
            ),
            walpha,
        );
        // Server-side decorations are drawn only for clients that actually
        // handed them over; CSD clients keep their own title bar.
        let server_decor = state.window_server_decor(&window);

        // A toplevel lives in the space from the moment it is created, but the
        // client may not have attached a buffer yet (Electron apps open helper
        // toplevels that never draw anything). Painting the decoration alone
        // would leave an empty grey window behind the app, so a window without
        // content is skipped entirely until its first buffer arrives.
        if content.is_empty() {
            continue;
        }

        // The frame is painted *behind* the client content: it is opaque and
        // covers the whole geometry, so pushing it after the surface would hide
        // the window entirely. Fullscreen windows carry no frame at all, and
        // CSD clients draw their own (see `window_server_decor`).
        if !props.fullscreen && server_decor {
            let layout = shell::decor_layout(geom);
            let deco_size = (
                layout.frame.w as f64 + 2.0 * shell::SHADOW_PAD as f64,
                layout.frame.h as f64 + 2.0 * shell::SHADOW_PAD as f64,
            );
            let params = shell::DecorParams {
                meta: &meta,
                focused,
                maximized: props.maximized,
            };
            let (buffer, loc) =
                state
                    .shell
                    .render_decoration(&theme, &mut state.fonts, &window, geom, params, None);
            push_decor(
                &mut elements, renderer, &buffer, loc, center, deco_size, wscale, walpha,
            );
            if let Some((target, mask, alpha)) = second {
                let (buffer, loc) = state.shell.render_decoration(
                    &target,
                    &mut state.fonts,
                    &window,
                    geom,
                    params,
                    mask,
                );
                push_decor(
                    &mut elements,
                    renderer,
                    &buffer,
                    loc,
                    center,
                    deco_size,
                    wscale,
                    walpha * alpha,
                );
            }
        }

        elements.extend(content);
    }
    state.shell.end_frame();

    // Fullscreen applications own the output; the desktop chrome is hidden.
    let fullscreen = state.has_fullscreen_window();

    // Dock.
    if !fullscreen {
        let items = input::dock_items(state);
        let focused = state.focused.clone();
        let (dock, dock_loc) = state.shell.render_dock(
            &theme,
            &mut state.fonts,
            win_w,
            win_h,
            &items,
            focused.as_ref(),
            None,
        );
        push_buffer(&mut elements, renderer, &dock, (dock_loc.x, dock_loc.y), None);
        if let Some((target, mask, alpha)) = second {
            let (dock, dock_loc) = state.shell.render_dock(
                &target,
                &mut state.fonts,
                win_w,
                win_h,
                &items,
                focused.as_ref(),
                mask,
            );
            push_buffer(
                &mut elements,
                renderer,
                &dock,
                (dock_loc.x, dock_loc.y),
                Some(alpha),
            );
        }

        // Top bar.
        let title = focused
            .as_ref()
            .map(|w| state.window_meta(w).title)
            .unwrap_or_else(|| "Materialand".to_string());
        let (topbar, topbar_loc) =
            state
                .shell
                .render_topbar(&theme, &mut state.fonts, win_w, &title, None);
        push_buffer(
            &mut elements,
            renderer,
            &topbar,
            (topbar_loc.x, topbar_loc.y),
            None,
        );
        if let Some((target, mask, alpha)) = second {
            let (topbar, topbar_loc) = state
                .shell
                .render_topbar(&target, &mut state.fonts, win_w, &title, mask);
            push_buffer(
                &mut elements,
                renderer,
                &topbar,
                (topbar_loc.x, topbar_loc.y),
                Some(alpha),
            );
        }
    }

    // Compact quick-settings sheet, anchored below the top bar.
    let quick_settings_p = state.shell.quick_settings_progress();
    if !fullscreen && (state.shell.quick_settings_open || quick_settings_p > 0.001) {
        let variant = state.theme_spec.variant;
        let frame = state.shell.render_quick_settings(
            &theme,
            &mut state.fonts,
            win_w,
            win_h,
            quick_settings_p,
            state.brightness,
            variant,
            None,
        );
        push_overlay(&mut elements, renderer, &frame, 1.0);
        if let Some((target, mask, alpha)) = second {
            let frame = state.shell.render_quick_settings(
                &target,
                &mut state.fonts,
                win_w,
                win_h,
                quick_settings_p,
                state.brightness,
                variant,
                mask,
            );
            push_overlay(&mut elements, renderer, &frame, alpha);
        }
    }

    // Launcher overlay. Grows out of the control that opened it; skipped once
    // fully closed.
    let launcher_p = state.shell.launcher_progress();
    if state.shell.launcher_open || launcher_p > 0.001 {
        let frame = state.shell.render_launcher(
            &theme,
            &mut state.fonts,
            win_w,
            win_h,
            launcher_p,
            None,
        );
        push_overlay(&mut elements, renderer, &frame, 1.0);
        if let Some((target, mask, alpha)) = second {
            let frame = state.shell.render_launcher(
                &target,
                &mut state.fonts,
                win_w,
                win_h,
                launcher_p,
                mask,
            );
            push_overlay(&mut elements, renderer, &frame, alpha);
        }
    }

    // Brightness popup, drawn above every other surface.
    let brightness_p = state.shell.brightness_progress();
    state
        .shell
        .set_slider_active(matches!(state.interaction, Interaction::Brightness));
    if !fullscreen && (state.shell.brightness_open || brightness_p > 0.001) {
        let frame = state.shell.render_brightness(
            &theme,
            &mut state.fonts,
            win_w,
            state.brightness,
            brightness_p,
            None,
        );
        push_overlay(&mut elements, renderer, &frame, 1.0);
        if let Some((target, mask, alpha)) = second {
            let frame = state.shell.render_brightness(
                &target,
                &mut state.fonts,
                win_w,
                state.brightness,
                brightness_p,
                mask,
            );
            push_overlay(&mut elements, renderer, &frame, alpha);
        }
    }

    // Session menu, drawn above every other surface.
    let power_p = state.shell.power_progress();
    if !fullscreen && (state.shell.power_open || power_p > 0.001) {
        let frame = state.shell.render_power_menu(
            &theme,
            &mut state.fonts,
            win_w,
            power_p,
            None,
        );
        push_overlay(&mut elements, renderer, &frame, 1.0);
        if let Some((target, mask, alpha)) = second {
            let frame = state.shell.render_power_menu(
                &target,
                &mut state.fonts,
                win_w,
                power_p,
                mask,
            );
            push_overlay(&mut elements, renderer, &frame, alpha);
        }
    }

    // Software pointer, always on top.
    let cursor = state.shell.render_cursor(&theme);
    if let Ok(el) = MemoryRenderBufferRenderElement::from_buffer(
        renderer,
        Point::from((state.cursor.x, state.cursor.y)),
        &cursor,
        None,
        None,
        None,
        Kind::Unspecified,
    ) {
        elements.push(UiElement::Memory(el));
    }

    // Compositor-side output overlays: the brightness dim and the night light.
    // They are elements (instead of `Frame::draw_solid`) so the DRM compositor
    // sees them like any other layer; both cover the whole output and sit above
    // everything else, exactly where they used to be drawn.
    let full = Rectangle::from_size(size);
    if state.brightness < 1.0 {
        let a = (1.0 - state.brightness).clamp(0.0, 1.0);
        elements.push(UiElement::Solid(SolidColorRenderElement::new(
            Id::new(),
            full,
            CommitCounter::default(),
            Color32F::new(0.0, 0.0, 0.0, a),
            Kind::Unspecified,
        )));
    }
    let night_light_alpha = 0.14 * state.shell.night_light_progress().clamp(0.0, 1.0);
    if night_light_alpha > 0.001 {
        elements.push(UiElement::Solid(SolidColorRenderElement::new(
            Id::new(),
            full,
            CommitCounter::default(),
            Color32F::new(1.0, 0.52, 0.20, night_light_alpha),
            Kind::Unspecified,
        )));
    }

    // `draw_render_elements` expects a front-to-back order (it reverses the
    // list internally and culls elements occluded by opaque ones); we built the
    // list back-to-front, so flip it.
    elements.reverse();
    elements
}

/// Frame callbacks toward every mapped client surface.
pub fn send_frames_all(state: &State) {
    let time = state.start_time.elapsed().as_millis() as u32;
    for window in state.space.elements() {
        if let Some(surface) = window.wl_surface() {
            send_frames(&surface, time);
            for (popup, _) in smithay::desktop::PopupManager::popups_for_surface(&surface) {
                send_frames(popup.wl_surface(), time);
            }
        }
    }
}

/// Longest time-step the shell animations may advance in a single tick. An
/// idle compositor wakes about once per second (the KMS loop's idle tick),
/// and letting a pause that long through would fast-forward every running
/// spring and tween (the palette reveal included) straight to its end pose,
/// so animation resumes smoothly instead.
const MAX_TICK: f32 = 0.1;

/// Drives the parts of the shell that depend on wall-clock time: the top-bar
/// clock and the animation springs. Shared by the nested and the KMS loops.
pub struct Ticker {
    clock: Clock,
    last_clock: Instant,
    last_frame: Instant,
}

impl Ticker {
    pub fn new(state: &mut State) -> Self {
        let mut clock = Clock::new();
        state.shell.clock = clock.text();
        Self {
            clock,
            last_clock: Instant::now(),
            last_frame: Instant::now(),
        }
    }

    /// Advance the clock, shell animations and pending window closes.
    /// Returns `true` while something is still animating, i.e. the caller
    /// should keep scheduling frames.
    pub fn tick(&mut self, state: &mut State) -> bool {
        let mut dirty = false;

        // Update the clock once per second, but only mark the frame dirty when
        // the displayed minute actually changes: an idle compositor must not
        // repaint once per second just to redraw the same "HH:MM".
        if self.last_clock.elapsed().as_secs() >= 1 {
            self.last_clock = Instant::now();
            let text = self.clock.text();
            if text != state.shell.clock {
                state.shell.clock = text;
                dirty = true;
            }
        }

        // Advance shell animations; keep redrawing while any is in flight.
        // The step is clamped: ticks can span nearly a second when the
        // compositor has been idle, and such a leap would complete animations
        // that just started in a single frame (see `MAX_TICK`).
        let now = Instant::now();
        let dt = (now - self.last_frame).as_secs_f32().min(MAX_TICK);
        self.last_frame = now;
        if state.shell.tick(dt) {
            dirty = true;
        }
        // Windows whose close animation finished get their `send_close` now.
        state.finish_closing();

        dirty
    }
}

/// Push a cached chrome buffer as a render element.
fn push_buffer(
    elements: &mut Vec<UiElement>,
    renderer: &mut GlesRenderer,
    buffer: &MemoryRenderBuffer,
    loc: (f64, f64),
    alpha: Option<f32>,
) {
    if let Ok(el) = MemoryRenderBufferRenderElement::from_buffer(
        renderer,
        Point::from(loc),
        buffer,
        alpha,
        None,
        None,
        Kind::Unspecified,
    ) {
        elements.push(UiElement::Memory(el));
    }
}

/// Push one overlay layer, letting the GPU scale it when the layer carries a
/// size override (container transforms of the launcher, sheets and menus).
fn push_layer(
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
fn push_overlay(
    elements: &mut Vec<UiElement>,
    renderer: &mut GlesRenderer,
    frame: &shell::OverlayFrame,
    fade: f32,
) {
    for (layer, alpha) in &frame.pieces {
        push_layer(elements, renderer, layer, Some(alpha * fade));
    }
}

/// Push a decoration buffer transformed for the window's open/close animation:
/// scaled about the window centre and resized numerically so neighbouring
/// pixels of the frame stay aligned with the scaled client content.
#[allow(clippy::too_many_arguments)]
fn push_decor(
    elements: &mut Vec<UiElement>,
    renderer: &mut GlesRenderer,
    buffer: &MemoryRenderBuffer,
    loc: Point<f64, Logical>,
    center: (f64, f64),
    size: (f64, f64),
    scale: f64,
    alpha: f32,
) {
    if scale <= 0.01 {
        return;
    }
    let at_rest = (scale - 1.0).abs() < 1e-3 && alpha >= 0.999;
    if at_rest {
        // Fast path identical to a plain buffer element.
        if let Ok(el) = MemoryRenderBufferRenderElement::from_buffer(
            renderer,
            loc.to_physical(1.0),
            buffer,
            None,
            None,
            None,
            Kind::Unspecified,
        ) {
            elements.push(UiElement::Memory(el));
        }
        return;
    }
    let loc = Point::from((
        center.0 + (loc.x - center.0) * scale,
        center.1 + (loc.y - center.1) * scale,
    ));
    let physical = Some(Size::from((
        (size.0 * scale).round() as i32,
        (size.1 * scale).round() as i32,
    )));
    if let Ok(el) = MemoryRenderBufferRenderElement::from_buffer(
        renderer,
        loc,
        buffer,
        Some(alpha),
        None,
        physical,
        Kind::Unspecified,
    ) {
        elements.push(UiElement::Memory(el));
    }
}

/// Distance from `origin` to the farthest output corner: the radius at which a
/// radial reveal has covered the whole screen.
fn max_reveal_radius(origin: Point<f64, Logical>, size: Size<i32, Physical>) -> f32 {
    let (w, h) = (size.w as f64, size.h as f64);
    let dx = origin.x.max(w - origin.x).max(0.0);
    let dy = origin.y.max(h - origin.y).max(0.0);
    ((dx * dx + dy * dy).sqrt()) as f32
}

fn send_frames(surface: &WlSurface, time: u32) {
    smithay::wayland::compositor::with_surface_tree_downward(
        surface,
        (),
        |_, _, &()| smithay::wayland::compositor::TraversalAction::DoChildren(()),
        |_surf, states, &()| {
            for callback in states
                .cached_state
                .get::<smithay::wayland::compositor::SurfaceAttributes>()
                .current()
                .frame_callbacks
                .drain(..)
            {
                callback.done(time);
            }
        },
        |_, _, &()| true,
    );
}
