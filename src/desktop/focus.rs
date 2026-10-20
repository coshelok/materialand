//! Focus.
use super::*;

impl State {
    pub fn focus_window(&mut self, window: Option<Window>, serial: Serial) {
        let previous = self.windows.focused.clone();
        if previous.as_ref() != window.as_ref() {
            if let Some(prev) = previous {
                if prev.set_activated(false) {
                    if let Some(t) = prev.toplevel() {
                        t.send_configure();
                    }
                }
            }
            if let Some(w) = &window {
                if w.set_activated(true) {
                    if let Some(t) = w.toplevel() {
                        t.send_configure();
                    }
                }
            }
            if let Some(window) = &window {
                self.windows.recent.retain(|item| item != window);
                self.windows.recent.push(window.clone());
            }
            self.windows.focused = window.clone();
        }

        let keyboard = self.keyboard.clone();
        match window.and_then(|w| w.wl_surface().map(|s| s.into_owned())) {
            Some(surface) => keyboard.set_focus(self, Some(surface), serial),
            None => keyboard.set_focus(self, None, serial),
        }
    }
}
