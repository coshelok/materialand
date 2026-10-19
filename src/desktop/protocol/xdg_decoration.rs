//! Respect the client's decoration preference instead of forcing a second frame.
use super::super::*;
use smithay::reexports::wayland_protocols::xdg::decoration::zv1::server::zxdg_toplevel_decoration_v1::Mode;

fn configure(toplevel: ToplevelSurface, mode: Mode) {
    tracing::debug!(?mode, "configuring window decorations");
    toplevel.with_pending_state(|state| state.decoration_mode = Some(mode));
    if toplevel.is_initial_configure_sent() {
        toplevel.send_configure();
    }
}

impl XdgDecorationHandler for State {
    fn new_decoration(&mut self, toplevel: ToplevelSurface) {
        // Supporting the protocol alone is not a request for a system titlebar.
        configure(toplevel, Mode::ClientSide);
    }

    fn request_mode(&mut self, toplevel: ToplevelSurface, mode: Mode) {
        configure(toplevel, mode);
    }

    fn unset_mode(&mut self, toplevel: ToplevelSurface) {
        configure(toplevel, Mode::ClientSide);
    }
}
