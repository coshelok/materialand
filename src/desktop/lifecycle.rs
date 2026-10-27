//! Lifecycle.
use super::*;

impl State {
    pub fn request_close_window(&mut self, window: &Window) {
        // Close is a request, not permission to hide a live client. It may need
        // to show a save dialog or refuse the request. Keep its pixels, focus
        // and input intact until the client actually unmaps/destroys the window.
        // Any future exit animation must use a snapshot of that confirmed exit.
        self.shell.cancel_window_open(window);
        if let Some(t) = window.toplevel() {
            t.send_close();
        }
    }

    pub fn next_serial(&mut self) -> Serial {
        smithay::utils::SERIAL_COUNTER.next_serial()
    }

    pub fn find_window(&self, surface: &ToplevelSurface) -> Option<Window> {
        self.windows
            .props
            .keys()
            .find(|w| w.toplevel().map(|t| t == surface).unwrap_or(false))
            .cloned()
    }

    pub(super) fn refresh_meta(&mut self, surface: &ToplevelSurface) {
        if let Some(window) = self
            .windows
            .props
            .keys()
            .find(|w| w.toplevel().map(|t| t == surface).unwrap_or(false))
            .cloned()
        {
            let meta = default_meta(&window);
            self.windows.meta.insert(window, meta);
        }
    }
}
