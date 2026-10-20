//! Winit owns only the host window/EGL framebuffer. Client coordinates stay logical.
use crate::{app, graphics::scene, input, Result};
use ::winit::platform::pump_events::PumpStatus;
use smithay::{
    backend::{
        input::{AbsolutePositionEvent, Event, InputEvent},
        renderer::{damage::OutputDamageTracker, gles::GlesRenderer, Color32F, ImportDma},
        winit::{self, WinitEvent},
    },
    output::Mode,
    utils::{Physical, Size, Transform},
};
use std::time::Instant;
pub fn run() -> Result<()> {
    let (mut backend, mut events) = winit::init::<GlesRenderer>()?;
    backend.window().set_cursor_visible(false);
    let formats = backend
        .renderer()
        .dmabuf_formats()
        .iter()
        .copied()
        .collect();
    let (mut display, mut state) = app::create(formats)?;
    // Connect to the host before publishing our own socket to children.
    let listener = app::runtime::socket()?;
    let _bus = app::session_bus::SessionBus::start();
    state.notifications = crate::notifications::Notifications::start();
    let mut size = backend.window_size();
    let mut refresh = host_refresh(backend.window());
    resize(&mut state, size, refresh);
    let mut tracker = OutputDamageTracker::new(size, 1.0, Transform::Flipped180);
    let mut ticker = app::Ticker::new(&mut state);
    let mut animating = false;
    let mut dirty = true;
    while state.running {
        let started = Instant::now();
        let mut resize_to = None;
        let status = events.dispatch_new_events(|event| match event {
            WinitEvent::Resized { size, .. } => resize_to = Some(size),
            WinitEvent::Redraw => dirty = true,
            WinitEvent::CloseRequested => state.running = false,
            WinitEvent::Input(event) => {
                dirty = true;
                match event {
                    InputEvent::Keyboard { event } => input::on_keyboard(&mut state, &event),
                    InputEvent::PointerMotionAbsolute { event } => {
                        let location = event.position_transformed(state.display.size);
                        input::on_pointer_motion(&mut state, location, event.time_msec());
                    }
                    InputEvent::PointerButton { event } => {
                        input::on_pointer_button(&mut state, &event)
                    }
                    InputEvent::PointerAxis { event } => {
                        input::on_pointer_axis::<winit::WinitInput>(&mut state, event)
                    }
                    _ => {}
                }
            }
            _ => {}
        });
        if !state.running || matches!(status, PumpStatus::Exit(_)) {
            break;
        }
        let next_refresh = host_refresh(backend.window());
        if next_refresh != refresh {
            refresh = next_refresh;
            resize(&mut state, size, refresh);
            dirty = true;
        }
        let budget = app::frame_interval(Some(refresh));
        if let Some(next) = resize_to.filter(|next| next.w > 0 && next.h > 0 && *next != size) {
            size = next;
            resize(&mut state, size, refresh);
            tracker = OutputDamageTracker::new(size, 1.0, Transform::Flipped180);
            dirty = true;
        }
        app::clients::accept(&mut display, &listener)?;
        dirty |= app::clients::dispatch(&mut display, &mut state)? > 0;
        let was_animating = animating;
        animating = ticker.tick(&mut state, budget);
        dirty |= was_animating || animating;
        if dirty && size.w > 0 && size.h > 0 {
            // Bind errors leave the scene dirty; a later event/tick can retry safely.
            let rendered = match backend.bind() {
                Ok((renderer, mut framebuffer)) => {
                    state.validate_imports(renderer);
                    let elements = scene::build_ui(&mut state, renderer, size);
                    tracker
                        .render_output(renderer, &mut framebuffer, 0, &elements, Color32F::BLACK)
                        .map(|_| ())
                        .map_err(|error| tracing::warn!(?error, "nested frame failed"))
                }
                Err(error) => {
                    tracing::warn!(?error, "nested framebuffer unavailable");
                    Err(())
                }
            };
            if rendered.is_ok() {
                display.flush_clients()?;
                backend.submit(None)?;
                state.shell.window_frame_presented();
                scene::send_frames_all(&state);
                display.flush_clients()?;
                dirty = false;
            }
        }
        std::thread::sleep(budget.saturating_sub(started.elapsed()));
    }
    Ok(())
}
fn host_refresh(window: &::winit::window::Window) -> i32 {
    window
        .current_monitor()
        .and_then(|monitor| monitor.refresh_rate_millihertz())
        .and_then(|refresh| i32::try_from(refresh).ok())
        .filter(|refresh| *refresh > 0)
        .unwrap_or(60_000)
}
fn resize(state: &mut crate::desktop::State, size: Size<i32, Physical>, refresh: i32) {
    if size.w > 0 && size.h > 0 {
        state.set_output_mode(Mode { size, refresh });
    }
}
