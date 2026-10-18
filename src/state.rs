use std::collections::{HashMap, HashSet};
use std::time::Instant;

use smithay::{
    backend::allocator::{dmabuf::Dmabuf, Format},
    desktop::{
        find_popup_root_surface, get_popup_toplevel_coords, PopupKeyboardGrab, PopupKind,
        PopupManager, PopupPointerGrab, PopupUngrabStrategy, Window,
    },
    input::{
        keyboard::{KeyboardHandle, Keysym, XkbConfig},
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
        seat::WaylandFocus,
        output::OutputHandler,
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
    font::Fonts,
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
    Move {
        window: Window,
        start_pointer: Point<f64, Logical>,
        start_location: Point<i32, Logical>,
    },
    Resize {
        window: Window,
        edges: ResizeEdges,
        start_pointer: Point<f64, Logical>,
        start_geometry: Rectangle<i32, Logical>,
    },
    /// Dragging the brightness slider in the top-bar popup.
    Brightness,
}

impl Interaction {
    /// The window this interaction is attached to, if any.
    pub fn window(&self) -> Option<&Window> {
        match self {
            Interaction::Move { window, .. } | Interaction::Resize { window, .. } => Some(window),
            Interaction::None | Interaction::Brightness => None,
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

    pub space: smithay::desktop::space::Space<Window>,
    pub output: Output,
    _output_global: GlobalId,

    pub shell: Shell,
    pub fonts: Fonts,
    pub theme: Theme,
    /// Palette seed + Material You variant the current `theme` was built from.
    pub theme_spec: ThemeSpec,

    pub running: bool,
    pub start_time: Instant,

    pub output_size: Size<i32, Logical>,
    pub cursor: Point<f64, Logical>,
    pub focused: Option<Window>,
    pub interaction: Interaction,
    /// Output dimming factor in `[0.2, 1.0]`; `1.0` leaves the image untouched.
    pub brightness: f32,
    pub meta: std::collections::HashMap<Window, WindowMeta>,
    pub props: std::collections::HashMap<Window, WindowProps>,
    pub serial: u32,
    /// Latest pointer-button serial that was actually delivered to a client.
    /// XDG interactive move/resize requests must match this serial and window.
    pub last_interactive_press: Option<(Serial, Window)>,
    /// Non-primary button presses swallowed by compositor chrome; their release
    /// must be swallowed too, otherwise clients may receive an unmatched release.
    pub swallowed_buttons: HashSet<u32>,
    /// Keys whose press was consumed by the shell; consume their releases too.
    pub swallowed_keys: HashSet<Keysym>,
    pub cascade: i32,
}

impl State {
    pub fn new(
        display: &mut smithay::reexports::wayland_server::Display<Self>,
        fonts: Fonts,
        apps: Vec<config::AppEntry>,
        dmabuf_formats: Vec<Format>,
        spec: ThemeSpec,
    ) -> Self {
        let dh = display.handle();
        let compositor_state = CompositorState::new::<Self>(&dh);
        // Advertise the common SHM formats used by GTK/Qt and simple clients.
        let shm_state = ShmState::new::<Self>(
            &dh,
            vec![
                smithay::reexports::wayland_server::protocol::wl_shm::Format::Argb8888,
                smithay::reexports::wayland_server::protocol::wl_shm::Format::Xrgb8888,
            ],
        );
        let mut dmabuf_state = DmabufState::new();
        let dmabuf_global = dmabuf_state.create_global::<Self>(&dh, dmabuf_formats);
        let mut seat_state = SeatState::new();
        let mut seat = seat_state.new_wl_seat(&dh, "seat-0");

        let output = Output::new(
            "materialand-0".to_string(),
            PhysicalProperties {
                size: config::OUTPUT_PHYSICAL_MM.into(),
                subpixel: Subpixel::Unknown,
                make: "Materialand".into(),
                model: "Nested".into(),
            },
        );
        let _output_global = output.create_global::<Self>(&dh);
        let mode = Mode {
            size: (1280, 800).into(),
            refresh: 60000,
        };
        output.set_preferred(mode);
        output.change_current_state(
            Some(mode),
            Some(Transform::Normal),
            Some(Scale::Integer(1)),
            Some((0, 0).into()),
        );

        let mut space = smithay::desktop::space::Space::default();
        space.map_output(&output, (0, 0));

        // Keyboard layouts and options: `MATERIALAND_XKB_*`, then the standard
        // `XKB_DEFAULT_*` variables, then the systemd-localed fragment, then
        // evdev/pc105/us (see `config::keyboard_settings`).
        let keyboard_settings = config::keyboard_settings();
        tracing::info!(
            layout = %keyboard_settings.layout,
            variant = %keyboard_settings.variant,
            options = %keyboard_settings.options,
            "keyboard configuration"
        );
        let xkb_config = XkbConfig {
            rules: &keyboard_settings.rules,
            model: &keyboard_settings.model,
            layout: &keyboard_settings.layout,
            variant: &keyboard_settings.variant,
            options: (!keyboard_settings.options.is_empty())
                .then(|| keyboard_settings.options.clone()),
        };
        // A layout name xkbcommon cannot compile (bad system configuration or
        // environment) must not keep the session from starting: fall back to
        // the plain US keymap with a warning instead.
        let keyboard = seat
            .add_keyboard(
                xkb_config,
                config::KEY_REPEAT_DELAY_MS,
                config::KEY_REPEAT_RATE_HZ,
            )
            .or_else(|err| {
                tracing::warn!(
                    ?err,
                    "the configured keymap was rejected; using the default US layout"
                );
                seat.add_keyboard(
                    XkbConfig::default(),
                    config::KEY_REPEAT_DELAY_MS,
                    config::KEY_REPEAT_RATE_HZ,
                )
            })
            .expect("add keyboard");
        let pointer = seat.add_pointer();

        // Registers the `zxdg_decoration_manager_v1` global; the display owns it.
        XdgDecorationState::new::<Self>(&dh);
        // Globals whose state objects carry no per-compositor data: the display
        // owns the global, so the returned handles can be dropped.
        CursorShapeManagerState::new::<Self>(&dh);
        FractionalScaleManagerState::new::<Self>(&dh);
        ViewporterState::new::<Self>(&dh);
        let activation_state = XdgActivationState::new::<Self>(&dh);

        State {
            compositor_state,
            xdg_shell_state: XdgShellState::new::<Self>(&dh),
            popup_manager: PopupManager::default(),
            shm_state,
            seat_state,
            data_device_state: DataDeviceState::new::<Self>(&dh),
            dmabuf_state,
            _dmabuf_global: dmabuf_global,
            activation_state,
            pointer,
            keyboard,
            space,
            output,
            _output_global,
            shell: Shell::new(apps, spec),
            fonts,
            theme: spec.dark(),
            theme_spec: spec,
            running: true,
            start_time: Instant::now(),
            output_size: (1280, 800).into(),
            cursor: (640.0, 400.0).into(),
            focused: None,
            interaction: Interaction::None,
            brightness: 1.0,
            meta: HashMap::new(),
            props: HashMap::new(),
            serial: 0,
            last_interactive_press: None,
            swallowed_buttons: HashSet::new(),
            swallowed_keys: HashSet::new(),
            cascade: 0,
        }
    }

    /// Adopt the mode of the physical connector driven by the KMS backend.
    ///
    /// The nested backend resizes through `update_output`; here the mode comes
    /// from the DRM connector and must be visible to clients (`wl_output`) and
    /// to the shell (layout sizes) before the output starts scanning out.
    pub fn set_output_mode(&mut self, mode: Mode) {
        let logical = mode.size.to_logical(1);
        if self.output_size == logical && self.output.current_mode() == Some(mode) {
            return;
        }
        self.output_size = logical;
        self.output.change_current_state(Some(mode), None, None, None);
        self.output.set_preferred(mode);
        self.shell.mark_dirty();
    }

    /// Switch light/dark with a radial reveal growing out of `origin`.
    pub fn toggle_theme(&mut self, origin: Point<f64, Logical>) {
        // Continue from the palette currently on screen when toggled mid-animation.
        let old = self.shell.effective_theme(&self.theme);
        let dark = !self.theme.is_dark;
        self.theme = if dark {
            self.theme_spec.dark()
        } else {
            self.theme_spec.light()
        };
        self.shell.set_theme_switch(dark);
        self.shell.begin_reveal(old, origin);
        self.prewarm_theme();
    }

    /// Switch the Material You palette variant with a brief crossfade.
    pub fn set_variant(&mut self, variant: ColorVariant) {
        if self.theme_spec.variant == variant {
            return;
        }
        let old = self.shell.effective_theme(&self.theme);
        self.theme_spec.variant = variant;
        self.theme = if self.theme.is_dark {
            self.theme_spec.dark()
        } else {
            self.theme_spec.light()
        };
        self.shell.begin_theme_change(old);
        self.prewarm_theme();
    }

    /// Render the new scheme's expensive layers up front — wallpaper, chrome
    /// and window decorations — so the reveal or crossfade that follows never
    /// has to rasterise them mid-animation.
    fn prewarm_theme(&mut self) {
        let w = self.output_size.w as f32;
        let h = self.output_size.h as f32;
        let theme = self.theme;
        let _ = self.shell.render_wallpaper(
            &theme,
            self.output_size.w as u32,
            self.output_size.h as u32,
            None,
        );
        let items = crate::input::dock_items(self);
        let _ = self.shell.render_topbar(&theme, &mut self.fonts, w, "", None);
        let _ = self
            .shell
            .render_dock(&theme, &mut self.fonts, w, h, &items, self.focused.as_ref(), None);
        if self.shell.quick_settings_open {
            let variant = self.theme_spec.variant;
            let _ = self.shell.render_quick_settings(
                &theme,
                &mut self.fonts,
                w,
                h,
                1.0,
                self.brightness,
                variant,
                None,
            );
        }
        let windows: Vec<_> = self.space.elements().cloned().collect();
        for window in windows {
            let props = self.window_props(&window);
            if props.fullscreen || !self.window_server_decor(&window) {
                continue;
            }
            let geom = self
                .space
                .element_geometry(&window)
                .unwrap_or_else(|| window.geometry());
            let meta = self.window_meta(&window);
            let focused = self.focused.as_ref() == Some(&window);
            let _ = self.shell.render_decoration(
                &theme,
                &mut self.fonts,
                &window,
                geom,
                crate::shell::DecorParams {
                    meta: &meta,
                    focused,
                    maximized: props.maximized,
                },
                None,
            );
        }
    }

    /// Close a window politely by playing its exit animation first.
    ///
    /// The `send_close` is deferred to [`Self::finish_closing`] once the
    /// animation has run, unless reduced motion makes it instant.
    pub fn request_close_window(&mut self, window: &Window) {
        if self.shell.begin_window_close(window) {
            if let Some(t) = window.toplevel() {
                t.send_close();
            }
        }
    }

    /// Send the deferred closes for windows whose exit animation finished.
    pub fn finish_closing(&mut self) {
        for window in self.shell.take_closed_windows() {
            if let Some(t) = window.toplevel() {
                t.send_close();
            }
        }
    }

    pub fn next_serial(&mut self) -> Serial {
        self.serial = self.serial.wrapping_add(1);
        Serial::from(self.serial)
    }

    /// Top-left of a window's surface in output coordinates.
    ///
    /// [`Space`] stores the position of the window *geometry* — the visible
    /// rect that our decorations frame. Clients drawing their own decorations
    /// (GTK windows, for example) inset that rect inside their buffer to leave
    /// room for shadows, so the surface origin sits `geometry().loc` above and
    /// left of it. Rendering (`Window::render_elements`) and hit-testing
    /// (`Window::surface_under`) both work in surface coordinates and share
    /// this helper so the two can never drift apart.
    pub fn window_surface_origin(&self, window: &Window) -> Point<i32, Logical> {
        self.space
            .element_location(window)
            .unwrap_or_default()
            - window.geometry().loc
    }

    pub fn window_meta(&self, window: &Window) -> WindowMeta {
        self.meta
            .get(window)
            .cloned()
            .unwrap_or_else(|| default_meta(window))
    }

    pub fn window_props(&self, window: &Window) -> WindowProps {
        self.props.get(window).copied().unwrap_or_default()
    }

    /// Whether the window has a buffer to show at all. Toplevels are mapped the
    /// moment they are created, but a client may not have attached a buffer yet
    /// and some apps (Electron) open helper toplevels that never draw anything;
    /// such windows must not take a dock slot, let alone paint an empty frame.
    pub fn window_has_content(&self, window: &Window) -> bool {
        let Some(toplevel) = window.toplevel() else {
            return true;
        };
        smithay::backend::renderer::utils::with_renderer_surface_state(
            toplevel.wl_surface(),
            |state| state.buffer().is_some(),
        )
        .unwrap_or(false)
    }

    /// Whether *we* draw the decoration frame for this window.
    ///
    /// A window only gets server-side decorations when its client agreed to
    /// them through `zxdg_decoration_manager_v1`: [`XdgDecorationHandler`]
    /// asks for `ServerSide` and the client commits the same mode back.
    /// Clients that ignore the protocol leave `decoration_mode` as `None` and
    /// draw their own title bar and shadow (Firefox/GTK do exactly this —
    /// their buffer is inset inside the window geometry, which is how
    /// `window_surface_origin` compensates). Painting our frame on top of
    /// theirs would leave a second, empty grey title bar behind the window,
    /// so the frame is drawn only when the client handed decorations over.
    pub fn window_server_decor(&self, window: &Window) -> bool {
        use smithay::reexports::wayland_protocols::xdg::decoration::zv1::server::zxdg_toplevel_decoration_v1::Mode;
        window
            .toplevel()
            .map(|toplevel| toplevel.current_state().decoration_mode == Some(Mode::ServerSide))
            .unwrap_or(false)
    }

    fn props_mut(&mut self, window: &Window) -> &mut WindowProps {
        self.props.entry(window.clone()).or_default()
    }

    /// Locate the mapped window that owns `surface`.
    pub fn find_window(&self, surface: &ToplevelSurface) -> Option<Window> {
        self.space
            .elements()
            .find(|w| w.toplevel().map(|t| t == surface).unwrap_or(false))
            .cloned()
    }

    /// True while a mapped fullscreen window owns the output.
    pub fn has_fullscreen_window(&self) -> bool {
        self.space
            .elements()
            .any(|window| self.window_props(window).fullscreen)
    }

    /// The usable area for maximized windows: below the top bar, above the dock.
    pub fn available_area(&self) -> Rectangle<i32, Logical> {
        let top = (config::TOPBAR_HEIGHT + 2.0 * config::TOPBAR_MARGIN) as i32;
        let bottom = (config::DOCK_HEIGHT + config::DOCK_MARGIN) as i32 + 8;
        let h = (self.output_size.h - top - bottom).max(200);
        Rectangle::new((0, top).into(), (self.output_size.w, h).into())
    }

    /// The full output rectangle (used for fullscreen windows).
    pub fn output_rect(&self) -> Rectangle<i32, Logical> {
        Rectangle::new((0, 0).into(), (self.output_size.w, self.output_size.h).into())
    }

    pub fn focus_window(&mut self, window: Option<Window>, serial: Serial) {
        let previous = self.focused.clone();
        if previous.as_ref() != window.as_ref() {
            if let Some(prev) = previous {
                if prev.set_activated(false) {
                    if let Some(t) = prev.toplevel() {
                        t.send_configure();
                    }
                }
            }
            if let Some(w) = &window {
                if w.set_activated(true) {
                    if let Some(t) = w.toplevel() {
                        t.send_configure();
                    }
                }
            }
            self.focused = window.clone();
        }

        let keyboard = self.keyboard.clone();
        match window.and_then(|w| w.wl_surface().map(|s| s.into_owned())) {
            Some(surface) => keyboard.set_focus(self, Some(surface), serial),
            None => keyboard.set_focus(self, None, serial),
        }
    }

    /// Next cascade position for a newly mapped window, kept inside the output
    /// and clear of the top bar and dock.
    pub fn next_location(&mut self) -> Point<i32, Logical> {
        let n = self.cascade.rem_euclid(6);
        self.cascade = self.cascade.wrapping_add(1);
        let step = config::CASCADE_STEP;
        let base_x = config::TOPBAR_MARGIN as i32 + 48;
        let base_y = config::TOPBAR_HEIGHT as i32 + config::TOPBAR_MARGIN as i32 + 8;
        let (sw, sh) = (self.output_size.w, self.output_size.h);
        let (ww, wh) = config::DEFAULT_WINDOW_SIZE;
        let dock = config::DOCK_HEIGHT as i32 + config::DOCK_MARGIN as i32;
        let max_x = (sw - ww - 24).max(base_x);
        let max_y = (sh - wh - dock - 24).max(base_y);
        Point::from(((base_x + n * step).min(max_x), (base_y + n * step).min(max_y)))
    }

    /// The client surface and surface-local coordinates under the cursor.
    pub fn surface_under_pointer(&self) -> Option<(WlSurface, Point<f64, Logical>)> {
        // A window that is playing its close animation still shows on screen
        // but must not receive any more input, so it is skipped here.
        let windows: Vec<Window> = self.space.elements().cloned().collect();
        for window in windows.into_iter().rev() {
            if self.shell.is_closing(&window) {
                continue;
            }
            let Some(geom) = self.space.element_geometry(&window) else {
                continue;
            };
            let inside = self.cursor.x >= geom.loc.x as f64
                && self.cursor.y >= geom.loc.y as f64
                && self.cursor.x < (geom.loc.x + geom.size.w) as f64
                && self.cursor.y < (geom.loc.y + geom.size.h) as f64;
            if !inside {
                continue;
            }
            let local = self.cursor - self.window_surface_origin(&window).to_f64();
            if let Some((surface, loc)) =
                window.surface_under(local, smithay::desktop::WindowSurfaceType::ALL)
            {
                return Some((surface, loc.to_f64()));
            }
        }
        None
    }
}

impl OutputHandler for State {}

impl BufferHandler for State {
    fn buffer_destroyed(&mut self, _buffer: &smithay::reexports::wayland_server::protocol::wl_buffer::WlBuffer) {
    }
}

impl CompositorHandler for State {
    fn compositor_state(&mut self) -> &mut CompositorState {
        &mut self.compositor_state
    }

    fn client_compositor_state<'a>(
        &self,
        client: &'a smithay::reexports::wayland_server::Client,
    ) -> &'a CompositorClientState {
        &client
            .get_data::<ClientState>()
            .expect("client state")
            .compositor_state
    }

    fn commit(&mut self, surface: &WlSurface) {
        self.popup_manager.commit(surface);
        smithay::backend::renderer::utils::on_commit_buffer_handler::<Self>(surface);
        if let Some(window) = self
            .space
            .elements()
            .find(|w| w.wl_surface().as_deref() == Some(surface))
            .cloned()
        {
            window.on_commit();
        }
        self.space.refresh();
    }
}

impl ShmHandler for State {
    fn shm_state(&self) -> &ShmState {
        &self.shm_state
    }
}

impl SeatHandler for State {
    type KeyboardFocus = WlSurface;
    type PointerFocus = WlSurface;
    type TouchFocus = WlSurface;

    fn seat_state(&mut self) -> &mut SeatState<Self> {
        &mut self.seat_state
    }
}

impl SelectionHandler for State {
    type SelectionUserData = ();
}

impl DataDeviceHandler for State {
    fn data_device_state(&self) -> &DataDeviceState {
        &self.data_device_state
    }
}

impl ClientDndGrabHandler for State {}
impl ServerDndGrabHandler for State {
    fn send(
        &mut self,
        _mime_type: String,
        _fd: std::os::unix::io::OwnedFd,
        _seat: Seat<Self>,
    ) {
    }
}

fn ss_mode() -> smithay::reexports::wayland_protocols::xdg::decoration::zv1::server::zxdg_toplevel_decoration_v1::Mode {
    use smithay::reexports::wayland_protocols::xdg::decoration::zv1::server::zxdg_toplevel_decoration_v1::Mode;
    Mode::ServerSide
}

impl XdgDecorationHandler for State {
    fn new_decoration(&mut self, toplevel: ToplevelSurface) {
        let mode = ss_mode();
        toplevel.with_pending_state(|state| {
            state.decoration_mode = Some(mode);
        });
        toplevel.send_configure();
    }

    fn request_mode(&mut self, toplevel: ToplevelSurface, _mode: smithay::reexports::wayland_protocols::xdg::decoration::zv1::server::zxdg_toplevel_decoration_v1::Mode) {
        let mode = ss_mode();
        toplevel.with_pending_state(|state| {
            state.decoration_mode = Some(mode);
        });
        toplevel.send_configure();
    }

    fn unset_mode(&mut self, _toplevel: ToplevelSurface) {}
}

impl XdgShellHandler for State {
    fn xdg_shell_state(&mut self) -> &mut XdgShellState {
        &mut self.xdg_shell_state
    }

    fn new_toplevel(&mut self, surface: ToplevelSurface) {
        use smithay::reexports::wayland_protocols::xdg::shell::server::xdg_toplevel::State;
        tracing::info!("new toplevel");
        let size = config::DEFAULT_WINDOW_SIZE;
        surface.with_pending_state(|state| {
            state.size = Some(size.into());
            state.states.set(State::Activated);
        });
        surface.send_configure();

        let window = Window::new_wayland_window(surface.clone());
        let meta = default_meta(&window);
        let location = self.next_location();
        self.meta.insert(window.clone(), meta);
        self.props.insert(window.clone(), WindowProps::default());
        self.space.map_element(window.clone(), location, false);
        // Raise only; activation (and the configure it implies) is owned by
        // `focus_window`, otherwise a `true` here would silently deactivate the
        // previous window without ever telling it via a configure.
        self.space.raise_element(&window, false);
        // Play the Material 3 entrance (scale + fade) before the first frame.
        self.shell.on_window_mapped(&window);
        let serial = self.next_serial();
        self.focus_window(Some(window), serial);
    }

    fn move_request(&mut self, surface: ToplevelSurface, _seat: wl_seat::WlSeat, serial: Serial) {
        let Some(window) = self.find_window(&surface) else {
            return;
        };
        let valid_press = self.last_interactive_press.as_ref().is_some_and(|(s, w)| {
            *s == serial && w == &window && self.focused.as_ref() == Some(&window)
        });
        if !valid_press {
            return;
        }
        self.last_interactive_press = None;
        let props = self.window_props(&window);
        if props.fullscreen {
            return;
        }
        let geom = self
            .space
            .element_geometry(&window)
            .unwrap_or_else(|| window.geometry());
        let serial = self.next_serial();
        self.space.raise_element(&window, false);
        self.focus_window(Some(window.clone()), serial);
        self.interaction = Interaction::Move {
            window,
            start_pointer: self.cursor,
            start_location: geom.loc,
        };
    }

    fn resize_request(
        &mut self,
        surface: ToplevelSurface,
        _seat: wl_seat::WlSeat,
        serial: Serial,
        edges: smithay::reexports::wayland_protocols::xdg::shell::server::xdg_toplevel::ResizeEdge,
    ) {
        use smithay::reexports::wayland_protocols::xdg::shell::server::xdg_toplevel::ResizeEdge;
        let Some(window) = self.find_window(&surface) else {
            return;
        };
        let valid_press = self.last_interactive_press.as_ref().is_some_and(|(s, w)| {
            *s == serial && w == &window && self.focused.as_ref() == Some(&window)
        });
        if !valid_press {
            return;
        }
        self.last_interactive_press = None;
        let props = self.window_props(&window);
        if props.fullscreen || props.maximized {
            return;
        }
        let edges = ResizeEdges {
            left: matches!(edges, ResizeEdge::Left | ResizeEdge::TopLeft | ResizeEdge::BottomLeft),
            right: matches!(edges, ResizeEdge::Right | ResizeEdge::TopRight | ResizeEdge::BottomRight),
            top: matches!(edges, ResizeEdge::Top | ResizeEdge::TopLeft | ResizeEdge::TopRight),
            bottom: matches!(
                edges,
                ResizeEdge::Bottom | ResizeEdge::BottomLeft | ResizeEdge::BottomRight
            ),
        };
        if edges == ResizeEdges::default() {
            return;
        }
        let geom = self
            .space
            .element_geometry(&window)
            .unwrap_or_else(|| window.geometry());
        let serial = self.next_serial();
        self.space.raise_element(&window, false);
        self.focus_window(Some(window.clone()), serial);
        self.interaction = Interaction::Resize {
            window,
            edges,
            start_pointer: self.cursor,
            start_geometry: geom,
        };
    }

    fn maximize_request(&mut self, surface: ToplevelSurface) {
        if let Some(window) = self.find_window(&surface) {
            self.set_maximized(&window, true);
        }
    }

    fn unmaximize_request(&mut self, surface: ToplevelSurface) {
        if let Some(window) = self.find_window(&surface) {
            self.set_maximized(&window, false);
        }
    }

    fn fullscreen_request(
        &mut self,
        surface: ToplevelSurface,
        _output: Option<smithay::reexports::wayland_server::protocol::wl_output::WlOutput>,
    ) {
        if let Some(window) = self.find_window(&surface) {
            self.set_fullscreen(&window, true);
        }
    }

    fn unfullscreen_request(&mut self, surface: ToplevelSurface) {
        if let Some(window) = self.find_window(&surface) {
            self.set_fullscreen(&window, false);
        }
    }

    fn minimize_request(&mut self, surface: ToplevelSurface) {
        if let Some(window) = self.find_window(&surface) {
            self.set_minimized(&window, true);
        }
    }

    fn new_popup(&mut self, surface: PopupSurface, _positioner: PositionerState) {
        if let Err(error) = self.popup_manager.track_popup(PopupKind::Xdg(surface.clone())) {
            tracing::warn!(?error, "could not register XDG popup surface");
            return;
        }
        // Clamp the initial placement to the output instead of accepting the
        // positioner's unconstrained geometry (menus at screen edges).
        self.unconstrain_popup(&surface);
        let _ = surface.send_configure();
    }

    fn grab(&mut self, surface: PopupSurface, seat: wl_seat::WlSeat, serial: Serial) {
        // `xdg_popup.grab`: hold the keyboard and pointer on the popup until
        // it is dismissed, so client menus keep input inside their surfaces.
        let Some(seat) = Seat::from_resource(&seat) else {
            return;
        };
        let kind = PopupKind::Xdg(surface);
        let Ok(root) = find_popup_root_surface(&kind) else {
            return;
        };
        match self.popup_manager.grab_popup(root, kind, &seat, serial) {
            Ok(mut grab) => {
                if let Some(keyboard) = seat.get_keyboard() {
                    // Never steal an unrelated grab that is already active.
                    if keyboard.is_grabbed()
                        && !(keyboard.has_grab(serial)
                            || keyboard.has_grab(grab.previous_serial().unwrap_or(serial)))
                    {
                        grab.ungrab(PopupUngrabStrategy::All);
                        return;
                    }
                    keyboard.set_focus(self, grab.current_grab(), serial);
                    keyboard.set_grab(self, PopupKeyboardGrab::new(&grab), serial);
                }
                if let Some(pointer) = seat.get_pointer() {
                    if pointer.is_grabbed()
                        && !(pointer.has_grab(serial)
                            || pointer.has_grab(grab.previous_serial().unwrap_or_else(|| grab.serial())))
                    {
                        grab.ungrab(PopupUngrabStrategy::All);
                        return;
                    }
                    pointer.set_grab(self, PopupPointerGrab::new(&grab), serial, Focus::Keep);
                }
            }
            Err(error) => tracing::warn!(?error, "popup grab denied"),
        }
    }

    fn reposition_request(
        &mut self,
        surface: PopupSurface,
        positioner: PositionerState,
        token: u32,
    ) {
        // The client asked to move the popup (parent moved/resized or a new
        // positioner). Store the new geometry, clamp it to the output and
        // acknowledge with the token; the client commits the new position.
        let geometry = positioner.get_geometry();
        surface.with_pending_state(|state| {
            state.geometry = geometry;
            state.positioner = positioner;
        });
        self.unconstrain_popup(&surface);
        surface.send_repositioned(token);
    }

    fn title_changed(&mut self, surface: ToplevelSurface) {
        self.refresh_meta(&surface);
    }

    fn app_id_changed(&mut self, surface: ToplevelSurface) {
        self.refresh_meta(&surface);
    }

    fn toplevel_destroyed(&mut self, surface: ToplevelSurface) {
        let mut dead: Vec<Window> = self
            .space
            .elements()
            .filter(|w| w.toplevel().map(|t| t == &surface).unwrap_or(false))
            .cloned()
            .collect();
        // Minimized windows are unmapped from the space but still tracked.
        for w in self.props.keys() {
            if w.toplevel().map(|t| t == &surface).unwrap_or(false) && !dead.contains(w) {
                dead.push(w.clone());
            }
        }
        let mut focused_died = false;
        for w in &dead {
            // Drop any in-flight open/close animation for this window.
            self.shell.on_window_destroyed(w);
            self.space.unmap_elem(w);
            self.meta.remove(w);
            self.props.remove(w);
            if self.focused.as_ref() == Some(w) {
                focused_died = true;
            }
            if self.interaction.window() == Some(w) {
                self.interaction = Interaction::None;
            }
        }
        self.space.refresh();

        if focused_died {
            // Clear the dead reference first, then hand focus to the topmost
            // remaining window (`elements()` is back-to-front). `focus_window`
            // updates activation, the keyboard focus and, when nothing is left,
            // clears the focus entirely.
            let next = self.space.elements().last().cloned();
            let serial = self.next_serial();
            self.focus_window(next, serial);
        }
    }
}

impl State {
    /// Re-place a popup so it stays inside the output: the positioner's
    /// geometry is clamped to the output rectangle, expressed relative to the
    /// popup's parent window, which is what `get_unconstrained_geometry`
    /// expects.
    fn unconstrain_popup(&self, popup: &PopupSurface) {
        let kind = PopupKind::Xdg(popup.clone());
        let Ok(root) = find_popup_root_surface(&kind) else {
            return;
        };
        let Some(window) = self
            .space
            .elements()
            .find(|w| w.wl_surface().as_deref() == Some(&root))
            .cloned()
        else {
            return;
        };
        let window_geo = self
            .space
            .element_geometry(&window)
            .unwrap_or_else(|| window.geometry());
        // Start from the whole output and subtract the popup's current
        // toplevel offset and the window location. This is a single-output
        // compositor, so `output_size` is the entire desktop rect.
        let mut target = Rectangle::from_size(self.output_size);
        target.loc -= get_popup_toplevel_coords(&kind);
        target.loc -= window_geo.loc;
        popup.with_pending_state(|state| {
            state.geometry = state.positioner.get_unconstrained_geometry(target);
        });
    }

    fn refresh_meta(&mut self, surface: &ToplevelSurface) {
        if let Some(window) = self
            .space
            .elements()
            .find(|w| w.toplevel().map(|t| t == surface).unwrap_or(false))
            .cloned()
        {
            let meta = default_meta(&window);
            self.meta.insert(window, meta);
        }
    }

    fn set_maximized(&mut self, window: &Window, maximized: bool) {
        use smithay::reexports::wayland_protocols::xdg::shell::server::xdg_toplevel::State as XdgState;

        let props = self.window_props(window);
        if props.maximized == maximized && (!maximized || !props.fullscreen) {
            return;
        }
        let area = self.available_area();

        if maximized {
            let current = self
                .space
                .element_geometry(window)
                .unwrap_or_else(|| window.geometry());
            let normal = props.saved.unwrap_or(current);
            {
                let p = self.props_mut(window);
                p.maximized = true;
                p.fullscreen = false;
                p.saved = Some(normal);
                p.restore_maximized = false;
            }
            if let Some(t) = window.toplevel() {
                t.with_pending_state(|s| {
                    s.states.set(XdgState::Maximized);
                    s.states.unset(XdgState::Fullscreen);
                    s.size = Some((area.size.w, area.size.h).into());
                });
                t.send_configure();
            }
            self.space.map_element(window.clone(), area.loc, false);
        } else {
            if props.fullscreen {
                return;
            }
            let restore = props
                .saved
                .unwrap_or_else(|| Rectangle::new((0, 0).into(), config::DEFAULT_WINDOW_SIZE.into()));
            {
                let p = self.props_mut(window);
                p.maximized = false;
                p.saved = None;
                p.restore_maximized = false;
            }
            if let Some(t) = window.toplevel() {
                t.with_pending_state(|s| {
                    s.states.unset(XdgState::Maximized);
                    s.states.unset(XdgState::Fullscreen);
                    s.size = Some((restore.size.w, restore.size.h).into());
                });
                t.send_configure();
            }
            self.space.map_element(window.clone(), restore.loc, false);
        }
        self.space.refresh();
    }

    fn close_shell_overlays_for_fullscreen(&mut self) {
        self.shell.brightness_open = false;
        self.shell.set_brightness_open(false);
        self.shell.set_quick_settings_open(false);
        self.shell.launcher_open = false;
        self.shell.set_launcher_open(false);
        self.shell.power_open = false;
        self.shell.set_power_open(false);
    }

    fn set_fullscreen(&mut self, window: &Window, fullscreen: bool) {
        use smithay::reexports::wayland_protocols::xdg::shell::server::xdg_toplevel::State as XdgState;

        let props = self.window_props(window);
        if props.fullscreen == fullscreen {
            return;
        }
        let output = self.output_rect();

        if fullscreen {
            self.close_shell_overlays_for_fullscreen();
            let current = self
                .space
                .element_geometry(window)
                .unwrap_or_else(|| window.geometry());
            let normal = props.saved.unwrap_or(current);
            {
                let p = self.props_mut(window);
                p.restore_maximized = props.maximized || props.restore_maximized;
                p.saved = Some(normal);
                p.fullscreen = true;
                p.maximized = false;
            }
            if let Some(t) = window.toplevel() {
                t.with_pending_state(|s| {
                    s.states.set(XdgState::Fullscreen);
                    s.states.unset(XdgState::Maximized);
                    s.size = Some((output.size.w, output.size.h).into());
                });
                t.send_configure();
            }
            self.space.map_element(window.clone(), output.loc, false);
        } else if props.restore_maximized {
            let normal = props
                .saved
                .unwrap_or_else(|| Rectangle::new((0, 0).into(), config::DEFAULT_WINDOW_SIZE.into()));
            let area = self.available_area();
            {
                let p = self.props_mut(window);
                p.fullscreen = false;
                p.maximized = true;
                p.restore_maximized = false;
                p.saved = Some(normal);
            }
            if let Some(t) = window.toplevel() {
                t.with_pending_state(|s| {
                    s.states.unset(XdgState::Fullscreen);
                    s.states.set(XdgState::Maximized);
                    s.size = Some((area.size.w, area.size.h).into());
                });
                t.send_configure();
            }
            self.space.map_element(window.clone(), area.loc, false);
        } else {
            let restore = props
                .saved
                .unwrap_or_else(|| Rectangle::new((0, 0).into(), config::DEFAULT_WINDOW_SIZE.into()));
            {
                let p = self.props_mut(window);
                p.fullscreen = false;
                p.maximized = false;
                p.restore_maximized = false;
                p.saved = None;
            }
            if let Some(t) = window.toplevel() {
                t.with_pending_state(|s| {
                    s.states.unset(XdgState::Fullscreen);
                    s.states.unset(XdgState::Maximized);
                    s.size = Some((restore.size.w, restore.size.h).into());
                });
                t.send_configure();
            }
            self.space.map_element(window.clone(), restore.loc, false);
        }
        self.space.refresh();
    }

    fn set_minimized(&mut self, window: &Window, minimized: bool) {
        use smithay::reexports::wayland_protocols::xdg::shell::server::xdg_toplevel::State as XdgState;

        let props = self.window_props(window);
        if props.minimized == minimized {
            return;
        }
        let loc = if minimized {
            self.space
                .element_location(window)
                .or(props.minimized_location)
                .unwrap_or_else(|| self.next_location())
        } else {
            props
                .minimized_location
                .or_else(|| self.space.element_location(window))
                .unwrap_or_else(|| self.next_location())
        };

        if let Some(t) = window.toplevel() {
            t.with_pending_state(|s| {
                if minimized {
                    s.states.set(XdgState::Suspended);
                } else {
                    s.states.unset(XdgState::Suspended);
                }
            });
            t.send_configure();
        }
        {
            let p = self.props_mut(window);
            p.minimized = minimized;
            p.minimized_location = if minimized { Some(loc) } else { None };
        }

        if minimized {
            self.space.unmap_elem(window);
            if self.focused.as_ref() == Some(window) {
                // Keep `focused` intact until `focus_window` deactivates the old
                // live surface; clearing it first skips the XDG configure.
                let next = self.space.elements().last().cloned();
                let serial = self.next_serial();
                self.focus_window(next, serial);
            }
        } else {
            self.space.map_element(window.clone(), loc, false);
            let serial = self.next_serial();
            self.space.raise_element(window, false);
            self.focus_window(Some(window.clone()), serial);
        }
        self.space.refresh();
    }

    /// Restore the given window and focus it (dock click on a minimized window).
    pub fn restore_window(&mut self, window: &Window, _serial: Serial) {
        self.set_minimized(window, false);
    }

    pub fn toggle_maximize(&mut self, window: &Window) {
        let maximized = self.window_props(window).maximized;
        self.set_maximized(window, !maximized);
    }

    pub fn minimize_window(&mut self, window: &Window) {
        self.set_minimized(window, true);
    }
}

impl TabletSeatHandler for State {}

impl DmabufHandler for State {    fn dmabuf_state(&mut self) -> &mut DmabufState {
        &mut self.dmabuf_state
    }

    fn dmabuf_imported(&mut self, _global: &DmabufGlobal, _dmabuf: Dmabuf, notifier: ImportNotifier) {
        // The `GlesRenderer` imports dmabufs lazily while drawing surface
        // elements, so validating the buffer here would duplicate that work.
        // Acknowledge the import and let the renderer reject unsupported buffers.
        let _ = notifier.successful::<State>();
    }
}

impl XdgActivationHandler for State {
    fn activation_state(&mut self) -> &mut XdgActivationState {
        &mut self.activation_state
    }

    fn request_activation(
        &mut self,
        _token: XdgActivationToken,
        _token_data: XdgActivationTokenData,
        surface: WlSurface,
    ) {
        // Bring the target toplevel forward. Minimized windows are restored so
        // activation from a notification or file manager actually shows them.
        let window = self
            .space
            .elements()
            .chain(self.props.keys())
            .find(|w| w.wl_surface().map(|s| *s == surface).unwrap_or(false))
            .cloned();
        if let Some(window) = window {
            if self.window_props(&window).minimized {
                let serial = self.next_serial();
                self.restore_window(&window, serial);
            } else {
                self.space.raise_element(&window, false);
                let serial = self.next_serial();
                self.focus_window(Some(window), serial);
            }
        }
    }
}

impl FractionalScaleHandler for State {
    fn new_fractional_scale(&mut self, surface: WlSurface) {
        // The output runs at an integer scale of 1, so the preferred fractional
        // scale is always 1.0. Announcing it keeps fractionally-scaled clients
        // from picking their own guess.
        let scale = self.output.current_scale().fractional_scale();
        compositor::with_states(&surface, |states| {
            fractional_scale::with_fractional_scale(states, |fs| {
                fs.set_preferred_scale(scale);
            });
        });
    }
}

/// Windows left untitled by their client fall back to "Window". GTK leaves the
/// program path (`/usr/bin/app`) as the default title when an app never calls
/// `gtk_window_set_title`; showing just the binary name is friendlier.
fn display_title(title: Option<String>) -> String {
    let Some(title) = title else {
        return "Window".to_string();
    };
    let trimmed = title.trim();
    if trimmed.starts_with('/') && !trimmed.chars().any(char::is_whitespace) {
        if let Some(name) = std::path::Path::new(trimmed).file_name() {
            let name = name.to_string_lossy();
            if !name.is_empty() {
                return name.into_owned();
            }
        }
    }
    title
}

fn default_meta(window: &Window) -> WindowMeta {
    let data = window.toplevel().and_then(|t| {
        let surface = t.wl_surface().clone();
        compositor::with_states(&surface, |states| {
            states
                .data_map
                .get::<XdgToplevelSurfaceData>()
                .and_then(|data| {
                    let guard = data.lock().ok()?;
                    Some((guard.title.clone(), guard.app_id.clone()))
                })
        })
    });
    let (title, app_id) = data.unwrap_or((None, None));
    WindowMeta {
        title: display_title(title),
        app_id: app_id.unwrap_or_default(),
    }
}

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
mod tests {
    use super::*;
    use smithay::reexports::wayland_server::Display;

    fn fresh() -> (Display<State>, State) {
        let mut display = Display::<State>::new().expect("display");
        let state = State::new(
            &mut display,
            Fonts::load().expect("test system font"),
            vec![],
            vec![],
            ThemeSpec::DEFAULT,
        );
        (display, state)
    }

    #[test]
    fn cascade_stays_within_output() {
        let (_display, mut s) = fresh();
        let (ww, wh) = config::DEFAULT_WINDOW_SIZE;
        let dock = config::DOCK_HEIGHT as i32 + config::DOCK_MARGIN as i32;
        for _ in 0..30 {
            let p = s.next_location();
            assert!(p.x >= 0 && p.y >= 0);
            assert!(p.x + ww <= s.output_size.w, "window spills right: {p:?}");
            assert!(p.y + wh <= s.output_size.h - dock, "window hits dock: {p:?}");
        }
    }

    #[test]
    fn dock_item_count_matches_the_built_list() {
        let (_display, s) = fresh();
        assert_eq!(
            crate::input::dock_item_count(&s),
            crate::input::dock_items(&s).len()
        );
    }

    #[test]
    fn theme_toggle_switches_scheme() {
        let (_display, mut s) = fresh();
        assert!(s.theme.is_dark);
        let origin = Point::from((500.0, 16.0));
        s.toggle_theme(origin);
        assert!(!s.theme.is_dark);
        s.toggle_theme(origin);
        assert!(s.theme.is_dark);
    }

    #[test]
    fn set_variant_keeps_brightness_and_changes_palette() {
        let (_display, mut s) = fresh();
        assert!(s.theme.is_dark);
        let before = s.theme.primary;
        s.set_variant(ColorVariant::Vibrant);
        assert!(s.theme.is_dark, "brightness must not change");
        assert_eq!(s.theme_spec.variant, ColorVariant::Vibrant);
        assert_ne!(s.theme.primary, before, "palette must change");
    }

    #[test]
    fn clicking_the_dock_launcher_keeps_the_launcher_open() {
        let (_display, mut s) = fresh();
        let (w, h) = (s.output_size.w as f32, s.output_size.h as f32);
        let items = crate::input::dock_items(&s);
        let rect = crate::shell::dock_item_rect(w, h, items.len(), 0);
        let pos = Point::from((
            (rect.x + rect.w / 2.0) as f64,
            (rect.y + rect.h / 2.0) as f64,
        ));
        // The press half of the click: the dock button swallows it. The
        // release must open the launcher and leave it open.
        s.shell.press_dock = Some(0);
        crate::input::finish_compositor_press(&mut s, pos);
        assert!(
            s.shell.launcher_open,
            "the launcher must stay open after the click that opened it"
        );
    }

    #[test]
    fn clicking_the_launcher_scrim_dismisses_it() {
        let (_display, mut s) = fresh();
        s.shell.launcher_open = true;
        let (w, h) = (s.output_size.w as f32, s.output_size.h as f32);
        // Inside the overlay, outside the centred card: a bare scrim click.
        let pos = Point::from((12.0, h as f64 - 12.0));
        assert_eq!(
            crate::shell::launcher_hit(w, h, 0, pos),
            crate::shell::LauncherHit::Scrim
        );
        crate::input::finish_compositor_press(&mut s, pos);
        assert!(
            !s.shell.launcher_open,
            "a scrim click must dismiss the launcher"
        );
    }

    #[test]
    fn user_button_opens_the_session_menu_without_quitting() {
        let (_display, mut s) = fresh();
        crate::input::perform_top_action(&mut s, crate::shell::TopAction::Power);
        assert!(s.shell.power_open, "the user button opens the session menu");
        assert!(s.running, "opening the menu must not stop the compositor");
        crate::input::perform_top_action(&mut s, crate::shell::TopAction::Power);
        assert!(!s.shell.power_open, "a second press closes the menu");
        assert!(s.running);
    }

    #[test]
    fn session_menu_logout_stops_the_compositor() {
        let (_display, mut s) = fresh();
        s.shell.power_open = true;
        crate::input::perform_power_action(&mut s, crate::shell::PowerAction::Logout);
        assert!(!s.running, "log out must stop the event loop");
        assert!(!s.shell.power_open, "picking an action closes the menu");
    }

    #[test]
    fn fullscreen_closes_all_shell_overlays() {
        let (_display, mut s) = fresh();
        s.shell.brightness_open = true;
        s.shell.quick_settings_open = true;
        s.shell.launcher_open = true;
        s.shell.power_open = true;

        s.close_shell_overlays_for_fullscreen();

        assert!(!s.shell.brightness_open);
        assert!(!s.shell.quick_settings_open);
        assert!(!s.shell.launcher_open);
        assert!(!s.shell.power_open);
    }

    #[test]
    fn serial_advances_monotonically() {
        let (_display, mut s) = fresh();
        let a: u32 = s.next_serial().into();
        let b: u32 = s.next_serial().into();
        assert!(b > a);
    }
}
