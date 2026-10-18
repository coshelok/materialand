//! Frame callbacks are released by a backend only after successful submission.
use crate::desktop::State;
use smithay::{
    desktop::PopupManager,
    reexports::wayland_server::protocol::wl_surface::WlSurface,
    wayland::{
        compositor::{with_surface_tree_downward, SurfaceAttributes, TraversalAction},
        seat::WaylandFocus,
    },
};
pub fn send_frames_all(state: &State) {
    let time = state.start_time.elapsed().as_millis() as u32;
    for window in state
        .windows
        .space
        .elements_for_output(&state.display.handle)
    {
        if !state.window_has_content(window) {
            continue;
        }
        if let Some(surface) = window.wl_surface() {
            send_frames(&surface, time);
            for (popup, _) in PopupManager::popups_for_surface(&surface) {
                send_frames(popup.wl_surface(), time);
            }
        }
    }
    if let smithay::input::pointer::CursorImageStatus::Surface(surface) = &state.cursor_status {
        send_frames(surface, time);
    }
}
fn send_frames(surface: &WlSurface, time: u32) {
    with_surface_tree_downward(
        surface,
        (),
        |_, _, _| TraversalAction::DoChildren(()),
        |_, states, _| {
            for callback in states
                .cached_state
                .get::<SurfaceAttributes>()
                .current()
                .frame_callbacks
                .drain(..)
            {
                callback.done(time);
            }
        },
        |_, _, _| true,
    );
}
