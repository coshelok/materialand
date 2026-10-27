//! Smithay XdgActivationHandler adapter.
use super::super::activation::ACTIVATION_TIMEOUT;
use super::super::*;
use smithay::reexports::wayland_server::{backend::ClientId, Resource};

struct Grant {
    owner: ClientId,
    issued: Instant,
}

impl State {
    pub(crate) fn expire_activation_tokens(&mut self) {
        let now = Instant::now();
        self.activation_state.retain_tokens(|_, data| {
            data.user_data.get::<Grant>().is_some_and(|grant| {
                now.saturating_duration_since(grant.issued) < ACTIVATION_TIMEOUT
            })
        });
    }
}
impl XdgActivationHandler for State {
    fn activation_state(&mut self) -> &mut XdgActivationState {
        &mut self.activation_state
    }

    fn token_created(&mut self, _token: XdgActivationToken, data: XdgActivationTokenData) -> bool {
        self.expire_activation_tokens();
        if self.activation_state.tokens().count() >= 64 {
            return false;
        }
        let Some(owner) = data.client_id.as_ref() else {
            return false;
        };
        let Some((serial, seat)) = data.serial.as_ref() else {
            return false;
        };
        if Seat::<State>::from_resource(seat)
            .and_then(|s| s.get_keyboard())
            .as_ref()
            != Some(&self.keyboard)
        {
            return false;
        }
        let focused_owner = self
            .keyboard
            .current_focus()
            .and_then(|s| s.client())
            .map(|c| c.id());
        if focused_owner.as_ref() != Some(owner) {
            return false;
        }
        if data
            .surface
            .as_ref()
            .is_some_and(|s| s.client().map(|c| c.id()).as_ref() != Some(owner))
        {
            return false;
        }
        let Some(issued) = self.activation_inputs.claim(*serial, owner, Instant::now()) else {
            return false;
        };
        data.user_data.insert_if_missing(|| Grant {
            owner: owner.clone(),
            issued,
        });
        true
    }

    fn request_activation(
        &mut self,
        token: XdgActivationToken,
        token_data: XdgActivationTokenData,
        surface: WlSurface,
    ) {
        // Consume before checking: a rejected or successful request cannot be
        // replayed later when the user's focus has changed again.
        if !self.activation_state.remove_token(&token) {
            return;
        }
        let Some(grant) = token_data.user_data.get::<Grant>() else {
            return;
        };
        if grant.issued.elapsed() >= ACTIVATION_TIMEOUT {
            return;
        }
        let focused_owner = self
            .keyboard
            .current_focus()
            .and_then(|s| s.client())
            .map(|c| c.id());
        if focused_owner.as_ref() != Some(&grant.owner) {
            return;
        }
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
