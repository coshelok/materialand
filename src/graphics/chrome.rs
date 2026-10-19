//! Shell chrome in back-to-front paint order.
use super::{scene::UiElement, uploads::*};
use crate::{
    desktop::{Interaction, State},
    input,
    shell::CircleMask,
    theme::Theme,
};
use smithay::{
    backend::renderer::gles::GlesRenderer,
    utils::{Physical, Size},
};
pub(super) fn paint(
    state: &mut State,
    renderer: &mut GlesRenderer,
    size: Size<i32, Physical>,
    theme: Theme,
    second: Option<(Theme, Option<CircleMask>, f32)>,
) -> Vec<UiElement> {
    let mut elements = Vec::new();
    let (win_w, win_h) = (size.w as f32, size.h as f32);
    let fullscreen = state.has_fullscreen_window();
    // Dock.
    if !fullscreen {
        let items = input::dock_items(state);
        let focused = state.windows.focused.clone();
        let (dock, dock_loc) = state.shell.render_dock(
            &theme,
            &mut state.fonts,
            win_w,
            win_h,
            &items,
            focused.as_ref(),
            None,
        );
        push_buffer(
            &mut elements,
            renderer,
            &dock,
            (dock_loc.x, dock_loc.y),
            None,
        );
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
            let (topbar, topbar_loc) =
                state
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
        let frame =
            state
                .shell
                .render_launcher(&theme, &mut state.fonts, win_w, win_h, launcher_p, None);
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
        let frame = state
            .shell
            .render_power_menu(&theme, &mut state.fonts, win_w, power_p, None);
        push_overlay(&mut elements, renderer, &frame, 1.0);
        if let Some((target, mask, alpha)) = second {
            let frame =
                state
                    .shell
                    .render_power_menu(&target, &mut state.fonts, win_w, power_p, mask);
            push_overlay(&mut elements, renderer, &frame, alpha);
        }
    }

    if !fullscreen && (state.shell.calendar.open || state.shell.calendar_progress() > 0.001) {
        let frame = state
            .shell
            .render_calendar(&theme, &mut state.fonts, win_w, win_h, None);
        push_overlay(&mut elements, renderer, &frame, 1.0);
        if let Some((target, mask, alpha)) = second {
            let frame = state
                .shell
                .render_calendar(&target, &mut state.fonts, win_w, win_h, mask);
            push_overlay(&mut elements, renderer, &frame, alpha);
        }
    }

    if let Some((buffer, loc)) =
        state
            .shell
            .render_app_menu(&theme, &mut state.fonts, win_w, win_h, None)
    {
        push_buffer(&mut elements, renderer, &buffer, (loc.x, loc.y), None);
        if let Some((target, mask, alpha)) = second {
            if let Some((buffer, loc)) =
                state
                    .shell
                    .render_app_menu(&target, &mut state.fonts, win_w, win_h, mask)
            {
                push_buffer(
                    &mut elements,
                    renderer,
                    &buffer,
                    (loc.x, loc.y),
                    Some(alpha),
                );
            }
        }
    }
    elements
}
