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
//!   backend, driven by input, client commits and output page flips,
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
            Event as SessionEvent, Session,
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

use crate::{desktop::State, graphics::scene as ui, input};

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
    ticker: crate::app::Ticker,
    /// The shell changed and wants a frame.
    dirty: bool,
    /// Shell animations are still running; keep scheduling frames.
    animating: bool,
    /// One-shot timer that coalesces redraw requests, together with the
    /// deadline it was armed for. Keeping the deadline lets a later request
    /// replace a pending idle tick with an earlier frame.
    timer: Option<(RegistrationToken, Instant)>,
}

/// Everything needed to drive one GPU: the output manager (kept for its
/// lifetime: it owns the DRM device and will be needed for mode changes),
/// the renderer and the active output.
struct Gpu {
    night_light: Option<color::NightLight>,
    backlight: Option<backlight::Backlight>,
    path: PathBuf,
    dev_id: u64,
    /// Frames go straight through [`OutputState::drm`]; never read directly.
    /// The manager is kept alive for the DRM device it owns (and for future
    /// mode changes), hence the underscore: ownership is the point.
    output: OutputState,
    renderer: GlesRenderer,
    manager: Manager,
}

struct OutputState {
    drm: Output,
    crtc: crtc::Handle,
    connector: connector::Handle,
    mode: control::Mode,
    frame: FrameState,
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

pub struct Backend {
    active: bool,
    session: LibSeatSession,
    seat: String,
    handle: LoopHandle<'static, App>,
    sources: Sources,
    /// GPU opened in [`Backend::new`], waiting for the shell state.
    pending: Option<PendingGpu>,
    gpu: Option<Gpu>,
}

mod device;
mod output;
mod scheduler;
mod seat;
use seat::*;
mod modes;
use modes::*;
mod events;
use events::*;
mod presentation;
use presentation::*;
mod event_loop;
mod probe;
pub use event_loop::run;
pub use probe::probe;

mod frame_state;
use frame_state::FrameState;

mod backlight;
mod color;
