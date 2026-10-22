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
#[ignore = "writes design previews under target/visual-audit for manual visual review"]
fn write_tablet_design_previews() {
    std::fs::create_dir_all("target/visual-audit").unwrap();
    let mut fonts = Fonts::load().unwrap();
    let apps: Vec<_> = [
        ("Browser", "public"),
        ("Terminal", "terminal"),
        ("Code", "code"),
        ("Files", "folder"),
        ("Music", "music_note"),
        ("Photos", "photo"),
        ("Mail", "mail"),
        ("Calculator", "calculate"),
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
    for (w, h) in [(320, 640), (640, 480), (1280, 800), (1920, 1080)] {
        for (name, theme) in [
            ("light", ThemeSpec::default().light()),
            ("dark", ThemeSpec::default().dark()),
        ] {
            let mut shell = Shell::new(apps.clone(), ThemeSpec::default());
            shell.clamp_home_page(w as f32, h as f32);
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
                shell.toggle_home_app(app.clone()).unwrap();
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
            let frame = shell.render_home(&theme, &mut fonts, w as f32, h as f32);
            for (i, (layer, alpha)) in frame.pieces.iter().enumerate() {
                let size = if i == 0 {
                    (area.w.ceil() as u32, area.h.ceil() as u32)
                } else {
                    let side = config::design().launcher.tile_size.ceil() as u32;
                    (side, side)
                };
                paint_preview_layer(&mut canvas, layer, *alpha, size);
            }
            let home_canvas = canvas.pixmap.clone();
            let wallpaper = shell.wallpapers.last().unwrap().surface.pixmap.clone();
            write_recent_preview(&mut shell, &mut fonts, &theme, &wallpaper, name, w, h);
            paint_preview_bars(&mut canvas, &mut shell, &mut fonts, &theme, w, h);
            canvas
                .pixmap
                .save_png(format!(
                    "target/visual-audit/materialand-tablet-{name}-{w}-home.png"
                ))
                .unwrap();
            let drawer = launcher_card(w as f32, h as f32);
            let drawer_pad = config::design().launcher.shadow_spread
                + config::design().launcher.shadow_extra_padding;
            let frame = shell.render_launcher(&theme, &mut fonts, w as f32, h as f32, 1.0, None);
            let mut drawer_canvas = Surface {
                pixmap: home_canvas.clone(),
            };
            for (i, (layer, alpha)) in frame.pieces.iter().enumerate() {
                let size = if i == 0 {
                    (1, 1)
                } else if i == 1 {
                    (
                        (drawer.w + 2.0 * drawer_pad).ceil() as u32,
                        (drawer.h + 2.0 * drawer_pad).ceil() as u32,
                    )
                } else if layer.source.is_some() {
                    let side = config::design().launcher.tile_size.ceil() as u32;
                    (side, side)
                } else {
                    let surface = &shell.launcher_scrollbar.as_ref().unwrap().surface;
                    (surface.width(), surface.pixmap.height())
                };
                paint_preview_layer(&mut drawer_canvas, layer, *alpha, size);
            }
            paint_preview_bars(&mut drawer_canvas, &mut shell, &mut fonts, &theme, w, h);
            drawer_canvas
                .pixmap
                .save_png(format!(
                    "target/visual-audit/materialand-tablet-{name}-{w}-launcher.png"
                ))
                .unwrap();
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
            let mut canvas = Surface {
                pixmap: home_canvas.clone(),
            };
            for (i, (layer, alpha)) in frame.pieces.iter().enumerate() {
                let size = if i == 0 {
                    (1, 1)
                } else {
                    let surface = if i >= 3 {
                        &shell.notice_layer.as_ref().unwrap().surface
                    } else {
                        &shell.qs_layer.as_ref().unwrap().surface
                    };
                    (surface.width(), surface.pixmap.height())
                };
                paint_preview_layer(&mut canvas, layer, *alpha, size);
            }
            paint_preview_bars(&mut canvas, &mut shell, &mut fonts, &theme, w, h);
            canvas
                .pixmap
                .save_png(format!(
                    "target/visual-audit/materialand-tablet-{name}-{w}-shade.png"
                ))
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
            let mut canvas = Surface {
                pixmap: home_canvas,
            };
            for (i, (layer, alpha)) in frame.pieces.iter().enumerate() {
                let size = if i == 0 {
                    (1, 1)
                } else {
                    let surface = if i >= 3 {
                        &shell.notice_layer.as_ref().unwrap().surface
                    } else {
                        &shell.qs_layer.as_ref().unwrap().surface
                    };
                    (surface.width(), surface.pixmap.height())
                };
                paint_preview_layer(&mut canvas, layer, *alpha, size);
            }
            paint_preview_bars(&mut canvas, &mut shell, &mut fonts, &theme, w, h);
            canvas
                .pixmap
                .save_png(format!(
                    "target/visual-audit/materialand-tablet-{name}-{w}-notifications.png"
                ))
                .unwrap();
        }
    }
}

fn write_recent_preview(
    shell: &mut Shell,
    fonts: &mut Fonts,
    theme: &Theme,
    wallpaper: &tiny_skia::Pixmap,
    name: &str,
    w: u32,
    h: u32,
) {
    let mut canvas = Surface {
        pixmap: wallpaper.clone(),
    };
    let d = &config::design().overview;
    canvas.fill_round_rect(
        0.0,
        0.0,
        w as f32,
        h as f32,
        0.0,
        Theme::with_alpha(theme.scrim, d.scrim_alpha),
    );
    let overview = crate::desktop::overview::Overview::default();
    let size = overview.card_size(w as f32, h as f32);
    let header = shell.overview_header(theme, fonts, w as f32, "Recent apps");
    canvas.blit(
        &raster(header, w, d.header_height.ceil() as u32),
        SkTransform::identity(),
        1.0,
        None,
    );
    for (index, title) in ["Browser", "Terminal", "Files", "Music", "Mail", "Code"]
        .into_iter()
        .enumerate()
    {
        let r = overview.card(w as f32, h as f32, index);
        let mut card = super::overview::paint_recent_card(
            theme,
            fonts,
            &crate::desktop::WindowMeta {
                title: title.into(),
                app_id: title.to_lowercase(),
            },
            size,
            false,
            index == 0,
        );
        fonts.draw_text_centered(
            &mut card,
            (size.0 / 2.0, d.app_header_height + size.1 / 2.0),
            TextStyle::title_medium(),
            "Preview client",
            theme.on_surface_variant,
        );
        let scale = r.w / size.0;
        canvas.blit(
            &card,
            SkTransform::from_scale(scale, scale)
                .post_translate(r.x, r.y - d.app_header_height * scale),
            1.0,
            None,
        );
    }
    paint_preview_bars(&mut canvas, shell, fonts, theme, w, h);
    canvas
        .pixmap
        .save_png(format!(
            "target/visual-audit/materialand-tablet-{name}-{w}-recents.png"
        ))
        .unwrap();
}

fn paint_preview_layer(canvas: &mut Surface, layer: &LayerFrame, alpha: f32, size: (u32, u32)) {
    let image = raster(layer.buffer.clone(), size.0, size.1);
    let source = layer.source.unwrap_or(RectF {
        x: 0.0,
        y: 0.0,
        w: size.0 as f32,
        h: size.1 as f32,
    });
    let (w, h) = layer
        .size
        .map(|s| (s.w as f32, s.h as f32))
        .unwrap_or((source.w, source.h));
    let (sx, sy) = (w / source.w, h / source.h);
    let (x, y) = (layer.loc.x as f32, layer.loc.y as f32);
    let clip = round_rect_path(x, y, w, h, 0.0);
    canvas.blit(
        &image,
        SkTransform::from_scale(sx, sy).post_translate(x - source.x * sx, y - source.y * sy),
        alpha,
        Some(&clip),
    );
}

fn paint_preview_bars(
    canvas: &mut Surface,
    shell: &mut Shell,
    fonts: &mut Fonts,
    theme: &Theme,
    w: u32,
    _h: u32,
) {
    let (buffer, _) = shell.render_statusbar(theme, fonts, w as f32);
    canvas.blit(
        &raster(buffer, w, config::design().statusbar.height.ceil() as u32),
        SkTransform::identity(),
        1.0,
        None,
    );
}

#[test]
#[ignore = "writes the actual installed app catalog for local icon review"]
fn write_installed_icons_preview() {
    let apps = crate::config::scan_apps();
    let mut selected: Vec<_> = apps
        .iter()
        .filter(|a| {
            let n = a.name.to_lowercase();
            n.contains("firefox")
                || n.contains("cursor")
                || n.contains("code")
                || n.contains("terminal")
        })
        .cloned()
        .collect();
    for app in apps {
        if selected.len() >= 32 {
            break;
        }
        if !selected.iter().any(|a| a.identity() == app.identity()) {
            selected.push(app);
        }
    }
    let mut shell = Shell::new(selected, ThemeSpec::default());
    let loaded = shell
        .apps
        .iter()
        .filter(|a| shell.app_icons.get(&a.icon).is_some())
        .count();
    println!(
        "Loaded original icons for {loaded}/{} installed apps",
        shell.apps.len()
    );
    assert!(loaded > 0);
    for app in &shell.apps {
        if app.name.to_lowercase().contains("firefox") || app.name.to_lowercase().contains("cursor")
        {
            assert!(
                shell.app_icons.get(&app.icon).is_some(),
                "missing installed icon: {} ({})",
                app.name,
                app.icon
            );
        }
    }
    let mut fonts = Fonts::load().unwrap();
    let theme = ThemeSpec::default().light();
    let (w, h) = (1280, 800);
    let frame = shell.render_launcher(&theme, &mut fonts, w as f32, h as f32, 1.0, None);
    let mut canvas = Surface::new(w, h);
    canvas.clear(theme.surface_container_low);
    for (i, (layer, alpha)) in frame.pieces.iter().enumerate() {
        let size = if i == 0 {
            (1, 1)
        } else if i == 1 {
            let s = &shell.launcher_layer.as_ref().unwrap().surface;
            (s.width(), s.pixmap.height())
        } else if i == frame.pieces.len() - 1
            && launcher_scroll_max(w as f32, h as f32, shell.apps.len()) > 0.0
        {
            let s = &shell.launcher_scrollbar.as_ref().unwrap().surface;
            (s.width(), s.pixmap.height())
        } else {
            let size = config::design().launcher.tile_size.ceil() as u32;
            (size, size)
        };
        paint_preview_layer(&mut canvas, layer, *alpha, size);
    }
    paint_preview_bars(&mut canvas, &mut shell, &mut fonts, &theme, w, h);
    std::fs::create_dir_all("target/visual-audit").unwrap();
    canvas
        .pixmap
        .save_png("target/visual-audit/installed-app-icons.png")
        .unwrap();
}
