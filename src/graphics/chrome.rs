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
    opening_windows: Vec<Vec<UiElement>>,
) -> Vec<UiElement> {
    let mut elements = Vec::new();
    let (win_w, win_h) = (size.w as f32, size.h as f32);
    let fullscreen = state.has_fullscreen_window();
    // Dock.
    if !fullscreen && !crate::shell::tablet_mode() {
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
                None,
            );
            push_masked_buffer(
                &mut elements,
                renderer,
                &dock,
                (dock_loc.x, dock_loc.y),
                Some(alpha),
                mask,
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
                    .render_topbar(&target, &mut state.fonts, win_w, &title, None);
            push_masked_buffer(
                &mut elements,
                renderer,
                &topbar,
                (topbar_loc.x, topbar_loc.y),
                Some(alpha),
                mask,
            );
        }
    }

    let shade_p = state.shell.quick_settings_progress().clamp(0.0, 1.0);
    if shade_p > 0.001 && !(crate::shell::tablet_mode() && crate::config::design().shade.fullscreen)
    {
        let layer = crate::shell::LayerFrame {
            rounded: None,
            source: None,
            buffer: state.shell.scrim_buffer(&theme),
            loc: (0.0, 0.0).into(),
            size: Some((win_w as i32, win_h as i32).into()),
        };
        push_layer(
            &mut elements,
            renderer,
            &layer,
            Some(shade_p * crate::config::design().launcher.scrim_alpha),
            None,
        );
    }
    // Compact quick-settings sheet, anchored below the top bar.
    let quick_settings_p = state.shell.quick_settings_progress();
    if state.shell.quick_settings_open || quick_settings_p > 0.001 {
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
                None,
            );
            push_masked_overlay(&mut elements, renderer, &frame, alpha, mask);
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
                None,
            );
            push_masked_overlay(&mut elements, renderer, &frame, alpha, mask);
        }
    }

    if state
        .shell
        .statusbar_visible(fullscreen, state.overview.visible())
    {
        let (buffer, loc) = state
            .shell
            .render_statusbar(&theme, &mut state.fonts, win_w);
        push_buffer(&mut elements, renderer, &buffer, (loc.x, loc.y), None);
        if let Some((target, mask, alpha)) = second {
            let (buffer, loc) = state
                .shell
                .render_statusbar(&target, &mut state.fonts, win_w);
            push_masked_buffer(
                &mut elements,
                renderer,
                &buffer,
                (loc.x, loc.y),
                Some(alpha),
                mask,
            );
        }
    }

    // A launching app grows above the outgoing launcher, which otherwise
    // obscures almost its entire entrance. Panels and menus remain in front.
    for group in opening_windows {
        elements.extend(group.into_iter().rev());
    }

    // Brightness popup, drawn above every other surface.
    let brightness_p = state.shell.brightness_progress();
    state.shell.set_slider_active(matches!(
        state.interaction,
        Interaction::Brightness | Interaction::ShadeBrightness
    ));
    if state.shell.brightness_open || brightness_p > 0.001 {
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
                None,
            );
            push_masked_overlay(&mut elements, renderer, &frame, alpha, mask);
        }
    }

    // Session menu, drawn above every other surface.
    let power_p = state.shell.power_progress();
    if state.shell.power_open || power_p > 0.001 {
        let frame = state
            .shell
            .render_power_menu(&theme, &mut state.fonts, win_w, power_p, None);
        push_overlay(&mut elements, renderer, &frame, 1.0);
        if let Some((target, mask, alpha)) = second {
            let frame =
                state
                    .shell
                    .render_power_menu(&target, &mut state.fonts, win_w, power_p, None);
            push_masked_overlay(&mut elements, renderer, &frame, alpha, mask);
        }
    }

    if state.shell.calendar.open || state.shell.calendar_progress() > 0.001 {
        let frame = state
            .shell
            .render_calendar(&theme, &mut state.fonts, win_w, win_h, None);
        push_overlay(&mut elements, renderer, &frame, 1.0);
        if let Some((target, mask, alpha)) = second {
            let frame = state
                .shell
                .render_calendar(&target, &mut state.fonts, win_w, win_h, None);
            push_masked_overlay(&mut elements, renderer, &frame, alpha, mask);
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
                    .render_app_menu(&target, &mut state.fonts, win_w, win_h, None)
            {
                push_masked_buffer(
                    &mut elements,
                    renderer,
                    &buffer,
                    (loc.x, loc.y),
                    Some(alpha),
                    mask,
                );
            }
        }
    }
    elements
}
