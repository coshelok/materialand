//! Popups.
use super::*;

impl State {
    pub(super) fn unconstrain_popup(&self, popup: &PopupSurface) {
        let kind = PopupKind::Xdg(popup.clone());
        let Ok(root) = find_popup_root_surface(&kind) else {
            return;
        };
        let Some(window) = self
            .windows
            .space
            .elements()
            .find(|w| w.wl_surface().as_deref() == Some(&root))
            .cloned()
        else {
            return;
        };
        let window_geo = self
            .windows
            .space
            .element_geometry(&window)
            .unwrap_or_else(|| window.geometry());
        // Start from the whole output and subtract the popup's current
        // toplevel offset and the window location. This is a single-output
        // compositor, so `output_size` is the entire desktop rect.
        let mut target = Rectangle::from_size(self.display.size);
        target.loc -= get_popup_toplevel_coords(&kind);
        target.loc -= window_geo.loc;
        popup.with_pending_state(|state| {
            state.geometry = state.positioner.get_unconstrained_geometry(target);
        });
    }
}
