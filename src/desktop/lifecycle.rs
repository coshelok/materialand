//! Lifecycle.
use super::*;

impl State {
    pub fn request_close_window(&mut self, window: &Window) {
        if self.shell.begin_window_close(window) {
            if let Some(t) = window.toplevel() {
                t.send_close();
            }
        }
    }

    pub fn finish_closing(&mut self) {
        for window in self.shell.take_closed_windows() {
            if let Some(t) = window.toplevel() {
                t.send_close();
            }
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
