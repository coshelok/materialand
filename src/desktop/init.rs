//! Init.
use super::*;

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
        let dmabuf_global = dmabuf_state.create_global::<Self>(&dh, dmabuf_formats.clone());
        let mut seat_state = SeatState::new();
        let mut seat = seat_state.new_wl_seat(&dh, "seat-0");

        let output = Output::new(
            "materialand-0".to_string(),
            PhysicalProperties {
                // A nested window has no physical diagonal. Direct mode
                // replaces these properties with the active DRM connector.
                size: (0, 0).into(),
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

        let theme = if std::env::var("MATERIALAND_THEME").is_ok_and(|v| v == "dark") {
            spec.dark()
        } else {
            spec.light()
        };
        let mut shell = Shell::new(apps, spec);
        shell.set_theme_switch(theme.is_dark);
        State {
            forwarded_buttons: HashSet::new(),
            cursor_status: smithay::input::pointer::CursorImageStatus::default_named(),
            pending_imports: Vec::new(),
            advertised_formats: dmabuf_formats,
            retired_dmabuf_globals: Vec::new(),
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
            windows: WindowManager {
                space,
                ..Default::default()
            },
            display: DisplayModel {
                handle: output,
                size: (1280, 800).into(),
            },
            display_handle: dh.clone(),
            session_mode: false,
            _output_global,
            retired_output_globals: Vec::new(),
            shell,
            fonts,
            theme,
            theme_spec: spec,
            running: true,
            start_time: Instant::now(),
            cursor: (640.0, 400.0).into(),
            interaction: Interaction::None,
            brightness: 1.0,
            last_interactive_press: None,
            swallowed_buttons: HashSet::new(),
            swallowed_keys: HashSet::new(),
        }
    }
}
