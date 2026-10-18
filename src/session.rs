//! Real-session backend: DRM/KMS outputs, libinput seats and libseat.
//!
//! Everything in this module is only used when Materialand runs as a real
//! desktop session (`materialand` without `--nested`): the compositor becomes
//! DRM master, scans out through the DRM compositor of Smithay and takes input
//! from libinput. Privileges come from libseat (logind or seatd), so the
//! compositor itself needs no root.
//!
//! Structure of the module:
//! - [`Backend`] owns the session and the currently driven GPU,
//! - [`App`] is the calloop state: the display, the shell state and the
//!   backend, driven event-driven instead of the fixed 60 Hz loop of winit,
//! - frames are paced on vblank: at most one frame is queued per output and
//!   client frame callbacks fire when the flip completes.

use std::{
    os::fd::AsFd,
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

use smithay::{
    backend::{
        allocator::{
            gbm::{GbmAllocator, GbmBufferFlags, GbmDevice},
            Fourcc,
        },
        drm::{
            compositor::{FrameError, FrameFlags, PrimaryPlaneElement},
            exporter::gbm::GbmFramebufferExporter,
            output::{DrmOutput, DrmOutputManager, DrmOutputRenderElements},
            DrmDevice, DrmDeviceFd, DrmDeviceNotifier, DrmEvent,
        },
        egl::{EGLContext, EGLDisplay},
        input::{AbsolutePositionEvent, Event, InputEvent, PointerMotionEvent},
        libinput::{LibinputInputBackend, LibinputSessionInterface},
        renderer::{gles::GlesRenderer, ImportDma},
        session::{
            libseat::{LibSeatSession, LibSeatSessionNotifier},
            Event as SessionEvent,
            Session,
        },
        udev::{self, UdevBackend, UdevEvent},
    },
    output::{Mode as OutputMode, OutputModeSource},
    reexports::{
        calloop::{
            self,
            generic::Generic,
            signals::{Signal, Signals},
            timer::{TimeoutAction, Timer},
            EventLoop, LoopHandle, PostAction, RegistrationToken,
        },
        drm::control::{self, connector, crtc, Device as _, ModeTypeFlags},
        input::Libinput,
        rustix::fs::OFlags,
        wayland_server::{Display, ListeningSocket},
    },
    utils::{DeviceFd, Physical, Size},
};

use crate::{
    input,
    state::{ClientState, State},
    ui,
};

/// At most one frame is in flight per output; while animating we want to try
/// again roughly every refresh interval.
const FRAME_BUDGET: Duration = Duration::from_micros(16_667);

/// While nothing is happening the loop still wakes once a second: the top-bar
/// clock only repaints when the displayed minute changes, so this costs one
/// timer wake-up per second and keeps the clock correct when idle.
const IDLE_TICK: Duration = Duration::from_secs(1);

/// Concrete types of the DRM output stack. `DrmDeviceFd` is the cloneable
/// device fd handed out by libseat (cloning shares the same fd, which keeps a
/// single DRM master for both KMS and GBM); `()` is the per-frame user data (a
/// single output needs none).
type Allocator = GbmAllocator<DrmDeviceFd>;
type Exporter = GbmFramebufferExporter<DrmDeviceFd>;
type Manager = DrmOutputManager<Allocator, Exporter, (), DrmDeviceFd>;
type Output = DrmOutput<Allocator, Exporter, (), DrmDeviceFd>;

/// The shared event-loop data.
pub struct App {
    display: Display<State>,
    state: State,
    listener: ListeningSocket,
    backend: Backend,
    ticker: ui::Ticker,
    /// The shell changed and wants a frame.
    dirty: bool,
    /// Shell animations are still running; keep scheduling frames.
    animating: bool,
    /// One-shot timer that coalesces redraw requests, together with the
    /// deadline it was armed for. Keeping the deadline lets a later request
    /// replace a pending idle tick with an earlier frame.
    timer: Option<(RegistrationToken, Instant)>,
}

impl App {
    /// Queue a shell tick; repeated calls before the tick happens collapse
    /// into the one already scheduled, and a nearer wake-up replaces one that
    /// was armed for later.
    fn schedule(&mut self, delay: Duration) {
        // While the session is paused there is no output to draw into and no
        // way to wake the loop back up, so don't arm anything.
        if self.backend.gpu.is_none() {
            return;
        }
        let deadline = Instant::now() + delay;
        if let Some((token, current)) = self.timer {
            if current <= deadline {
                return;
            }
            // Usually the idle tick is pending: input must not wait for it,
            // otherwise the first frame after a pause arrives late and motion
            // reads as a slideshow.
            self.backend.handle.remove(token);
            self.timer = None;
        }
        match self.backend.handle.insert_source(
            Timer::from_duration(delay),
            |_deadline, _, app: &mut App| app.on_timer(),
        ) {
            Ok(token) => self.timer = Some((token, deadline)),
            Err(err) => tracing::error!(?err, "failed to arm the frame timer"),
        }
    }

    /// A frame is queued on the output and waits for its vblank.
    fn pending(&self) -> bool {
        self.backend
            .gpu
            .as_ref()
            .map(|gpu| gpu.output.pending)
            .unwrap_or(false)
    }

    /// Advance time-dependent shell state and render if anything changed.
    fn on_timer(&mut self) -> TimeoutAction {
        self.timer = None;
        let animating = self.ticker.tick(&mut self.state);
        self.animating = animating;
        if animating {
            self.dirty = true;
        }
        if self.dirty && !self.pending() {
            render(self);
        }
        if self.animating || self.pending() || self.dirty {
            self.schedule(FRAME_BUDGET);
        } else {
            // Idle: keep ticking so the top-bar clock stays accurate.
            self.schedule(IDLE_TICK);
        }
        TimeoutAction::Drop
    }

    fn request_redraw(&mut self) {
        self.dirty = true;
        self.schedule(FRAME_BUDGET);
    }
}

/// Everything needed to drive one GPU: the output manager (kept for its
/// lifetime: it owns the DRM device and will be needed for mode changes),
/// the renderer and the active output.
struct Gpu {
    path: PathBuf,
    dev_id: u64,
    /// Frames go straight through [`OutputState::drm`]; never read directly.
    /// The manager is kept alive for the DRM device it owns (and for future
    /// mode changes), hence the underscore: ownership is the point.
    _manager: Manager,
    renderer: GlesRenderer,
    output: OutputState,
}

struct OutputState {
    drm: Output,
    crtc: crtc::Handle,
    /// A queued frame has not been acknowledged by vblank yet.
    pending: bool,
}

/// A device that was opened but has no CRTC assigned yet.
///
/// The GLES renderer must exist before `State` is built (the
/// `zwp_linux_dmabuf_v1` global advertises its formats), while the connector
/// mode can only be published through the `Output` living in `State`; this
/// struct bridges the two phases.
struct PendingGpu {
    path: PathBuf,
    dev_id: u64,
    device: DrmDevice,
    notifier: DrmDeviceNotifier,
    gbm: GbmDevice<DrmDeviceFd>,
    renderer: GlesRenderer,
    connector: connector::Handle,
    crtc: crtc::Handle,
    mode: control::Mode,
}

#[derive(Default)]
struct Sources {
    udev: Option<RegistrationToken>,
    libinput: Option<RegistrationToken>,
    drm: Option<RegistrationToken>,
}

/// A start-up error whose `Debug` view is its message.
///
/// Rust prints `Debug` for the error returned from `main`, so wrapping the
/// text in `io::Error::other` would bury it in `Custom { kind: Other, .. }`.
struct SessionOpenError(String);

impl std::fmt::Display for SessionOpenError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::fmt::Debug for SessionOpenError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for SessionOpenError {}

/// Open a libseat session, with a readable error.
///
/// libseat reports `FailedToOpenSession` when neither a running `seatd` nor a
/// logind session on a seat could be used, and the raw error gives no hint
/// about that. The README has the full troubleshooting notes.
fn open_session() -> Result<(LibSeatSession, LibSeatSessionNotifier), Box<dyn std::error::Error>> {
    LibSeatSession::new().map_err(|error| {
        SessionOpenError(format!(
            "could not open a libseat session ({error}); libseat needs a logind session on \
             the seat or a running seatd daemon. Start the compositor from a display-manager \
             session or a TTY login instead of a systemd user service, or enable seatd; use \
             --nested to run inside another compositor"
        ))
        .into()
    })
}

pub struct Backend {
    session: LibSeatSession,
    seat: String,
    handle: LoopHandle<'static, App>,
    sources: Sources,
    /// GPU opened in [`Backend::new`], waiting for the shell state.
    pending: Option<PendingGpu>,
    gpu: Option<Gpu>,
}

impl Backend {
    /// Connect to the session manager and open the first GPU that has a
    /// connected connector.
    pub fn new(handle: LoopHandle<'static, App>) -> Result<Self, Box<dyn std::error::Error>> {
        let (session, notifier) = open_session()?;
        let seat = session.seat();
        tracing::info!(seat = %seat, "acquired a libseat session");
        // The notifier stays registered for the whole process lifetime (unlike
        // the device sources it survives pause/resume), so its token is not
        // kept around.
        handle.insert_source(notifier, |event, _, app: &mut App| {
            on_session_event(app, event)
        })?;

        let mut backend = Self {
            session,
            seat,
            handle,
            sources: Sources::default(),
            pending: None,
            gpu: None,
        };
        backend.attach_input()?;
        backend.pending = backend.open_pending_gpu()?;
        Ok(backend)
    }

    /// libinput (input devices) and a udev monitor for DRM hotplug.
    fn attach_input(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let udev = UdevBackend::new(&self.seat)?;
        let token = self
            .handle
            .insert_source(udev, |event, _, app: &mut App| on_udev_event(app, event))?;
        self.sources.udev = Some(token);

        // libinput opens its devices through our session, which lets logind
        // hand out the input device descriptors without root.
        let mut context = Libinput::new_with_udev(LibinputSessionInterface::from(self.session.clone()));
        context
            .udev_assign_seat(&self.seat)
            .map_err(|_| format!("libinput could not take seat {}", self.seat))?;
        let token = self.handle.insert_source(
            LibinputInputBackend::new(context),
            |event, _, app: &mut App| on_input_event(app, event),
        )?;
        self.sources.libinput = Some(token);
        Ok(())
    }

    /// Publish the opened GPU to the shell: assign the CRTC, set the connector
    /// mode on the output and start scanning out.
    fn attach_output(&mut self, state: &mut State) -> Result<(), Box<dyn std::error::Error>> {
        let Some(pending) = self.pending.take() else {
            return Ok(());
        };
        let PendingGpu {
            path,
            dev_id,
            device,
            notifier,
            gbm,
            mut renderer,
            connector,
            crtc,
            mode,
        } = pending;

        // The output must carry the real mode before the DRM compositor starts
        // tracking it: `OutputModeSource` reads the mode of the Output.
        state.set_output_mode(to_output_mode(mode));

        let allocator = GbmAllocator::new(
            gbm.clone(),
            GbmBufferFlags::RENDERING | GbmBufferFlags::SCANOUT,
        );
        let exporter = GbmFramebufferExporter::new(gbm.clone(), None);
        let formats = renderer.dmabuf_formats().iter().copied().collect::<Vec<_>>();
        let mut manager: Manager = DrmOutputManager::new(
            device,
            allocator,
            exporter,
            Some(gbm.clone()),
            [Fourcc::Argb8888, Fourcc::Xrgb8888],
            formats,
        );

        // The initial commit paints black; the first real frame follows right
        // after through the normal render path.
        let mut elements: DrmOutputRenderElements<GlesRenderer, ui::UiElement> =
            DrmOutputRenderElements::new();
        elements.add_output(&crtc, smithay::backend::renderer::Color32F::BLACK, std::iter::empty());

        let drm_output = manager.initialize_output(
            crtc,
            mode,
            &[connector],
            OutputModeSource::from(&state.output),
            None,
            &mut renderer,
            &elements,
        )?;
        tracing::info!(
            device = %path.display(),
            connector = ?connector,
            mode = ?mode.name(),
            "output initialized"
        );

        let token = self.handle.insert_source(notifier, |event, _, app: &mut App| {
            on_drm_event(app, event)
        })?;
        self.sources.drm = Some(token);
        self.gpu = Some(Gpu {
            path,
            dev_id,
            _manager: manager,
            renderer,
            output: OutputState {
                drm: drm_output,
                crtc,
                pending: false,
            },
        });
        Ok(())
    }

    /// Try every DRM card until one has a connected connector.
    fn open_pending_gpu(&mut self) -> Result<Option<PendingGpu>, Box<dyn std::error::Error>> {
        let mut errors = Vec::new();
        let candidates = self.candidates();
        for path in candidates {
            match self.try_open_gpu(&path) {
                Ok(Some(gpu)) => return Ok(Some(gpu)),
                Ok(None) => {
                    tracing::debug!(device = %path.display(), "no connected connector");
                }
                Err(err) => {
                    tracing::debug!(device = %path.display(), %err, "cannot drive this device");
                    errors.push(format!("{}: {err}", path.display()));
                }
            }
        }
        if errors.is_empty() {
            Err("no DRM card with a connected connector was found".into())
        } else {
            Err(format!("no usable DRM device: {}", errors.join("; ")).into())
        }
    }

    /// Device paths to try, the firmware-selected primary GPU first.
    ///
    /// `MATERIALAND_DRM_DEVICE` (e.g. `/dev/dri/card1`) forces a specific card
    /// to the front of the list, which is the escape hatch on machines where
    /// the wrong GPU is picked; the remaining candidates are still tried if it
    /// cannot be driven.
    fn candidates(&self) -> Vec<PathBuf> {
        let mut list = Vec::new();
        if let Some(forced) = std::env::var_os("MATERIALAND_DRM_DEVICE") {
            list.push(PathBuf::from(forced));
        }
        if let Ok(Some(primary)) = udev::primary_gpu(&self.seat) {
            if !list.contains(&primary) {
                list.push(primary);
            }
        }
        if let Ok(all) = udev::all_gpus(&self.seat) {
            for path in all {
                if !list.contains(&path) {
                    list.push(path);
                }
            }
        }
        list
    }

    /// Open one card, create the renderer and pick the connector to drive.
    /// `Ok(None)` means the device works but has nothing plugged in.
    fn try_open_gpu(&mut self, path: &Path) -> Result<Option<PendingGpu>, Box<dyn std::error::Error>> {
        let fd = self
            .session
            .open(path, OFlags::RDWR | OFlags::CLOEXEC | OFlags::NOCTTY)
            .map_err(|err| format!("cannot open the device: {err:?}"))?;

        // `DrmDeviceFd` is cloneable and shares one fd (and one DRM master)
        // between KMS and GBM; creating a second `DrmDeviceFd` from the raw fd
        // instead would drop the master lock twice.
        let drm_fd = DrmDeviceFd::new(DeviceFd::from(fd));
        let (device, notifier) = DrmDevice::new(drm_fd.clone(), false)?;

        let Some((connector, crtc, mode)) = pick_connector(&device) else {
            return Ok(None);
        };

        let gbm = GbmDevice::new(drm_fd.clone())?;
        let renderer = create_renderer(gbm.clone())?;
        let dev_id = std::fs::metadata(path)
            .map(|meta| std::os::unix::fs::MetadataExt::rdev(&meta))
            .unwrap_or_default();

        Ok(Some(PendingGpu {
            path: path.to_path_buf(),
            dev_id,
            device,
            notifier,
            gbm,
            renderer,
            connector,
            crtc,
            mode,
        }))
    }

    /// Drop the active output and device, e.g. when the session pauses.
    fn drop_gpu(&mut self) {
        if let Some(token) = self.sources.drm.take() {
            self.handle.remove(token);
        }
        if let Some(gpu) = self.gpu.take() {
            tracing::debug!(device = %gpu.path.display(), "releasing the DRM device");
        }
        self.pending = None;
    }

    /// Re-open devices after a pause or when the active card disappeared.
    fn resume(&mut self, state: &mut State) {
        if self.gpu.is_some() || self.pending.is_some() {
            return;
        }
        if self.sources.udev.is_none() {
            if let Err(err) = self.attach_input() {
                tracing::error!(%err, "failed to reattach input devices");
                return;
            }
        }
        match self.open_pending_gpu() {
            Ok(pending) => self.pending = pending,
            Err(err) => {
                tracing::error!(%err, "failed to re-open a DRM device");
                return;
            }
        }
        if let Err(err) = self.attach_output(state) {
            tracing::error!(%err, "failed to re-initialize the output");
            self.pending = None;
            self.gpu = None;
        }
    }

    /// Suspend everything the session owns: close devices, release DRM master.
    fn pause(&mut self) {
        if let Some(token) = self.sources.udev.take() {
            self.handle.remove(token);
        }
        if let Some(token) = self.sources.libinput.take() {
            self.handle.remove(token);
        }
        self.drop_gpu();
        tracing::info!("session paused");
    }

    /// dmabuf formats the renderer can import; the compositor advertises them
    /// through `zwp_linux_dmabuf_v1`.
    pub fn dmabuf_formats(&self) -> Vec<smithay::backend::allocator::Format> {
        let renderer = match self.pending.as_ref() {
            Some(pending) => &pending.renderer,
            None => match self.gpu.as_ref() {
                Some(gpu) => &gpu.renderer,
                None => return Vec::new(),
            },
        };
        renderer.dmabuf_formats().iter().copied().collect()
    }
}

/// Build the GLES renderer on top of a GBM device (EGL platform GBM).
fn create_renderer(gbm: GbmDevice<DrmDeviceFd>) -> Result<GlesRenderer, Box<dyn std::error::Error>> {
    // SAFETY: the GBM device fd stays alive for at least as long as the EGL
    // display and there is only one EGL display per GBM device in this process.
    let display = unsafe { EGLDisplay::new(gbm)? };
    let context = EGLContext::new(&display)?;
    // SAFETY: the context is fresh and not current in any other thread.
    let renderer = unsafe { GlesRenderer::new(context)? };
    Ok(renderer)
}

/// The output mode to drive a connector with: its preferred mode, falling back
/// to the first one it reports.
fn pick_mode(connector: &connector::Info) -> Option<control::Mode> {
    connector
        .modes()
        .iter()
        .find(|mode| mode.mode_type().contains(ModeTypeFlags::PREFERRED))
        .or_else(|| connector.modes().first())
        .copied()
}

/// Pick the first connected connector, a CRTC it can use and a mode.
fn pick_connector(
    device: &DrmDevice,
) -> Option<(connector::Handle, crtc::Handle, control::Mode)> {
    let resources = device.resource_handles().ok()?;
    for connector_handle in resources.connectors() {
        let Ok(info) = device.get_connector(*connector_handle, false) else {
            continue;
        };
        if info.state() != connector::State::Connected {
            continue;
        }
        let Some(mode) = pick_mode(&info) else {
            continue;
        };
        let crtc = info
            .encoders()
            .iter()
            .filter_map(|encoder| device.get_encoder(*encoder).ok())
            .flat_map(|encoder| resources.filter_crtcs(encoder.possible_crtcs()))
            .next();
        if let Some(crtc) = crtc {
            return Some((*connector_handle, crtc, mode));
        }
    }
    None
}

fn refresh_millihz(refresh_hz: u32) -> i32 {
    (refresh_hz as i32).saturating_mul(1_000)
}

fn to_output_mode(mode: control::Mode) -> OutputMode {
    let (w, h) = mode.size();
    OutputMode {
        size: Size::from((w as i32, h as i32)),
        // wl_output.mode.refresh is expressed in millihertz, not hertz.
        refresh: refresh_millihz(mode.vrefresh()),
    }
}

fn is_card(path: &Path) -> bool {
    path.parent().map(|dir| dir == Path::new("/dev/dri")).unwrap_or(false)
        && path
            .file_name()
            .and_then(|name| name.to_str())
            .map(|name| name.starts_with("card"))
            .unwrap_or(false)
}

fn on_session_event(app: &mut App, event: SessionEvent) {
    match event {
        SessionEvent::PauseSession => {
            // The kernel revokes our devices (VT switch, lock): drop them.
            app.backend.pause();
        }
        SessionEvent::ActivateSession => {
            app.backend.resume(&mut app.state);
            app.request_redraw();
        }
    }
}

fn on_udev_event(app: &mut App, event: UdevEvent) {
    match event {
        UdevEvent::Added { path, .. } => {
            if !is_card(&path) {
                return;
            }
            if app.backend.gpu.is_some() || app.backend.pending.is_some() {
                return;
            }
            tracing::info!(device = %path.display(), "DRM device appeared, trying to drive it");
            app.backend.resume(&mut app.state);
            app.request_redraw();
        }
        UdevEvent::Removed { device_id } => {
            let active = app
                .backend
                .gpu
                .as_ref()
                .map(|gpu| gpu.dev_id)
                .or_else(|| app.backend.pending.as_ref().map(|gpu| gpu.dev_id));
            if active != Some(device_id) {
                return;
            }
            tracing::warn!("the DRM device in use disappeared");
            app.backend.drop_gpu();
            app.backend.resume(&mut app.state);
            app.request_redraw();
        }
        UdevEvent::Changed { .. } => {}
    }
}

fn on_drm_event(app: &mut App, event: DrmEvent) {
    match event {
        DrmEvent::VBlank(crtc) => on_vblank(app, crtc),
        DrmEvent::Error(err) => tracing::error!(?err, "DRM device error"),
    }
}

fn on_vblank(app: &mut App, crtc: crtc::Handle) {
    let Some(gpu) = app.backend.gpu.as_mut() else {
        return;
    };
    if gpu.output.crtc != crtc || !gpu.output.pending {
        return;
    }
    if let Err(err) = gpu.output.drm.frame_submitted() {
        tracing::warn!(?err, "failed to acknowledge the submitted frame");
    }
    gpu.output.pending = false;

    // The frame is on screen now: release client frame callbacks and, if the
    // shell is still changing, render the next frame right away so animations
    // follow the panel refresh instead of the timer.
    ui::send_frames_all(&app.state);
    if let Err(err) = app.display.flush_clients() {
        tracing::warn!(?err, "failed to flush client events after vblank");
    }
    if app.dirty || app.animating {
        render(app);
    }
}

fn on_input_event(app: &mut App, event: InputEvent<LibinputInputBackend>) {
    let state = &mut app.state;
    match event {
        InputEvent::Keyboard { event } => input::on_keyboard(state, &event),
        InputEvent::PointerMotion { event } => {
            input::on_pointer_motion_relative(
                state,
                event.delta_x(),
                event.delta_y(),
                event.time_msec(),
            );
        }
        InputEvent::PointerMotionAbsolute { event } => {
            let loc = event.position_transformed(state.output_size);
            input::on_pointer_motion(state, loc, event.time_msec());
        }
        InputEvent::PointerButton { event } => input::on_pointer_button(state, &event),
        InputEvent::PointerAxis { event } => {
            input::on_pointer_axis::<LibinputInputBackend>(state, event);
        }
        // Touch, tablet and gesture events are not wired into the shell yet;
        // the nested backend has the same gap.
        _ => return,
    }
    app.dirty = true;
    app.schedule(FRAME_BUDGET);
}

/// Render one frame into the primary plane of the active output.
fn render(app: &mut App) {
    let App {
        state,
        backend,
        dirty,
        ..
    } = app;
    let Some(gpu) = backend.gpu.as_mut() else {
        return;
    };
    if gpu.output.pending {
        return;
    }
    let size = Size::<i32, Physical>::from((state.output_size.w, state.output_size.h));
    if size.w <= 0 || size.h <= 0 {
        return;
    }

    let elements = ui::build_ui(state, &mut gpu.renderer, size);
    let clear = smithay::backend::renderer::Color32F::new(0.0, 0.0, 0.0, 1.0);
    let result = match gpu.output.drm.render_frame(
        &mut gpu.renderer,
        &elements,
        clear,
        FrameFlags::DEFAULT,
    ) {
        Ok(result) => result,
        Err(err) => {
            tracing::error!(?err, "failed to render a frame");
            return;
        }
    };

    if result.is_empty {
        // Nothing changed since the last commit; no need to submit.
        *dirty = false;
        return;
    }
    if result.needs_sync() {
        // The renderer could not hand out a fence for the primary plane
        // buffer; wait for the GPU before the flip instead.
        if let PrimaryPlaneElement::Swapchain(element) = &result.primary_element {
            if element.sync.wait().is_err() {
                tracing::warn!("interrupted while waiting for the render fence");
            }
        }
    }
    // Dropping the result releases the swapchain slot, which `queue_frame`
    // needs, so it must happen before the frame is queued.
    drop(result);

    match gpu.output.drm.queue_frame(()) {
        Ok(()) => {
            gpu.output.pending = true;
            *dirty = false;
        }
        Err(FrameError::EmptyFrame) => *dirty = false,
        Err(err) => tracing::error!(?err, "failed to queue the frame"),
    }
}

/// Run the compositor as a real session until asked to quit.
pub fn run(
    mut event_loop: EventLoop<'static, App>,
    display: Display<State>,
    state: State,
    listener: ListeningSocket,
    ticker: ui::Ticker,
    backend: Backend,
) -> Result<(), Box<dyn std::error::Error>> {
    let handle = event_loop.handle();

    let mut app = App {
        display,
        state,
        listener,
        backend,
        ticker,
        dirty: true,
        animating: false,
        timer: None,
    };

    // Wayland clients: the display socket and the listening socket are both
    // level-triggered so a wake-up always drains every pending event.
    let display_fd = app.display.as_fd().try_clone_to_owned()?;
    handle.insert_source(
        Generic::new(display_fd, calloop::Interest::READ, calloop::Mode::Level),
        |_readiness, _fd, app: &mut App| {
            let dispatched = match app.display.dispatch_clients(&mut app.state) {
                Ok(dispatched) => dispatched,
                Err(err) => {
                    tracing::error!(?err, "failed to dispatch client requests");
                    return Ok(PostAction::Continue);
                }
            };
            if let Err(err) = app.display.flush_clients() {
                tracing::error!(?err, "failed to flush client events");
            }
            app.state.space.refresh();
            app.state.popup_manager.cleanup();
            if dispatched > 0 {
                app.request_redraw();
            }
            Ok(PostAction::Continue)
        },
    )?;

    let listener_fd = app.listener.as_fd().try_clone_to_owned()?;
    handle.insert_source(
        Generic::new(listener_fd, calloop::Interest::READ, calloop::Mode::Level),
        |_readiness, _fd, app: &mut App| {
            loop {
                match app.listener.accept() {
                    Ok(Some(stream)) => {
                        tracing::info!("client connected");
                        if let Err(err) = app
                            .display
                            .handle()
                            .insert_client(stream, std::sync::Arc::new(ClientState::default()))
                        {
                            tracing::warn!(?err, "failed to insert the client");
                        }
                    }
                    Ok(None) => break,
                    Err(err) => {
                        tracing::warn!(?err, "failed to accept a client");
                        break;
                    }
                }
            }
            Ok(PostAction::Continue)
        },
    )?;

    // Ctrl-C / SIGTERM: leave the seat and exit.
    let signals = Signals::new(&[Signal::SIGINT, Signal::SIGTERM])?;
    handle.insert_source(signals, |signal, _, app: &mut App| {
        tracing::info!(?signal, "terminating");
        app.state.running = false;
    })?;

    // Publish the connector mode and mode-set the output.
    app.backend.attach_output(&mut app.state)?;
    app.request_redraw();

    let socket_name = std::env::var("WAYLAND_DISPLAY")
        .unwrap_or_else(|_| crate::config::SOCKET_NAME.to_owned());
    tracing::info!("compositor ready on WAYLAND_DISPLAY={socket_name}");
    while app.state.running {
        // Wake-ups come from the sockets, the devices and the shell timer.
        if let Err(err) = event_loop.dispatch(None::<Duration>, &mut app) {
            tracing::error!(?err, "event loop error");
            std::thread::sleep(Duration::from_millis(16));
        }
    }
    // Dropping the backend closes the DRM device, which releases DRM master
    // and lets the display manager restore the console.
    Ok(())
}

/// `materialand --check`: report which connectors and modes the KMS backend
/// would drive, without touching the display.
///
/// The devices are opened read-only through the session and no CRTC is
/// programmed, so it is safe to run inside a running graphical session.
pub fn probe() -> Result<(), Box<dyn std::error::Error>> {
    let (mut session, _notifier) = open_session()?;
    let seat = session.seat();
    println!("seat: {seat}");

    let mut cards = Vec::new();
    if let Ok(Some(primary)) = udev::primary_gpu(&seat) {
        println!("primary GPU: {}", primary.display());
        cards.push(primary);
    }
    for path in udev::all_gpus(&seat)? {
        if !cards.contains(&path) {
            cards.push(path);
        }
    }

    for path in cards {
        println!("\n{}:", path.display());
        let fd = match session.open(&path, OFlags::RDONLY | OFlags::CLOEXEC | OFlags::NOCTTY) {
            Ok(fd) => fd,
            Err(err) => {
                println!("  cannot open: {err:?}");
                continue;
            }
        };
        let device = probe_device::ProbeDevice::new(fd);
        let resources = match device.resource_handles() {
            Ok(resources) => resources,
            Err(err) => {
                println!("  cannot query resources: {err}");
                continue;
            }
        };
        for handle in resources.connectors() {
            match device.get_connector(*handle, false) {
                Ok(info) => {
                    println!(
                        "  connector {handle:?}: {:?} {:?}",
                        info.interface(),
                        info.state()
                    );
                    let mode = pick_mode(&info);
                    match mode {
                        Some(mode) => {
                            let (w, h) = mode.size();
                            println!("    mode: {w}x{h}@{}", mode.vrefresh());
                        }
                        None => println!("    no modes reported"),
                    }
                }
                Err(err) => println!("  connector {handle:?}: {err}"),
            }
        }
    }
    Ok(())
}

/// Read-only DRM device used by `--check`.
mod probe_device {
    use std::os::fd::{AsFd, BorrowedFd, OwnedFd};

    /// Marker impls only: every method comes from the `drm` crate's trait
    /// providers, which issue pure getter ioctls that need no DRM master.
    pub struct ProbeDevice(OwnedFd);

    impl ProbeDevice {
        pub fn new(fd: OwnedFd) -> Self {
            Self(fd)
        }
    }

    impl AsFd for ProbeDevice {
        fn as_fd(&self) -> BorrowedFd<'_> {
            self.0.as_fd()
        }
    }

    impl smithay::reexports::drm::Device for ProbeDevice {}
    impl smithay::reexports::drm::control::Device for ProbeDevice {}
}


#[cfg(test)]
mod output_mode_tests {
    use super::refresh_millihz;

    #[test]
    fn output_refresh_is_reported_in_millihertz() {
        assert_eq!(refresh_millihz(60), 60_000);
        assert_eq!(refresh_millihz(144), 144_000);
        assert_eq!(refresh_millihz(0), 0);
    }
}
