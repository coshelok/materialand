//! Explicit visual-review artifacts; no seat, live clients or system changes.
use super::*;
fn raster(mut buffer: MemoryRenderBuffer, w: u32, h: u32) -> Surface {
    let mut data = Vec::new();
    buffer
        .render()
        .draw::<_, ()>(|pixels| {
            data.extend_from_slice(pixels);
            Ok(vec![])
        })
        .unwrap();
    Surface {
        pixmap: tiny_skia::Pixmap::from_vec(data, tiny_skia::IntSize::from_wh(w, h).unwrap())
            .unwrap(),
    }
}
#[test]
#[ignore = "writes design previews under /tmp for manual visual review"]
fn write_tablet_design_previews() {
    let mut fonts = Fonts::load().unwrap();
    let apps: Vec<_> = [
        ("Browser", "public"),
        ("Terminal", "terminal"),
        ("Code", "code"),
        ("Files", "folder"),
        ("Music", "music_note"),
        ("Photos", "photo"),
    ]
    .into_iter()
    .map(|(name, icon)| AppEntry {
        name: name.into(),
        exec: "/bin/true".into(),
        icon: icon.into(),
        terminal: false,
        desktop_file: Some(format!("/applications/{name}.desktop").into()),
    })
    .collect();
    for (w, h) in [(1280, 800), (640, 480)] {
        for (name, theme) in [
            ("light", ThemeSpec::default().light()),
            ("dark", ThemeSpec::default().dark()),
        ] {
            let mut shell = Shell::new(apps.clone(), ThemeSpec::default());
            shell.set_reduce_motion(true);
            shell.set_theme_switch(theme.is_dark);
            shell.clock = "14:32".into();
            shell.brightness_available = true;
            shell.night_light_available = true;
            shell.notifications = crate::notifications::Snapshot {
                available: true,
                revision: 1,
                items: vec![crate::notifications::Notice {
                    id: 1,
                    app: "Mail".into(),
                    summary: "A new message".into(),
                    body: "Your notification appears here.".into(),
                }],
                ..Default::default()
            };
            for app in &apps {
                shell
                    .place_home_app(app.clone(), shell.home.apps.len())
                    .unwrap();
            }
            shell.set_system_status(crate::system::Status {
                battery: Some(crate::system::Battery {
                    percent: 67,
                    charging: false,
                    plugged: false,
                }),
                wifi: crate::system::Radio {
                    available: true,
                    enabled: true,
                    label: "Home Wi-Fi".into(),
                    busy: false,
                },
                bluetooth: crate::system::Radio {
                    available: true,
                    enabled: true,
                    label: "1 connected".into(),
                    busy: false,
                },
                ..Default::default()
            });
            shell.render_wallpaper(&theme, w, h, None);
            let mut canvas = Surface::new(w, h);
            canvas.blit(
                &shell.wallpapers.last().unwrap().surface,
                SkTransform::identity(),
                1.0,
                None,
            );
            let area = home::home_layout(w as f32, h as f32).area;
            let (buffer, loc) = shell.render_home(&theme, &mut fonts, w as f32, h as f32, None);
            canvas.blit(
                &raster(buffer, area.w.ceil() as u32, area.h.ceil() as u32),
                SkTransform::from_translate(loc.x as f32, loc.y as f32),
                1.0,
                None,
            );
            let (buffer, loc) = shell.render_navigation(&theme, &mut fonts, w as f32, h as f32);
            let nav = navigation_rect(w as f32, h as f32);
            let pad = floating_dock_padding();
            canvas.blit(
                &raster(
                    buffer,
                    (nav.w.ceil() + 2.0 * pad) as u32,
                    (nav.h.ceil() + 2.0 * pad) as u32,
                ),
                SkTransform::from_translate(loc.x as f32, loc.y as f32),
                1.0,
                None,
            );
            let (buffer, _) = shell.render_statusbar(&theme, &mut fonts, w as f32);
            canvas.blit(
                &raster(buffer, w, config::design().statusbar.height.ceil() as u32),
                SkTransform::identity(),
                1.0,
                None,
            );
            let home_canvas = canvas.pixmap.clone();
            canvas
                .pixmap
                .save_png(format!("/tmp/materialand-tablet-{name}-{w}-home.png"))
                .unwrap();
            let drawer = launcher_card(w as f32, h as f32);
            let drawer_pad = config::design().launcher.shadow_spread
                + config::design().launcher.shadow_extra_padding;
            let frame = shell.render_launcher(&theme, &mut fonts, w as f32, h as f32, 1.0, None);
            let (drawer_layer, _) = frame.pieces.last().unwrap();
            let drawer_surface = raster(
                drawer_layer.buffer.clone(),
                (drawer.w + 2.0 * drawer_pad).ceil() as u32,
                (drawer.h + 2.0 * drawer_pad).ceil() as u32,
            );
            let mut drawer_canvas = Surface {
                pixmap: home_canvas.clone(),
            };
            drawer_canvas.blit(
                &drawer_surface,
                SkTransform::from_translate(drawer.x - drawer_pad, drawer.y - drawer_pad),
                1.0,
                None,
            );
            drawer_canvas
                .pixmap
                .save_png(format!("/tmp/materialand-tablet-{name}-{w}-launcher.png"))
                .unwrap();
            let r = quick_settings_rect(w as f32, h as f32);
            let pad = config::design().shadow.padding;
            let frame = shell.render_quick_settings(
                &theme,
                &mut fonts,
                w as f32,
                h as f32,
                1.0,
                0.6,
                ThemeSpec::default().variant,
                None,
            );
            let (layer, _) = frame.pieces.last().unwrap();
            let panel = raster(
                layer.buffer.clone(),
                (r.w + 2.0 * pad).ceil() as u32,
                (r.h + 2.0 * pad).ceil() as u32,
            );
            canvas.blit(
                &panel,
                SkTransform::from_translate(r.x - pad, r.y - pad),
                1.0,
                None,
            );
            canvas
                .pixmap
                .save_png(format!("/tmp/materialand-tablet-{name}-{w}-shade.png"))
                .unwrap();
            shell.notifications_open = true;
            let frame = shell.render_quick_settings(
                &theme,
                &mut fonts,
                w as f32,
                h as f32,
                1.0,
                0.6,
                ThemeSpec::default().variant,
                None,
            );
            let (layer, _) = frame.pieces.last().unwrap();
            let panel = raster(
                layer.buffer.clone(),
                (r.w + 2.0 * pad).ceil() as u32,
                (r.h + 2.0 * pad).ceil() as u32,
            );
            let mut canvas = Surface {
                pixmap: home_canvas,
            };
            canvas.blit(
                &panel,
                SkTransform::from_translate(r.x - pad, r.y - pad),
                1.0,
                None,
            );
            canvas
                .pixmap
                .save_png(format!(
                    "/tmp/materialand-tablet-{name}-{w}-notifications.png"
                ))
                .unwrap();
        }
    }
}
