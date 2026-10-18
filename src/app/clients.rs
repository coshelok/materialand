//! Shared client acceptance, dispatch and state refresh.
use crate::{
    desktop::{ClientState, State},
    Result,
};
use smithay::reexports::wayland_server::{Display, ListeningSocket};
use std::sync::Arc;
pub fn accept(display: &mut Display<State>, listener: &ListeningSocket) -> Result<()> {
    while let Some(stream) = listener.accept()? {
        display
            .handle()
            .insert_client(stream, Arc::new(ClientState::default()))?;
    }
    Ok(())
}
pub fn dispatch(display: &mut Display<State>, state: &mut State) -> Result<usize> {
    let count = display.dispatch_clients(state)?;
    state.windows.space.refresh();
    state.popup_manager.cleanup();
    display.flush_clients()?;
    Ok(count)
}
