//! Use scene ordering for hover/release, so overlays cannot click through.
use super::{
    target::{target, Target},
    *,
};
use crate::{desktop::hit_test::WindowHit, shell::DecorButton};
use smithay::desktop::Window;

fn hit(state: &State, pos: Point<f64, Logical>) -> Option<(Window, DecorButton)> {
    if !matches!(state.interaction, Interaction::None) || state.pointer.is_grabbed() {
        return None;
    }
    match target(state, pos) {
        Target::Window(WindowHit::Decoration { window, geometry }) => {
            DecorButton::from_hit(shell::decor_hit(&shell::decor_layout(geometry), pos))
                .map(|button| (window, button))
        }
        _ => None,
    }
}
pub(crate) fn update_hover(state: &mut State) -> bool {
    let next = hit(state, state.cursor);
    state.shell.decoration_controls.hover(next)
}
pub(super) fn release(state: &mut State, pos: Point<f64, Logical>) -> bool {
    if !state.shell.decoration_controls.has_press() {
        return false;
    }
    let hit = hit(state, pos);
    if let Some((window, button)) = state.shell.decoration_controls.release(hit) {
        match button {
            DecorButton::Close => state.request_close_window(&window),
            DecorButton::Minimize => state.minimize_window(&window),
            DecorButton::Maximize => state.toggle_maximize(&window),
        }
    }
    true
}
