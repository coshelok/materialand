//! Buffer commits are the only authority for mapping client windows.
use super::super::*;
impl CompositorHandler for State {
    fn compositor_state(&mut self) -> &mut CompositorState {
        &mut self.compositor_state
    }
    fn client_compositor_state<'a>(
        &self,
        client: &'a smithay::reexports::wayland_server::Client,
    ) -> &'a CompositorClientState {
        &client
            .get_data::<ClientState>()
            .expect("registered Wayland client")
            .compositor_state
    }
    fn commit(&mut self, surface: &WlSurface) {
        smithay::backend::renderer::utils::on_commit_buffer_handler::<Self>(surface);
        self.popup_manager.commit(surface);
        let mut root = surface.clone();
        while let Some(parent) = compositor::get_parent(&root) {
            root = parent;
        }
        if let Some(PopupKind::Xdg(popup)) = self.popup_manager.find_popup(surface) {
            if !popup.is_initial_configure_sent() {
                self.unconstrain_popup(&popup);
                let _ = popup.send_configure();
            }
        }
        let window = self
            .windows
            .props
            .keys()
            .find(|w| w.wl_surface().as_deref() == Some(&root))
            .cloned();
        if let Some(window) = window {
            window.on_commit();
            if let Some(toplevel) = window.toplevel() {
                if !toplevel.is_initial_configure_sent() {
                    toplevel.send_configure();
                }
            }
            if self.window_has_content(&window) {
                if !self.window_props(&window).minimized {
                    if self.windows.space.element_location(&window).is_none() {
                        let location = self.next_location();
                        self.windows
                            .space
                            .map_element(window.clone(), location, false);
                        let app_id = self.window_meta(&window).app_id;
                        self.shell.on_window_mapped(&window, &app_id);
                        let serial = self.next_serial();
                        self.focus_window(Some(window.clone()), serial);
                    }
                    self.commit_resize(&window);
                    self.reconcile_window_placement(&window);
                }
            } else {
                self.windows.space.unmap_elem(&window);
                self.shell.on_window_destroyed(&window);
                if self.windows.focused.as_ref() == Some(&window) {
                    let next = self.windows.space.elements().last().cloned();
                    let serial = self.next_serial();
                    self.focus_window(next, serial);
                }
            }
        }
        self.windows.space.refresh();
    }
}
