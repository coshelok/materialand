mod anim;
mod clock;
mod config;
mod font;
mod input;
mod render;
mod session;
mod shell;
mod state;
mod style;
mod theme;
mod ui;

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;

use ::winit::platform::pump_events::PumpStatus;

use smithay::{
    backend::{
        allocator::Format,
        input::{AbsolutePositionEvent, Event, InputEvent},
        renderer::{
            gles::GlesRenderer, utils::draw_render_elements, Color32F, Frame, ImportDma, Renderer,
            RendererSuper,
        },
        winit::{self, WinitEvent},
    },
    reexports::wayland_server::{Display, ListeningSocket},
    utils::{Physical, Rectangle, Size, Transform},
};

use crate::{
    font::Fonts,
    state::{ClientState, State},
};

/// Guarantees that `XDG_RUNTIME_DIR` points at a usable per-user runtime
/// directory. A graphical login session always sets it, but launching the
/// compositor from a bare TTY (e.g. `DISPLAY=:1 ./materialand`) leaves it
/// unset, which makes `ListeningSocket::bind` fail with `RuntimeDirNotSet`.
///
/// When the variable is missing we fall back to `/run/user/<uid>`, and, if
/// that does not exist, create a private directory under the system temp dir.
fn ensure_runtime_dir() -> Result<(), Box<dyn std::error::Error>> {
    if let Some(dir) = std::env::var_os("XDG_RUNTIME_DIR") {
        let path = PathBuf::from(&dir);
        if path.is_absolute() {
            return Ok(());
        }
    }

    let uid = smithay::reexports::rustix::process::getuid().as_raw();
    let system = PathBuf::from(format!("/run/user/{uid}"));
    if system.is_dir() {
        std::env::set_var("XDG_RUNTIME_DIR", &system);
        tracing::info!(dir = %system.display(), "XDG_RUNTIME_DIR was unset; using system runtime dir");
        return Ok(());
    }

    let fallback = std::env::temp_dir().join(format!("materialand-{uid}"));
    std::fs::create_dir_all(&fallback)?;
    // The Wayland protocol requires this directory to be private to the user.
    std::fs::set_permissions(
        &fallback,
        std::os::unix::fs::PermissionsExt::from_mode(0o700),
    )?;
    std::env::set_var("XDG_RUNTIME_DIR", &fallback);
    tracing::warn!(dir = %fallback.display(), "XDG_RUNTIME_DIR was unset; using a temporary runtime dir");
    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    if let Ok(filter) = tracing_subscriber::EnvFilter::try_from_default_env() {
        tracing_subscriber::fmt().with_env_filter(filter).init();
    } else {
        tracing_subscriber::fmt().init();
    }

    let args: Vec<String> = std::env::args().skip(1).collect();
    let has = |flag: &str| args.iter().any(|arg| arg == flag);

    if has("--help") || has("-h") {
        print_help();
        return Ok(());
    }
    // `--check` reports what the KMS backend would pick without touching the
    // display, so it is safe inside a running session.
    if has("--check") {
        ensure_runtime_dir()?;
        return session::probe();
    }

    ensure_runtime_dir()?;

    // Nested (winit) mode stays available for development and for trying the
    // shell without leaving the running desktop.
    let nested = has("--nested") || std::env::var_os("MATERIALAND_NESTED").is_some();
    if nested {
        run_nested()
    } else {
        run_session()
    }
}

fn print_help() {
    println!(
        "materialand — a Material 3 desktop compositor

Usage: materialand [OPTIONS]

Options:
  --nested   run as a nested window inside the current compositor
  --check    list the DRM devices, connectors and modes that would be used
  -h, --help print this help

Without options the compositor takes over the machine as a real session
(DRM/KMS + libinput through libseat).

Environment:
  MATERIALAND_NESTED=1        same as --nested
  MATERIALAND_DRM_DEVICE=...  DRM card to try first (e.g. /dev/dri/card1)
  MATERIALAND_SEED=#rrggbb    palette seed (otherwise taken from the wallpaper)
  MATERIALAND_WALLPAPER=...   wallpaper path for the palette seed
  MATERIALAND_VARIANT=...     Material You variant name
  MATERIALAND_FONT_DIR=...    directory searched first for UI fonts
  MATERIALAND_XKB_LAYOUT=...  keyboard layouts, e.g. us,ru (see README)
  MATERIALAND_XKB_OPTIONS=... xkb options, e.g. grp:alt_shift_toggle"
    );
}

/// Full session: DRM/KMS output, libinput input, driven by calloop.
fn run_session() -> Result<(), Box<dyn std::error::Error>> {
    let event_loop = smithay::reexports::calloop::EventLoop::<session::App>::try_new()?;
    let handle = event_loop.handle();

    // The session backend has to come first: it opens the GPU, and the
    // `zwp_linux_dmabuf_v1` global must advertise the renderer's formats.
    let backend = session::Backend::new(handle)?;
    let dmabuf_formats = backend.dmabuf_formats();

    let (display, mut state) = init_common(dmabuf_formats)?;
    let socket_name = format!("materialand-{}", std::process::id());
    let listener = ListeningSocket::bind(&socket_name)?;

    // Clients spawned later must connect to us. There is no XWayland, so
    // `DISPLAY` stays unset; the session metadata tells toolkits which kind of
    // session they are in.
    std::env::set_var("WAYLAND_DISPLAY", &socket_name);
    std::env::set_var("XDG_SESSION_TYPE", "wayland");
    std::env::set_var("XDG_CURRENT_DESKTOP", "Materialand");
    input::set_session_mode(true);

    let ticker = ui::Ticker::new(&mut state);

    // Launch the detected terminal so there is something to see immediately.
    input::spawn_program(&state.shell.terminal);

    session::run(event_loop, display, state, listener, ticker, backend)
}

/// Nested mode: a window inside the host compositor, used for development.
fn run_nested() -> Result<(), Box<dyn std::error::Error>> {
    // The winit backend must exist before the state so the `zwp_linux_dmabuf_v1`
    // global can advertise exactly the formats the GLES renderer can import.
    let (mut backend, mut winit) = winit::init::<GlesRenderer>()?;

    // The compositor draws its own pointer inside the nested window, so the
    // host's cursor must be hidden or two cursors would be visible at once.
    backend.window().set_cursor_visible(false);

    let dmabuf_formats: Vec<_> = backend.renderer().dmabuf_formats().iter().copied().collect();
    let (mut display, mut state) = init_common(dmabuf_formats)?;

    let socket_name = format!("materialand-{}", std::process::id());
    let listener = ListeningSocket::bind(&socket_name)?;

    // Must be set *after* `winit::init`: the nested backend needs the host's
    // `WAYLAND_DISPLAY` to create its window, while clients spawned later must
    // connect to us instead.
    std::env::set_var("WAYLAND_DISPLAY", &socket_name);

    let initial = backend.window_size();
    update_output(&mut state, initial);

    let mut ticker = ui::Ticker::new(&mut state);

    // Frame pacing: `WinitEventLoop::dispatch_new_events` never blocks (it
    // pumps with a zero timeout), so without this the loop would spin at 100%
    // CPU. Sleep out the remainder of each frame budget to cap at ~60 FPS.
    let frame_budget = std::time::Duration::from_micros(16_667);
    let mut next_frame = Instant::now();

    // Launch the detected terminal so there is something to see immediately.
    input::spawn_program(&state.shell.terminal);
    tracing::info!("compositor ready on WAYLAND_DISPLAY={socket_name}");

    // Redraw only when something actually changed: an idle compositor must not
    // repaint the whole screen every frame. Set whenever our own shell state
    // changes (theme, launcher, ...).
    let mut dirty = true;
    loop {
        let mut events: usize = 0;
        let status = winit.dispatch_new_events(|event| {
            events += 1;
            match event {
                WinitEvent::Resized { size, .. } => update_output(&mut state, size),
                WinitEvent::Input(InputEvent::Keyboard { event }) => {
                    input::on_keyboard(&mut state, &event);
                }
                WinitEvent::Input(InputEvent::PointerMotionAbsolute { event }) => {
                    let loc = event.position_transformed(state.output_size);
                    input::on_pointer_motion(&mut state, loc, event.time() as u32);
                }
                WinitEvent::Input(InputEvent::PointerButton { event }) => {
                    input::on_pointer_button(&mut state, &event);
                }
                WinitEvent::CloseRequested => state.running = false,
                _ => {}
            }
        });

        if !state.running {
            break;
        }
        if let PumpStatus::Exit(_) = status {
            break;
        }

        // Accept pending clients.
        if let Some(stream) = listener.accept()? {
            tracing::info!("client connected");
            display
                .handle()
                .insert_client(stream, Arc::new(ClientState::default()))?;
        }

        // `dispatch_clients` yields the number of processed events; combined
        // with the winit events it tells us whether a redraw is warranted.
        let dispatched = display.dispatch_clients(&mut state)?;
        display.flush_clients()?;
        state.space.refresh();
        state.popup_manager.cleanup();

        // Clock, animations and pending window closes.
        if ticker.tick(&mut state) {
            dirty = true;
        }

        if dirty || events > 0 || dispatched > 0 {
            let size = backend.window_size();
            let rendered = match backend.bind() {
                Ok((renderer, mut framebuffer)) => {
                    render_nested(&mut state, renderer, &mut framebuffer, size)
                }
                Err(error) => {
                    tracing::error!(?error, "failed to bind the output framebuffer");
                    false
                }
            };
            if rendered {
                backend.submit(Some(&[Rectangle::from_size(size)]))?;
                dirty = false;
            } else {
                // Keep the output dirty; a transient renderer/import error should
                // be retried instead of panicking or freezing the shell.
                dirty = true;
                std::thread::sleep(std::time::Duration::from_millis(8));
            }
        }

        // Frame pacing: `WinitEventLoop::dispatch_new_events` never blocks (it
        // pumps with a zero timeout), so cap the loop at ~60 FPS. When idle this
        // merely sleeps; while clients animate it paces the redraws.
        next_frame += frame_budget;
        let now = Instant::now();
        if next_frame > now {
            std::thread::sleep(next_frame - now);
        } else {
            // We are behind schedule (heavy frame); resync instead of
            // accumulating a backlog of missed frames.
            next_frame = now;
        }
    }

    Ok(())
}

/// Shared start-up: palette, fonts, application database and shell state.
fn init_common(
    dmabuf_formats: Vec<Format>,
) -> Result<(Display<State>, State), Box<dyn std::error::Error>> {
    let mut display: Display<State> = Display::new()?;
    let fonts = Fonts::load()?;
    let apps = config::scan_apps();
    tracing::info!(apps = apps.len(), "discovered applications");

    // Read the Material You palette seed: `MATERIALAND_SEED` wins, otherwise
    // it is extracted from `MATERIALAND_WALLPAPER`, otherwise the baseline
    // purple. `MATERIALAND_VARIANT` selects one of the nine Material variants.
    let (spec, seed_source) = theme::ThemeSpec::from_env();
    tracing::info!(
        seed = %format!("#{:06x}", spec.seed & 0x00ff_ffff),
        variant = spec.variant.name(),
        source = %seed_source.describe(),
        "material palette"
    );

    let state = State::new(&mut display, fonts, apps, dmabuf_formats, spec);
    Ok((display, state))
}

/// Resize the output to the nested window.
fn update_output(state: &mut State, size: Size<i32, Physical>) {
    if size.w <= 0 || size.h <= 0 {
        return;
    }
    if state.output_size.w == size.w && state.output_size.h == size.h {
        return;
    }
    state.output_size = Size::from((size.w, size.h));
    state.output.change_current_state(
        Some(smithay::output::Mode {
            size: (size.w, size.h).into(),
            refresh: 60000,
        }),
        None,
        None,
        None,
    );
    state.shell.mark_dirty();
}

fn render_nested(
    state: &mut State,
    renderer: &mut GlesRenderer,
    framebuffer: &mut <GlesRenderer as RendererSuper>::Framebuffer<'_>,
    size: Size<i32, Physical>,
) -> bool {
    let damage = Rectangle::from_size(size);

    // All shell layers, including the brightness/night-light overlays.
    let elements = ui::build_ui(state, renderer, size);

    let mut frame = match renderer.render(framebuffer, size, Transform::Flipped180) {
        Ok(frame) => frame,
        Err(e) => {
            tracing::error!("render failed: {e:?}");
            return false;
        }
    };
    if let Err(e) = frame.clear(Color32F::new(0.0, 0.0, 0.0, 1.0), &[damage]) {
        tracing::error!("clear failed: {e:?}");
        return false;
    }
    if let Err(e) = draw_render_elements(&mut frame, 1.0, &elements, &[damage]) {
        tracing::error!("draw failed: {e:?}");
        return false;
    }
    if let Err(error) = frame.finish() {
        tracing::error!(?error, "failed to finish the render frame");
        return false;
    }

    // Frame callbacks toward clients.
    ui::send_frames_all(state);
    true
}
