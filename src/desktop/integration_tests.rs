//! In-process Wayland clients exercise actual configure/ack/buffer transactions.
use super::*;
use smithay::reexports::wayland_protocols::xdg::activation::v1::client::{
    xdg_activation_token_v1, xdg_activation_v1,
};
use smithay::reexports::{
    rustix::fs::{memfd_create, MemfdFlags},
    wayland_protocols::xdg::shell::client::{xdg_surface, xdg_toplevel, xdg_wm_base},
    wayland_server::Display,
};
use std::{fs::File, io::Write, os::fd::AsFd, os::unix::net::UnixStream, sync::Arc};
use wayland_client::{
    delegate_noop,
    protocol::{wl_buffer, wl_compositor, wl_registry, wl_shm, wl_shm_pool, wl_surface},
    Connection, Dispatch, QueueHandle,
};
#[derive(Default)]
struct Client {
    compositor: Option<wl_compositor::WlCompositor>,
    shm: Option<wl_shm::WlShm>,
    wm: Option<xdg_wm_base::XdgWmBase>,
    windows: Vec<ClientWindow>,
    activation: Option<xdg_activation_v1::XdgActivationV1>,
    seat: Option<wayland_client::protocol::wl_seat::WlSeat>,
    tokens: Vec<String>,
}
struct ClientWindow {
    surface: wl_surface::WlSurface,
    xdg: xdg_surface::XdgSurface,
    top: xdg_toplevel::XdgToplevel,
    size: (i32, i32),
    color: u8,
    closed: bool,
    ignore_close: bool,
    close_requests: usize,
}
impl Dispatch<wl_registry::WlRegistry, ()> for Client {
    fn event(
        state: &mut Self,
        registry: &wl_registry::WlRegistry,
        event: wl_registry::Event,
        _: &(),
        _: &Connection,
        qh: &QueueHandle<Self>,
    ) {
        if let wl_registry::Event::Global {
            name,
            interface,
            version,
        } = event
        {
            match interface.as_str() {
                "wl_compositor" => {
                    state.compositor = Some(registry.bind(name, version.min(4), qh, ()))
                }
                "wl_shm" => state.shm = Some(registry.bind(name, 1, qh, ())),
                "xdg_wm_base" => state.wm = Some(registry.bind(name, version.min(5), qh, ())),
                "xdg_activation_v1" => state.activation = Some(registry.bind(name, 1, qh, ())),
                "wl_seat" => state.seat = Some(registry.bind(name, 1, qh, ())),
                _ => {}
            }
        }
    }
}
impl Dispatch<xdg_wm_base::XdgWmBase, ()> for Client {
    fn event(
        _: &mut Self,
        wm: &xdg_wm_base::XdgWmBase,
        event: xdg_wm_base::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        if let xdg_wm_base::Event::Ping { serial } = event {
            wm.pong(serial);
        }
    }
}
impl Dispatch<xdg_toplevel::XdgToplevel, usize> for Client {
    fn event(
        state: &mut Self,
        _: &xdg_toplevel::XdgToplevel,
        event: xdg_toplevel::Event,
        index: &usize,
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        match event {
            xdg_toplevel::Event::Configure { width, height, .. } => {
                state.windows[*index].size = (width.max(1), height.max(1))
            }
            xdg_toplevel::Event::Close => {
                let window = &mut state.windows[*index];
                window.close_requests += 1;
                if window.ignore_close {
                    return;
                }
                window.closed = true;
                window.top.destroy();
                window.xdg.destroy();
                window.surface.destroy();
            }
            _ => {}
        }
    }
}
impl Dispatch<xdg_surface::XdgSurface, usize> for Client {
    fn event(
        state: &mut Self,
        xdg: &xdg_surface::XdgSurface,
        event: xdg_surface::Event,
        index: &usize,
        _: &Connection,
        qh: &QueueHandle<Self>,
    ) {
        if let xdg_surface::Event::Configure { serial } = event {
            let window = &state.windows[*index];
            if window.closed {
                return;
            }
            xdg.ack_configure(serial);
            let (w, h) = window.size;
            let fd = memfd_create("materialand-test", MemfdFlags::CLOEXEC).unwrap();
            let mut file = File::from(fd);
            let pixels = [window.color, 80, 120, 255].repeat((w * h) as usize);
            file.write_all(&pixels).unwrap();
            let pool = state
                .shm
                .as_ref()
                .unwrap()
                .create_pool(file.as_fd(), w * h * 4, qh, ());
            let buffer = pool.create_buffer(0, w, h, w * 4, wl_shm::Format::Argb8888, qh, ());
            window.xdg.set_window_geometry(0, 0, w, h);
            window.surface.attach(Some(&buffer), 0, 0);
            window.surface.damage(0, 0, w, h);
            window.surface.commit();
            pool.destroy();
            buffer.destroy();
        }
    }
}
delegate_noop!(Client: ignore wl_compositor::WlCompositor);
delegate_noop!(Client: ignore wl_shm::WlShm);
delegate_noop!(Client: ignore wl_shm_pool::WlShmPool);
delegate_noop!(Client: ignore wl_surface::WlSurface);
delegate_noop!(Client: ignore wl_buffer::WlBuffer);
delegate_noop!(Client: ignore xdg_activation_v1::XdgActivationV1);
delegate_noop!(Client: ignore wayland_client::protocol::wl_seat::WlSeat);
impl Dispatch<xdg_activation_token_v1::XdgActivationTokenV1, ()> for Client {
    fn event(
        state: &mut Self,
        _: &xdg_activation_token_v1::XdgActivationTokenV1,
        event: xdg_activation_token_v1::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        if let xdg_activation_token_v1::Event::Done { token } = event {
            state.tokens.push(token);
        }
    }
}
struct Harness {
    display: Display<State>,
    state: State,
    connection: Connection,
    queue: wayland_client::EventQueue<Client>,
    client: Client,
    _registry: wl_registry::WlRegistry,
    server_client: smithay::reexports::wayland_server::Client,
}
impl Harness {
    fn activation_token(&mut self, serial: Serial, source: usize) -> String {
        let request = self
            .client
            .activation
            .as_ref()
            .unwrap()
            .get_activation_token(&self.queue.handle(), ());
        request.set_serial(serial.into(), self.client.seat.as_ref().unwrap());
        request.set_surface(&self.client.windows[source].surface);
        request.commit();
        self.pump();
        request.destroy();
        self.client.tokens.pop().unwrap()
    }
    fn new() -> Self {
        let mut display = Display::<State>::new().unwrap();
        let mut state = State::new(
            &mut display,
            Fonts::load().unwrap(),
            vec![],
            vec![],
            ThemeSpec::default(),
        );
        state.shell.set_reduce_motion(true);
        let (a, b) = UnixStream::pair().unwrap();
        let server_client = display
            .handle()
            .insert_client(a, Arc::new(ClientState::default()))
            .unwrap();
        let connection = Connection::from_socket(b).unwrap();
        let queue = connection.new_event_queue();
        let registry = connection.display().get_registry(&queue.handle(), ());
        let mut harness = Self {
            display,
            state,
            connection,
            queue,
            client: Client::default(),
            _registry: registry,
            server_client,
        };
        harness.pump();
        assert!(harness.client.wm.is_some());
        harness
    }
    fn pump(&mut self) {
        for _ in 0..20 {
            self.connection.flush().unwrap();
            self.display.dispatch_clients(&mut self.state).unwrap();
            self.display
                .backend()
                .flush(Some(self.server_client.id()))
                .unwrap();
            if let Some(guard) = self.connection.prepare_read() {
                let backend = self.connection.backend();
                let fd = backend.poll_fd();
                let mut pollfd = libc::pollfd {
                    fd: std::os::fd::AsRawFd::as_raw_fd(&fd),
                    events: libc::POLLIN,
                    revents: 0,
                };
                // A bounded poll allows queued events to arrive without a blocking roundtrip.
                let ready = unsafe { libc::poll(&mut pollfd, 1, 2) };
                if ready > 0 {
                    guard.read().unwrap();
                }
            }
            self.queue.dispatch_pending(&mut self.client).unwrap();
        }
    }
    fn window(&mut self, title: &str, color: u8) -> Window {
        let index = self.client.windows.len();
        let qh = self.queue.handle();
        let surface = self
            .client
            .compositor
            .as_ref()
            .unwrap()
            .create_surface(&qh, ());
        let xdg = self
            .client
            .wm
            .as_ref()
            .unwrap()
            .get_xdg_surface(&surface, &qh, index);
        let top = xdg.get_toplevel(&qh, index);
        top.set_title(title.into());
        top.set_app_id(title.into());
        top.set_min_size(240, 180);
        self.client.windows.push(ClientWindow {
            surface: surface.clone(),
            xdg,
            top,
            size: (400, 300),
            color,
            closed: false,
            ignore_close: false,
            close_requests: 0,
        });
        surface.commit();
        self.pump();
        self.state
            .windows
            .order
            .iter()
            .find(|window| self.state.window_meta(window).title == title)
            .unwrap()
            .clone()
    }
}
#[test]
#[ignore = "requires Wayland socket sendmsg permitted by the execution environment"]
fn opening_window_consumes_hits_on_content_outside_its_xdg_geometry() {
    let mut h = Harness::new();
    let window = h.window("Extended client content", 180);
    let client = &h.client.windows[0];
    client
        .xdg
        .set_window_geometry(40, 40, client.size.0 - 80, client.size.1 - 80);
    client.surface.commit();
    h.pump();
    h.state.shell.set_reduce_motion(false);
    h.state.shell.set_window_launch_origin(
        &window,
        crate::shell::RectF {
            x: 200.0,
            y: 300.0,
            w: 100.0,
            h: 100.0,
        },
    );
    let frame = h.state.window_frame_rect(&window).unwrap();
    let pose = h.state.shell.window_presentation(&window, frame);
    let origin = h.state.window_surface_origin(&window);
    let source = (f64::from(origin.x + 10), f64::from(origin.y + 10));
    let point = Point::from((
        f64::from(frame.x.round())
            + (source.0 - f64::from(frame.x.round())) * pose.scale()
            + f64::from((pose.target.x - frame.x).round()),
        f64::from(frame.y.round())
            + (source.1 - f64::from(frame.y.round())) * f64::from(pose.target.h / frame.h)
            + f64::from((pose.target.y - frame.y).round()),
    ));
    assert!(
        !pose.target.contains(point),
        "fixture is outside the animated frame"
    );
    assert!(
        matches!(h.state.window_under(point), Some(super::hit_test::WindowHit::Opening(hit)) if hit == window)
    );
}

#[test]
#[ignore = "requires Wayland socket sendmsg permitted by the execution environment"]
fn titlebar_release_outside_or_under_overlay_does_not_close_the_client() {
    use smithay::reexports::wayland_protocols::xdg::decoration::zv1::server::zxdg_toplevel_decoration_v1::Mode;
    let mut h = Harness::new();
    let window = h.window("Titlebar controls", 180);
    h.state
        .request_mode(window.toplevel().unwrap().clone(), Mode::ServerSide);
    h.pump();
    assert!(h.state.window_server_decor(&window));
    let geometry = h.state.windows.space.element_geometry(&window).unwrap();
    let layout = crate::shell::decor_layout(geometry);
    let point = Point::from((f64::from(layout.close.0), f64::from(layout.close.1)));
    crate::input::on_pointer_motion(&mut h.state, point, 1);
    h.state
        .shell
        .decoration_controls
        .begin(window.clone(), crate::shell::DecorButton::Close);
    h.pump();
    assert_eq!(
        h.client.windows[0].close_requests, 0,
        "press does not close"
    );
    crate::input::finish_compositor_press(&mut h.state, (0.0, 0.0).into());
    h.pump();
    assert_eq!(
        h.client.windows[0].close_requests, 0,
        "release outside cancels"
    );

    h.state
        .shell
        .decoration_controls
        .begin(window.clone(), crate::shell::DecorButton::Close);
    h.state.swallowed_buttons.insert(0x110);
    crate::input::prepare_shell_overlay(&mut h.state);
    h.state.shell.set_launcher_open(true);
    h.state.shell.launcher_open = true;
    h.state.swallowed_buttons.remove(&0x110);
    crate::input::finish_compositor_press(&mut h.state, point);
    h.pump();
    assert_eq!(
        h.client.windows[0].close_requests, 0,
        "overlay blocks release"
    );
    assert!(
        h.state.shell.launcher_open,
        "old release must not close the new overlay"
    );
    h.state.shell.launcher_open = false;
    h.state.shell.set_launcher_open(false);

    h.state
        .shell
        .decoration_controls
        .begin(window.clone(), crate::shell::DecorButton::Close);
    crate::input::finish_compositor_press(&mut h.state, point);
    h.pump();
    assert_eq!(h.client.windows[0].close_requests, 1);
    assert!(!h.state.windows.props.contains_key(&window));
}

#[test]
#[ignore = "requires Wayland socket sendmsg permitted by the execution environment"]
fn activation_needs_recent_user_input_and_tokens_are_consumed() {
    let mut h = Harness::new();
    let source = h.window("Source", 180);
    let target = h.window("Target", 200);
    h.state.minimize_window(&target);
    h.pump();
    assert_eq!(h.state.windows.focused.as_ref(), Some(&source));
    let serial = h.state.next_serial();
    // Merely knowing a serial does not authorize a background focus request.
    let token = h.activation_token(serial, 0);
    h.client
        .activation
        .as_ref()
        .unwrap()
        .activate(token, &h.client.windows[1].surface);
    h.pump();
    assert!(h.state.window_props(&target).minimized);
    assert_eq!(h.state.activation_state.tokens().count(), 0);

    h.state
        .record_activation_input(serial, &source.wl_surface().unwrap());
    let token = h.activation_token(serial, 0);
    assert_eq!(h.state.activation_state.tokens().count(), 1);
    // One delivered event may issue only one valid token.
    let duplicate = h.activation_token(serial, 0);
    h.client
        .activation
        .as_ref()
        .unwrap()
        .activate(duplicate, &h.client.windows[1].surface);
    h.pump();
    assert!(h.state.window_props(&target).minimized);
    h.client
        .activation
        .as_ref()
        .unwrap()
        .activate(token.clone(), &h.client.windows[1].surface);
    h.pump();
    assert!(!h.state.window_props(&target).minimized);
    assert_eq!(h.state.windows.focused.as_ref(), Some(&target));
    assert_eq!(h.state.activation_state.tokens().count(), 0);
    h.state.minimize_window(&target);
    h.client
        .activation
        .as_ref()
        .unwrap()
        .activate(token, &h.client.windows[1].surface);
    h.pump();
    assert!(h.state.window_props(&target).minimized);

    let serial = h.state.next_serial();
    h.state.activation_inputs.record(
        serial,
        h.server_client.id(),
        Instant::now() - std::time::Duration::from_secs(6),
    );
    h.activation_token(serial, 0);
    assert_eq!(h.state.activation_state.tokens().count(), 0);
}
#[test]
#[ignore = "requires Wayland socket sendmsg permitted by the execution environment"]
fn rejected_close_keeps_window_visible_and_save_dialog_accessible() {
    for reduced in [false, true] {
        let mut h = Harness::new();
        h.state.shell.set_reduce_motion(reduced);
        let window = h.window("Unsaved document", 180);
        h.client.windows[0].ignore_close = true;
        h.state.request_close_window(&window);
        h.pump();
        assert_eq!(h.client.windows[0].close_requests, 1);
        let frame = h.state.window_frame_rect(&window).unwrap();
        for _ in 0..180 {
            h.state.shell.tick(1.0 / 60.0);
            let pose = h.state.shell.window_presentation(&window, frame);
            assert_eq!(pose.alpha, 1.0);
            assert_eq!(pose.target, frame);
            assert!(h.state.window_has_content(&window));
        }
        assert_eq!(h.state.windows.focused.as_ref(), Some(&window));
        let dialog = h.window("Save changes?", 200);
        assert_eq!(h.state.windows.focused.as_ref(), Some(&dialog));
        h.state.request_close_window(&dialog);
        h.pump();
        assert!(h.state.window_has_content(&window));
        // A cancelled request must not suppress a later explicit close.
        h.client.windows[0].ignore_close = false;
        h.state.request_close_window(&window);
        h.pump();
        assert_eq!(h.client.windows[0].close_requests, 2);
        assert!(!h.state.windows.props.contains_key(&window));
    }
}
#[test]
#[ignore = "requires Wayland socket sendmsg permitted by the execution environment"]
fn real_clients_survive_overview_minimize_split_resize_and_pair_member_close() {
    let mut h = Harness::new();
    let a = h.window("App A", 180);
    let b = h.window("App B", 220);
    assert!(h.state.window_has_content(&a) && h.state.window_has_content(&b));
    h.state.minimize_window(&a);
    h.pump();
    assert!(h.state.window_props(&a).minimized);
    h.state.open_overview();
    h.pump();
    assert_eq!(h.state.overview.items.len(), 2);
    let index = h
        .state
        .overview
        .items
        .iter()
        .position(|item| item.window == a)
        .unwrap();
    h.state.overview_action(overview::OverviewHit {
        index,
        action: overview::OverviewAction::Select,
    });
    h.pump();
    assert!(!h.state.window_props(&a).minimized);
    assert_eq!(h.state.windows.focused.as_ref(), Some(&a));
    h.state.open_overview();
    let first = h
        .state
        .overview
        .items
        .iter()
        .position(|item| item.window == a)
        .unwrap();
    let second = h
        .state
        .overview
        .items
        .iter()
        .position(|item| item.window == b)
        .unwrap();
    let split_hit = |index| overview::OverviewHit {
        index,
        action: overview::OverviewAction::Split,
    };
    h.state.overview_action(split_hit(first));
    assert_eq!(h.state.overview.pending_split.as_ref(), Some(&a));
    // Tapping the same icon cancels selection; a second app completes the pair.
    h.state.overview_action(split_hit(first));
    assert!(h.state.overview.pending_split.is_none());
    h.state.overview_action(split_hit(first));
    let original_mode = h.state.display.handle.current_mode().unwrap();
    h.state.set_output_mode(Mode {
        size: (400, 768).into(),
        refresh: original_mode.refresh,
    });
    h.pump();
    h.state.overview_action(split_hit(second));
    assert!(h.state.split.is_none());
    assert_eq!(h.state.overview.pending_split.as_ref(), Some(&a));
    assert!(h.state.overview.notice.is_some());
    h.state.set_output_mode(original_mode);
    h.pump();
    h.state.overview_action(split_hit(second));
    assert!(h.state.overview.pending_split.is_none());
    assert!(!h.state.overview.open);
    assert_eq!(
        h.state
            .split
            .as_ref()
            .map(|split| (&split.left, &split.right)),
        Some((&a, &b))
    );
    h.pump();
    assert!(h.state.split.is_some());
    let divider = h.state.split_divider().unwrap();
    // The enlarged grab zone deliberately overlaps each paired client's CSD.
    for dx in [-9.0, 0.0, 9.0] {
        let point = (
            f64::from(divider.x + divider.w / 2.0 + dx),
            f64::from(divider.y + 100.0),
        )
            .into();
        assert!(h.state.split_divider_hit(point));
    }
    h.state.cursor = (
        f64::from(divider.x + divider.w / 2.0 + 9.0),
        f64::from(divider.y + 100.0),
    )
        .into();
    h.state.begin_split_drag();
    let center_before = h.state.split_divider().unwrap().x;
    h.state.move_split_divider(h.state.cursor.x as f32);
    assert_eq!(
        h.state.split_divider().unwrap().x,
        center_before,
        "off-center grab moved the divider"
    );
    h.state.move_split_divider(850.0);
    h.state.tick_split(1.0 / 144.0);
    h.pump();
    let left = h.state.split_area_for(&a).unwrap();
    let right = h.state.split_area_for(&b).unwrap();
    assert_eq!(a.geometry().size, left.size);
    assert_eq!(b.geometry().size, right.size);
    assert!(left.loc.x + left.size.w < right.loc.x);
    assert_eq!(left.loc.x, h.state.available_area().loc.x);
    assert_eq!(
        right.loc.x + right.size.w,
        h.state.available_area().loc.x + h.state.available_area().size.w
    );
    h.state.finish_interaction(0x110);
    assert!(h.state.split.is_some());
    // A client resize request on either inner edge resizes the pair, not one
    // restored floating window in the middle of the screen.
    for (window, edges) in [
        (
            &a,
            ResizeEdges {
                right: true,
                ..Default::default()
            },
        ),
        (
            &b,
            ResizeEdges {
                left: true,
                ..Default::default()
            },
        ),
    ] {
        let geometry = h.state.windows.space.element_geometry(window).unwrap();
        h.state.begin_resize(window.clone(), edges, geometry, 0x110);
        assert!(matches!(h.state.interaction, Interaction::SplitDivider));
        assert!(h.state.split.is_some());
        h.state.finish_interaction(0x110);
    }
    assert!(
        matches!(h.state.window_under((f64::from(left.loc.x+20),f64::from(left.loc.y+30)).into()),Some(super::hit_test::WindowHit::Client{window,..}) if window==a)
    );
    h.state.set_output_mode(Mode {
        size: (1024, 768).into(),
        refresh: 144_000,
    });
    h.pump();
    assert!(h.state.split.is_some());
    assert_eq!(a.geometry().size, h.state.split_area_for(&a).unwrap().size);
    h.state.request_close_window(&a);
    h.pump();
    assert!(!h.state.windows.props.contains_key(&a));
    assert!(h.state.split.is_none());
    assert!(h.state.window_has_content(&b));
    assert!(b.toplevel().unwrap().alive());
}
#[test]
#[ignore = "requires Wayland socket sendmsg permitted by the execution environment"]
fn overview_close_sends_a_real_close_and_maximizing_exits_split() {
    let mut h = Harness::new();
    let a = h.window("One", 180);
    let b = h.window("Two", 210);
    let original = b.geometry().size;
    h.state.toggle_maximize(&b);
    h.pump();
    h.state.start_split(a.clone(), b.clone());
    h.pump();
    h.state.toggle_maximize(&a);
    h.pump();
    assert!(h.state.split.is_none());
    assert_eq!(b.geometry().size, original);
    h.state.open_overview();
    let index = h
        .state
        .overview
        .items
        .iter()
        .position(|item| item.window == b)
        .unwrap();
    h.state.overview_action(overview::OverviewHit {
        index,
        action: overview::OverviewAction::Close,
    });
    h.pump();
    h.state.sync_overview();
    assert!(!h.state.overview.items.iter().any(|item| item.window == b));
}
