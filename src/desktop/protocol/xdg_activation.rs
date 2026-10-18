//! Smithay XdgActivationHandler adapter.
use super::super::*;
impl XdgActivationHandler for State {
    fn activation_state(&mut self) -> &mut XdgActivationState {
        &mut self.activation_state
    }

    fn request_activation(
        &mut self,
        _token: XdgActivationToken,
        _token_data: XdgActivationTokenData,
        surface: WlSurface,
    ) {
        // Bring the target toplevel forward. Minimized windows are restored so
        // activation from a notification or file manager actually shows them.
        let window = self
            .windows
            .space
            .elements()
            .chain(self.windows.props.keys())
            .find(|w| w.wl_surface().map(|s| *s == surface).unwrap_or(false))
            .cloned();
        if let Some(window) = window {
            if self.window_props(&window).minimized {
                let serial = self.next_serial();
                self.restore_window(&window, serial);
            } else {
                self.windows.space.raise_element(&window, false);
                let serial = self.next_serial();
                self.focus_window(Some(window), serial);
            }
        }
    }
}
