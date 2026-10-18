//! Application bootstrap; display backends own their event loops and devices.
mod cli;
pub mod clients;
mod clock;
pub mod runtime;
pub mod session_bus;
mod tick;
use crate::{backend, config, desktop::State, graphics::text::Fonts, theme::ThemeSpec, Result};
use smithay::{backend::allocator::Format, reexports::wayland_server::Display};
pub use tick::Ticker;
pub fn create(formats: Vec<Format>) -> Result<(Display<State>, State)> {
    let mut display = Display::new()?;
    let (spec, source) = ThemeSpec::from_env();
    tracing::info!(source=%source.describe(),"material palette");
    let state = State::new(
        &mut display,
        Fonts::load()?,
        config::scan_apps(),
        formats,
        spec,
    );
    Ok((display, state))
}
pub fn run() -> Result<()> {
    let filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| "materialand=info".into());
    tracing_subscriber::fmt().with_env_filter(filter).init();
    let mode = cli::parse(
        std::env::args().skip(1),
        std::env::var("MATERIALAND_NESTED").is_ok_and(|v| v == "1"),
    )?;
    if mode == cli::Mode::Help {
        cli::help();
        return Ok(());
    }
    runtime::prepare()?;
    match mode {
        cli::Mode::Nested => backend::nested::run(),
        cli::Mode::Probe => backend::drm::probe(),
        cli::Mode::Direct => {
            let event_loop =
                smithay::reexports::calloop::EventLoop::<backend::drm::App>::try_new()?;
            let backend = backend::drm::Backend::new(event_loop.handle())?;
            let (display, mut state) = create(backend.dmabuf_formats())?;
            state.session_mode = true;
            let listener = runtime::socket()?;
            let _bus = session_bus::SessionBus::start();
            let ticker = Ticker::new(&mut state);
            crate::input::spawn_program(&state.shell.terminal);
            backend::drm::run(event_loop, display, state, listener, ticker, backend)
        }
        cli::Mode::Help => unreachable!(),
    }
}
