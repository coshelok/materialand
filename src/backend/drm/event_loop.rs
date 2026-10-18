//! The direct-session loop waits on sockets, devices and one frame timer.
use super::*;
use crate::app::clients;
pub fn run(
    mut event_loop: EventLoop<'static, App>,
    display: Display<State>,
    state: State,
    listener: ListeningSocket,
    ticker: crate::app::Ticker,
    backend: Backend,
) -> crate::Result<()> {
    let handle = event_loop.handle();
    let mut app = App {
        display,
        state,
        listener,
        ticker,
        backend,
        dirty: true,
        animating: false,
        timer: None,
    };
    let fd = app.display.as_fd().try_clone_to_owned()?;
    handle.insert_source(
        Generic::new(fd, calloop::Interest::READ, calloop::Mode::Level),
        |_, _, app: &mut App| {
            match clients::dispatch(&mut app.display, &mut app.state) {
                Ok(count) if count > 0 => app.request_redraw(),
                Ok(_) => {}
                Err(error) => tracing::warn!(%error,"Wayland dispatch failed"),
            }
            Ok(PostAction::Continue)
        },
    )?;
    let fd = app.listener.as_fd().try_clone_to_owned()?;
    handle.insert_source(
        Generic::new(fd, calloop::Interest::READ, calloop::Mode::Level),
        |_, _, app: &mut App| {
            if let Err(error) = clients::accept(&mut app.display, &app.listener) {
                tracing::warn!(%error,"Wayland accept failed");
            }
            Ok(PostAction::Continue)
        },
    )?;
    handle.insert_source(
        Signals::new(&[Signal::SIGINT, Signal::SIGTERM])?,
        |_, _, app: &mut App| app.state.running = false,
    )?;
    app.backend.attach_output(&mut app.state)?;
    app.request_redraw();
    while app.state.running {
        if let Err(error) = event_loop.dispatch(None::<Duration>, &mut app) {
            tracing::error!(%error,"direct event loop failed");
            return Err(error.into());
        }
    }
    Ok(())
}
