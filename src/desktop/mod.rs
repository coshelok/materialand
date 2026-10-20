//! Application state, window ownership and protocol boundary.
use std::collections::{HashMap, HashSet};
use std::time::Instant;

use smithay::{
    backend::allocator::{dmabuf::Dmabuf, Format},
    desktop::{
        find_popup_root_surface, get_popup_toplevel_coords, PopupKeyboardGrab, PopupKind,
        PopupManager, PopupPointerGrab, PopupUngrabStrategy, Window,
    },
    input::{
        keyboard::{KeyboardHandle, XkbConfig},
        pointer::{Focus, PointerHandle},
        Seat, SeatHandler, SeatState,
    },
    output::{Mode, Output, PhysicalProperties, Scale, Subpixel},
    reexports::wayland_server::{
        backend::GlobalId,
        protocol::{wl_seat, wl_surface::WlSurface},
    },
    utils::{Logical, Point, Rectangle, Serial, Size, Transform},
    wayland::{
        buffer::BufferHandler,
        compositor::{self, CompositorClientState, CompositorHandler, CompositorState},
        cursor_shape::CursorShapeManagerState,
        dmabuf::{DmabufGlobal, DmabufHandler, DmabufState, ImportNotifier},
        fractional_scale::{self, FractionalScaleHandler, FractionalScaleManagerState},
        output::OutputHandler,
        seat::WaylandFocus,
        selection::{
            data_device::{
                ClientDndGrabHandler, DataDeviceHandler, DataDeviceState, ServerDndGrabHandler,
            },
            SelectionHandler,
        },
        shell::xdg::{
            decoration::{XdgDecorationHandler, XdgDecorationState},
            PopupSurface, PositionerState, ToplevelSurface, XdgShellHandler, XdgShellState,
            XdgToplevelSurfaceData,
        },
        shm::{ShmHandler, ShmState},
        tablet_manager::TabletSeatHandler,
        viewporter::ViewporterState,
        xdg_activation::{
            XdgActivationHandler, XdgActivationState, XdgActivationToken, XdgActivationTokenData,
        },
    },
};

use crate::{
    config,
    graphics::text::Fonts,
    shell::Shell,
    theme::{ColorVariant, Theme, ThemeSpec},
};

/// A left/right/top/bottom resize edge set.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ResizeEdges {
    pub left: bool,
    pub right: bool,
    pub top: bool,
    pub bottom: bool,
}

/// In-flight pointer interaction owned by the compositor (not the client).
#[derive(Clone, Debug)]
pub enum Interaction {
    None,
    SplitDivider,
    Move {
        button: u32,
        window: Window,
        start_pointer: Point<f64, Logical>,
        start_location: Point<i32, Logical>,
    },
    Resize {
        button: u32,
        window: Window,
        edges: ResizeEdges,
        start_pointer: Point<f64, Logical>,
        start_geometry: Rectangle<i32, Logical>,
    },
    /// Dragging the brightness slider in the top-bar popup.
    Brightness,
    ShadeBrightness,
}

impl Interaction {
    pub fn button(&self) -> Option<u32> {
        match self {
            Self::Move { button, .. } | Self::Resize { button, .. } => Some(*button),
            Self::Brightness | Self::ShadeBrightness | Self::SplitDivider => Some(0x110),
            Self::None => None,
        }
    }

    /// The window this interaction is attached to, if any.
    pub fn window(&self) -> Option<&Window> {
        match self {
            Interaction::Move { window, .. } | Interaction::Resize { window, .. } => Some(window),
            Interaction::None
            | Interaction::Brightness
            | Interaction::ShadeBrightness
            | Interaction::SplitDivider => None,
        }
    }
}

/// Metadata tracked for each mapped toplevel.
#[derive(Clone, Debug, Default)]
pub struct WindowMeta {
    pub title: String,
    pub app_id: String,
}

/// Persistent window-management state (independent of transient metadata).
#[derive(Clone, Copy, Debug, Default)]
pub struct WindowProps {
    pub maximized: bool,
    pub fullscreen: bool,
    pub minimized: bool,
    /// Geometry of the normal (unmaximized, non-fullscreen) window.
    pub saved: Option<Rectangle<i32, Logical>>,
    /// Whether leaving fullscreen should return to the maximized state.
    pub restore_maximized: bool,
    /// Mapped location is lost when the window is removed from `Space`.
    pub minimized_location: Option<Point<i32, Logical>>,
}

pub struct State {
    pub forwarded_buttons: HashSet<u32>,
    pub cursor_status: smithay::input::pointer::CursorImageStatus,
    pub pending_imports: Vec<(Dmabuf, ImportNotifier)>,
    advertised_formats: Vec<Format>,
    retired_dmabuf_globals: Vec<DmabufGlobal>,
    pub windows: WindowManager,
    pub overview: overview::Overview,
    pub services: crate::system::Services,
    pub notifications: crate::notifications::Notifications,
    pub shortcut_drag: Option<crate::input::ShortcutDrag>,
    pub split: Option<split::Split>,
    pub display: DisplayModel,
    pub session_mode: bool,
    pub display_handle: smithay::reexports::wayland_server::DisplayHandle,
    pub compositor_state: CompositorState,
    pub xdg_shell_state: XdgShellState,
    pub popup_manager: PopupManager,
    pub shm_state: ShmState,
    pub seat_state: SeatState<Self>,
    pub data_device_state: DataDeviceState,
    pub dmabuf_state: DmabufState,
    /// Kept alive for the lifetime of the compositor; the `GlobalId` inside
    /// unregisters the `zwp_linux_dmabuf_v1` global when dropped.
    _dmabuf_global: DmabufGlobal,
    activation_state: XdgActivationState,

    pub pointer: PointerHandle<Self>,
    pub keyboard: KeyboardHandle<Self>,

    _output_global: GlobalId,
    /// Disabled outputs remain bindable for clients with registry events in flight.
    retired_output_globals: Vec<GlobalId>,

    pub shell: Shell,
    pub fonts: Fonts,
    pub theme: Theme,
    /// Palette seed + Material You variant the current `theme` was built from.
    pub theme_spec: ThemeSpec,
    appearance_store: config::AppearanceStore,

    pub running: bool,
    pub start_time: Instant,

    pub cursor: Point<f64, Logical>,
    pub interaction: Interaction,
    pub(crate) launcher_swipe: crate::input::LauncherSwipe,
    /// Hardware backlight fraction, independent of the rendered scene.
    pub brightness: f32,
    /// Latest pointer-button serial that was actually delivered to a client.
    /// XDG interactive move/resize requests must match this serial and window.
    pub last_interactive_press: Option<(Serial, Window)>,
    /// Non-primary button presses swallowed by compositor chrome; their release
    /// must be swallowed too, otherwise clients may receive an unmatched release.
    pub swallowed_buttons: HashSet<u32>,
    /// Keys whose press was consumed by the shell; consume their releases too.
    pub swallowed_keys: HashSet<smithay::input::keyboard::Keycode>,
}

mod appearance;
mod focus;
mod geometry;
mod init;
mod lifecycle;
mod metadata;
mod output;
mod popups;
mod window_modes;
use metadata::default_meta;
mod protocol;
#[derive(Default)]
pub struct ClientState {
    pub compositor_state: CompositorClientState,
}

impl smithay::reexports::wayland_server::backend::ClientData for ClientState {}

smithay::delegate_xdg_shell!(State);
smithay::delegate_compositor!(State);
smithay::delegate_shm!(State);
smithay::delegate_seat!(State);
smithay::delegate_data_device!(State);
smithay::delegate_output!(State);
smithay::delegate_xdg_decoration!(State);
smithay::delegate_dmabuf!(State);
smithay::delegate_xdg_activation!(State);
smithay::delegate_cursor_shape!(State);
smithay::delegate_fractional_scale!(State);
smithay::delegate_viewporter!(State);

#[cfg(test)]
mod tests;

mod ownership;
pub use ownership::{DisplayModel, WindowManager};

pub mod resize;

pub mod hit_test;

mod buffers;

mod coordinates;

mod placement;

pub mod overview;
pub mod split;

#[cfg(test)]
mod integration_tests;
