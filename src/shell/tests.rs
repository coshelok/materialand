use super::*;
use smithay::utils::Size;

fn pt(x: f64, y: f64) -> Point<f64, Logical> {
    Point::from((x, y))
}

fn app(name: &str) -> AppEntry {
    AppEntry {
        name: name.into(),
        exec: "/bin/true".into(),
        icon: String::new(),
        terminal: false,
        desktop_file: None,
    }
}

#[test]
fn pinned_has_exactly_one_terminal() {
    let s = Shell::new(vec![], ThemeSpec::default());
    assert_eq!(s.pinned.len(), 1);
    let s = Shell::new(
        vec![app("Foot Terminal"), app("Firefox")],
        ThemeSpec::default(),
    );
    assert_eq!(s.pinned.len(), 1);
    assert_eq!(s.pinned[0].name, "Foot Terminal");
}

#[test]
fn terminal_command_follows_discovered_app() {
    let s = Shell::new(
        vec![app("Kitty Terminal"), app("Firefox")],
        ThemeSpec::default(),
    );
    assert_eq!(s.terminal, vec!["/bin/true".to_string()]);
    let s = Shell::new(vec![], ThemeSpec::default());
    assert!(!s.terminal.is_empty());
}

#[test]
fn kitty_wins_over_other_terminals() {
    let s = Shell::new(
        vec![app("Foot Terminal"), app("kitty"), app("Firefox")],
        ThemeSpec::default(),
    );
    assert_eq!(s.pinned[0].name, "kitty");
    let s = Shell::new(
        vec![app("Foot Terminal"), app("Alacritty"), app("Firefox")],
        ThemeSpec::default(),
    );
    assert_eq!(s.pinned[0].name, "Alacritty");
}

#[test]
fn session_menu_rows_hit_their_actions() {
    let w = 1000.0;
    for i in 0..POWER_ACTIONS.len() {
        let row = power_row_rect(w, i);
        let center = pt((row.x + row.w / 2.0) as f64, (row.y + row.h / 2.0) as f64);
        assert_eq!(power_row_at(w, center), Some(i));
    }
    let card = power_menu_rect(w);
    assert_eq!(
        power_row_at(w, pt((card.x + 2.0) as f64, (card.y + 2.0) as f64)),
        None
    );
    assert_eq!(power_row_at(w, pt(500.0, 400.0)), None);
}

#[test]
fn session_menu_sits_below_the_user_button() {
    let w = 1000.0;
    let bar = topbar_rect(w);
    let button = topbar_button_rect(w, TopAction::Power);
    let card = power_menu_rect(w);
    assert!(card.y >= bar.bottom());
    assert!(
        (card.right() - button.right()).abs() < 0.5,
        "right edge follows the button"
    );
    assert!(card.h >= POWER_ACTIONS.len() as f32 * crate::config::design().power.row_height);
}

#[test]
fn topbar_hit_buttons() {
    let w = 1000.0;
    assert_eq!(topbar_hit(w, pt(30.0, 30.0)), Some(TopAction::Home));
    assert_eq!(topbar_hit(w, pt(960.0, 30.0)), Some(TopAction::Power));
    assert_eq!(topbar_hit(w, pt(910.0, 30.0)), Some(TopAction::Theme));
    assert_eq!(topbar_hit(w, pt(860.0, 30.0)), Some(TopAction::Brightness));
    assert_eq!(topbar_hit(w, pt(500.0, 30.0)), None);
    assert_eq!(topbar_hit(w, pt(300.0, 5.0)), None);
    assert_eq!(topbar_hit(w, pt(30.0, 64.0)), None, "below the bar");
}

#[test]
fn quick_settings_tiles_have_actionable_hit_regions() {
    let (w, h) = (1000.0, 800.0);
    for action in [
        QuickAction::Theme,
        QuickAction::NightLight,
        QuickAction::ReduceMotion,
        QuickAction::Brightness,
        QuickAction::Wifi,
        QuickAction::Bluetooth,
    ] {
        let r = quick_setting_tile_rect(w, h, quick_action_index(action));
        let center = pt((r.x + r.w / 2.0) as f64, (r.y + r.h / 2.0) as f64);
        assert_eq!(quick_settings_hit(w, h, center, 1.0), Some(action));
    }
}

#[test]
fn palette_chips_map_to_variants() {
    let (w, h) = (1000.0, 800.0);
    for i in 0..VARIANTS.len() {
        let chip =
            palette_chip_rect(w, h, i).unwrap_or_else(|| panic!("chip {i} must be reachable"));
        let center = pt(
            (chip.x + chip.w / 2.0) as f64,
            (chip.y + chip.h / 2.0) as f64,
        );
        assert_eq!(palette_chip_hit(w, h, center), Some(i));
    }
}

#[test]
fn quick_settings_sheet_fits_palette_grid() {
    let (w, h) = (1000.0, 800.0);
    let rect = quick_settings_rect(w, h);
    assert!(rect.h >= crate::config::design().quick_settings.height - 1.0);
    let last = palette_chip_rect(w, h, VARIANTS.len() - 1).expect("last chip fits");
    assert!(last.bottom() <= rect.bottom());
}

#[test]
fn brightness_hit_maps_and_clamps() {
    let w = 1000.0;
    let card = brightness_popup(w);
    let track = brightness_track(w);
    let cy = (card.y + card.h / 2.0) as f64;
    let lo = brightness_hit(w, pt(track.x as f64, cy)).unwrap();
    let hi = brightness_hit(w, pt(track.right() as f64, cy)).unwrap();
    assert!((lo - BRIGHTNESS_MIN).abs() < 1e-4, "left edge is dimmest");
    assert!(
        (hi - BRIGHTNESS_MAX).abs() < 1e-4,
        "right edge is brightest"
    );
    // A point outside the card is not a hit.
    assert!(brightness_hit(w, pt(10.0, 500.0)).is_none());
}

#[test]
fn dock_hit_indexes() {
    let (w, h) = (1000.0, 600.0);
    assert_eq!(dock_hit(w, h, 2, pt(460.0, 540.0)), Some(0));
    assert_eq!(dock_hit(w, h, 2, pt(520.0, 540.0)), Some(1));
    assert_eq!(dock_hit(w, h, 2, pt(500.0, 540.0)), None);
    assert_eq!(dock_hit(w, h, 2, pt(460.0, 100.0)), None);
}

#[test]
fn dock_item_rect_matches_hit() {
    let (w, h) = (1000.0, 600.0);
    for i in 0..3 {
        let rect = dock_item_rect(w, h, 3, i);
        let center = pt(
            (rect.x + rect.w / 2.0) as f64,
            (rect.y + rect.h / 2.0) as f64,
        );
        assert_eq!(dock_hit(w, h, 3, center), Some(i));
    }
}

#[test]
fn launcher_hit_regions() {
    let (w, h) = (1000.0, 700.0);
    assert_eq!(
        launcher_hit(w, h, 30, 0.0, pt(10.0, 10.0)),
        LauncherHit::Scrim
    );
    let search = search_rect(w, h);
    assert_eq!(
        launcher_hit(
            w,
            h,
            30,
            0.0,
            pt(
                f64::from(search.x + search.w / 2.0),
                f64::from(search.y + search.h / 2.0)
            )
        ),
        LauncherHit::Input
    );
    let origin = launcher_grid_origin(w, h);
    let columns = launcher_columns(launcher_card(w, h).w);
    for slot in [0, columns * 2 + 1] {
        let (x, y) = launcher_tile_pos(origin, slot, columns);
        assert_eq!(
            launcher_hit(w, h, 30, 0.0, pt(f64::from(x + 20.0), f64::from(y + 20.0))),
            LauncherHit::Tile(slot)
        );
    }
}

#[test]
fn tablet_drawer_fills_the_area_below_the_status_strip() {
    for (w, h) in [(320.0, 240.0), (640.0, 480.0), (1280.0, 800.0)] {
        let r = launcher_card(w, h);
        assert!(r.x >= 0.0 && r.right() <= w);
        assert!(r.y >= top_inset() && r.bottom() <= h - bottom_inset());
    }
}

#[test]
fn decor_hit_regions() {
    let geom = Rectangle::new(Point::from((100, 200)), Size::from((400, 300)));
    let l = decor_layout(geom);
    assert_eq!(decor_hit(&l, pt(479.0, 178.0)), DecorHit::Close);
    assert_eq!(decor_hit(&l, pt(300.0, 175.0)), DecorHit::Move);
    assert!(matches!(decor_hit(&l, pt(102.0, 350.0)), DecorHit::Resize(e) if e.left));
    assert!(matches!(decor_hit(&l, pt(497.0, 350.0)), DecorHit::Resize(e) if e.right));
    assert!(matches!(decor_hit(&l, pt(300.0, 496.0)), DecorHit::Resize(e) if e.bottom));
    assert!(
        matches!(decor_hit(&l, pt(300.0, (l.frame.y + 2.0) as f64)), DecorHit::Resize(e) if e.top),
        "the top edge must be resizable, not only movable"
    );
    assert_eq!(decor_hit(&l, pt(300.0, 350.0)), DecorHit::None);
}

#[test]
fn press_visuals_animate_and_settle() {
    let mut shell = Shell::new(vec![], ThemeSpec::default());
    let key = PressKey::Top(TopAction::Theme);
    shell.press_begin(key, (10.0, 10.0), 12.0, 0.86);
    // The scale spring is already aiming at the pressed value, so one tick
    // is enough to move it off the rest pose.
    shell.tick(1.0 / 60.0);
    assert!(shell.press_scale(key) < 1.0, "press squeezes the control");
    assert!(shell.ripple_draw(key).is_some(), "press starts a ripple");
    shell.press_end(key);
    let mut steps = 0;
    while !shell.press.is_empty() {
        shell.tick(1.0 / 60.0);
        steps += 1;
        assert!(steps < 600, "press animation must settle");
    }
    assert_eq!(shell.press_scale(key), 1.0);
    assert!(shell.ripple_draw(key).is_none());
}

#[test]
fn toggle_night_light_animates_the_switch() {
    let mut shell = Shell::new(vec![], ThemeSpec::default());
    assert!(shell.switch_spring(QuickAction::NightLight) < 0.5);
    shell.toggle_night_light();
    for _ in 0..180 {
        shell.tick(1.0 / 60.0);
    }
    assert!(shell.switch_spring(QuickAction::NightLight) > 0.99);
}

#[test]
fn reduce_motion_snaps_the_switch() {
    let mut shell = Shell::new(vec![], ThemeSpec::default());
    shell.set_reduce_motion(true);
    shell.toggle_night_light();
    assert_eq!(shell.switch_spring(QuickAction::NightLight), 1.0);
    assert!(!shell.tick(1.0 / 60.0), "nothing may be left animating");
}

#[test]
fn reduce_motion_engages_its_own_switch() {
    let mut shell = Shell::new(vec![], ThemeSpec::default());
    assert!(shell.switch_spring(QuickAction::ReduceMotion) < 0.5);
    shell.set_reduce_motion(true);
    assert_eq!(
        shell.switch_spring(QuickAction::ReduceMotion),
        1.0,
        "the tile's switch must show the setting as on"
    );
    shell.set_reduce_motion(false);
    for _ in 0..240 {
        shell.tick(1.0 / 60.0);
    }
    assert!(
        shell.switch_spring(QuickAction::ReduceMotion) < 0.01,
        "switching back off must animate the switch down"
    );
}

#[test]
fn theme_fade_returns_both_endpoints_only_while_running() {
    let mut shell = Shell::new(vec![], ThemeSpec::default());
    let old = crate::theme::ThemeSpec::default().dark();
    assert!(shell.theme_fade().is_none(), "no fade when idle");
    shell.begin_theme_change(old);
    let (base, alpha) = shell.theme_fade().expect("fade while running");
    assert_eq!(base.cache_key(), old.cache_key(), "base is the old scheme");
    assert!(alpha < 1.0, "the new scheme starts fully transparent");
    for _ in 0..240 {
        shell.tick(1.0 / 60.0);
    }
    assert!(shell.theme_fade().is_none(), "the fade must end");
}

#[test]
fn truncate_adds_ellipsis() {
    assert_eq!(truncate("hello", 10), "hello");
    assert_eq!(truncate("hello world", 8), "hello w…");
    assert_eq!(truncate("abc", 0), "");
}

#[test]
fn launcher_columns_positive() {
    assert!(launcher_columns(720.0) >= 1);
    assert!(launcher_columns(10.0) >= 1);
}

#[test]
fn sheets_rebound_preserve_content_shape_and_reverse_without_a_jump() {
    for (w, h) in [(320.0, 640.0), (1280.0, 800.0), (3840.0, 2160.0)] {
        let mut shell = Shell::new(vec![], ThemeSpec::default());
        shell.launcher_open = true;
        shell.set_launcher_open(true);
        shell.set_quick_settings_open(true);
        let mut launcher_peak = 0.0_f32;
        let mut shade_peak = 0.0_f32;
        for _ in 0..240 {
            shell.tick(1.0 / 144.0);
            for placement in [
                shell.launcher_placement(w, h, shell.launcher_progress()),
                shell.quick_settings_placement(w, h, shell.quick_settings_progress()),
            ] {
                assert_eq!(placement.target.w, placement.source.w);
                assert_eq!(placement.target.h, placement.source.h);
                let icon = RectF {
                    x: 100.0,
                    y: 180.0,
                    w: 56.0,
                    h: 56.0,
                };
                let drawn = placement.map_rect(icon);
                let point =
                    placement.unmap(pt(f64::from(drawn.x + 28.0), f64::from(drawn.y + 28.0)));
                assert!((point.x - 128.0).abs() < 0.001 && (point.y - 208.0).abs() < 0.001);
            }
            launcher_peak = launcher_peak.max(
                launcher_card(w, h).y
                    - shell
                        .launcher_placement(w, h, shell.launcher_progress())
                        .target
                        .y,
            );
            shade_peak = shade_peak.max(
                shell
                    .quick_settings_placement(w, h, shell.quick_settings_progress())
                    .target
                    .y
                    - quick_settings_rect(w, h).y,
            );
        }
        assert!(
            launcher_peak > 3.0 && launcher_peak <= 16.01,
            "drawer rebound {launcher_peak}"
        );
        assert!(
            shade_peak > 3.0 && shade_peak <= 16.01,
            "shade rebound {shade_peak}"
        );
        assert_eq!(shell.launcher_progress(), 1.0);
        assert_eq!(shell.quick_settings_progress(), 1.0);
        shell.set_quick_settings_open(false);
        shell.tick(0.1);
        let before = shell.quick_settings_progress();
        shell.set_quick_settings_open(true);
        assert_eq!(
            shell.quick_settings_progress(),
            before,
            "retargeting must not reset position"
        );
        shell.set_reduce_motion(true);
        assert_eq!(shell.quick_settings_progress(), 1.0);
        shell.set_quick_settings_open(false);
        assert_eq!(shell.quick_settings_progress(), 0.0);
    }
}

#[test]
fn scrolling_reuses_drawer_textures_and_crops_tiles_at_the_fixed_header() {
    let mut shell = Shell::new(
        (0..100)
            .map(|i| {
                let mut a = app(&format!("App {i:03}"));
                a.desktop_file = Some(format!("/apps/{i}.desktop").into());
                a
            })
            .collect(),
        ThemeSpec::default(),
    );
    let mut fonts = Fonts::load().unwrap();
    let theme = ThemeSpec::default().light();
    shell.set_reduce_motion(true);
    let (w, h) = (640.0, 480.0);
    shell.render_launcher(&theme, &mut fonts, w, h, 1.0, None);
    let background = shell
        .launcher_layer
        .as_ref()
        .unwrap()
        .surface
        .pixmap
        .data()
        .as_ptr();
    let tile_id = shell.apps[0].identity();
    let tile = shell.app_tiles[&tile_id][0].surface.pixmap.data().as_ptr();
    shell.scroll_launcher(w, h, 30.0);
    let frame = shell.render_launcher(&theme, &mut fonts, w, h, 1.0, None);
    assert_eq!(
        shell
            .launcher_layer
            .as_ref()
            .unwrap()
            .surface
            .pixmap
            .data()
            .as_ptr(),
        background
    );
    assert_eq!(
        shell.app_tiles[&tile_id][0].surface.pixmap.data().as_ptr(),
        tile
    );
    let viewport = launcher_viewport(w, h);
    let pieces: Vec<_> = frame
        .pieces
        .iter()
        .filter(|(p, _)| p.source.is_some())
        .collect();
    assert!(!pieces.is_empty());
    assert!(pieces.iter().any(|(p, _)| p.source.unwrap().y > 0.0));
    for (piece, _) in pieces {
        assert!(piece.loc.y >= f64::from(viewport.y));
        assert!(
            piece.loc.y + f64::from(piece.size.unwrap().h) <= f64::from(viewport.bottom()) + 1.0
        );
    }
}

#[test]
fn shade_scroll_moves_only_the_body_and_keeps_its_raster_cache() {
    let mut shell = Shell::new(vec![], ThemeSpec::default());
    let mut fonts = Fonts::load().unwrap();
    let theme = ThemeSpec::default().dark();
    shell.set_reduce_motion(true);
    let first = shell.render_quick_settings(
        &theme,
        &mut fonts,
        640.0,
        480.0,
        1.0,
        0.5,
        ColorVariant::Neutral,
        None,
    );
    let cache = shell
        .qs_layer
        .as_ref()
        .unwrap()
        .surface
        .pixmap
        .data()
        .as_ptr();
    shell.scroll_shade(64.0, shade_scroll_max(640.0, 480.0));
    let second = shell.render_quick_settings(
        &theme,
        &mut fonts,
        640.0,
        480.0,
        1.0,
        0.5,
        ColorVariant::Neutral,
        None,
    );
    assert_eq!(
        cache,
        shell
            .qs_layer
            .as_ref()
            .unwrap()
            .surface
            .pixmap
            .data()
            .as_ptr()
    );
    assert_eq!(
        first.pieces[1].0.source.unwrap().y,
        second.pieces[1].0.source.unwrap().y
    );
    assert_eq!(first.pieces[2].0.loc, second.pieces[2].0.loc);
    assert_eq!(
        second.pieces[2].0.source.unwrap().y - first.pieces[2].0.source.unwrap().y,
        64.0
    );
}

#[test]
fn clearing_notifications_clamps_displayed_and_pending_scroll() {
    let mut shell = Shell::new(vec![], ThemeSpec::default());
    shell.notifications_open = true;
    shell.notifications.items = (0..12)
        .map(|id| crate::notifications::Notice {
            id,
            app: "Mail".into(),
            summary: "Message".into(),
            body: String::new(),
        })
        .collect();
    shell.set_reduce_motion(true);
    shell.scroll_shade(240.0, shell.shade_scroll_limit(640.0, 480.0));
    assert!(shell.shade_scroll > 0.0);
    shell.notifications.items.clear();
    assert!(shell.clamp_shade_scroll(640.0, 480.0));
    assert_eq!(shell.shade_scroll, 0.0);
    assert_eq!(shell.shade_scroll_target, 0.0);
    assert!(shell.shade_scroll_motion.settled());
    assert!(!shell.clamp_shade_scroll(640.0, 480.0));

    // A service update can arrive before the first animated scroll frame.
    shell.set_reduce_motion(false);
    shell.scroll_shade(200.0, 500.0);
    shell.clamp_shade_scroll(640.0, 480.0);
    shell.shade_scroll_motion.step(1.0 / 60.0);
    assert_eq!(shell.shade_scroll_motion.value(), 0.0);
}

#[test]
fn launcher_layer_caches_across_the_animation() {
    let mut shell = Shell::new(vec![], ThemeSpec::default());
    let mut fonts = crate::graphics::text::Fonts::load().expect("test system font");
    let theme = crate::theme::ThemeSpec::default().dark();
    let (w, h) = (1280.0, 800.0);
    let _ = shell.render_launcher(&theme, &mut fonts, w, h, 1.0, None);
    let key = shell
        .launcher_layer
        .as_ref()
        .expect("the layer is cached on the first render")
        .key;
    // Halfway through the open animation no press is active, so the very
    // same content layer must be reused instead of being re-rasterised
    // (which used to cost ~24 ms per frame).
    let _ = shell.render_launcher(&theme, &mut fonts, w, h, 0.5, None);
    assert_eq!(
        shell.launcher_layer.as_ref().unwrap().key,
        key,
        "the launcher content layer must be cached across the animation"
    );
}

#[test]
fn launcher_frame_scales_and_scrims_on_the_gpu_path() {
    let mut shell = Shell::new(vec![], ThemeSpec::default());
    let mut fonts = crate::graphics::text::Fonts::load().expect("test system font");
    let theme = crate::theme::ThemeSpec::default().dark();
    let (w, h) = (1280.0, 800.0);
    let frame = shell.render_launcher(&theme, &mut fonts, w, h, 1.0, None);
    // Scrim (stretched over the output) plus the scaled card layer.
    assert!(frame.pieces.len() >= 2);
    assert!(frame.pieces[0].0.size.is_some(), "the scrim is stretched");
    assert!(
        frame.pieces[1].0.size.is_some(),
        "the card layer carries a size override"
    );
    assert!((frame.pieces[1].1 - 1.0).abs() < 1e-3, "fully open");
}

#[test]
fn floating_dock_fits_small_outputs_and_large_window_lists() {
    for w in [320.0, 800.0, 1920.0] {
        for count in [1, 4, 20] {
            let dock = dock_rect(w, 800.0, count);
            assert!(dock.x >= 0.0 && dock.right() <= w);
            assert!(
                dock.bottom() < 800.0,
                "the dock floats above the bottom edge"
            );
            let mut right = dock.x;
            for i in 0..count {
                let icon = dock_item_rect(w, 800.0, count, i);
                assert!(icon.x >= right && icon.right() <= dock.right());
                right = icon.right();
            }
        }
    }
}

#[test]
fn reduced_motion_finishes_all_open_panels() {
    let mut shell = Shell::new(vec![], ThemeSpec::default());
    shell.launcher_open = true;
    shell.set_launcher_open(true);
    shell.set_power_open(true);
    shell.set_brightness_open(true);
    shell.set_quick_settings_open(true);
    shell.tick(0.05);
    shell.set_reduce_motion(true);
    assert_eq!(shell.power_progress(), 1.0);
    assert_eq!(shell.launcher_progress(), 1.0);
    assert_eq!(shell.brightness_progress(), 1.0);
    assert_eq!(shell.quick_settings_progress(), 1.0);
    assert!(!shell.tick(1.0 / 60.0));
}

#[test]
fn theme_reveal_uses_local_coordinates_for_floating_chrome() {
    let mut surface = Surface::new(100, 80);
    surface.clear(Color::WHITE);
    let cached = surface.to_memory_buffer();
    let (mut buffer, loc) = masked_from(
        &surface,
        Some(CircleMask {
            cx: 550.0,
            cy: 340.0,
            radius: 10.0,
        }),
        &cached,
        (500.0, 300.0).into(),
    );
    assert_eq!(loc, (540.0, 330.0).into());
    buffer
        .render()
        .draw(|pixels| {
            assert_eq!(pixels.len(), 20 * 20 * 4);
            assert_eq!(pixels[(10 * 20 + 10) * 4 + 3], 255);
            Ok::<_, ()>(Vec::new())
        })
        .unwrap();
}

#[test]
fn dock_grows_shrinks_and_retargets_without_jumping() {
    let mut shell = Shell::new(vec![], ThemeSpec::default());
    let entry = |name: &str| {
        DockItem::Launch(AppEntry {
            name: name.into(),
            exec: name.into(),
            icon: name.into(),
            terminal: false,
            desktop_file: Some(format!("/applications/{name}.desktop").into()),
        })
    };
    let small = vec![DockItem::Launcher, entry("a")];
    let large = vec![DockItem::Launcher, entry("a"), entry("b"), entry("c")];
    shell.sync_dock(1000.0, &small);
    let start = shell.dock_rect(1000.0, 800.0, small.len()).w;
    shell.sync_dock(1000.0, &large);
    assert_eq!(shell.dock_rect(1000.0, 800.0, large.len()).w, start);
    shell.tick(1.0 / 60.0);
    let growing = shell.dock_rect(1000.0, 800.0, large.len()).w;
    assert!(growing > start);
    let icon = shell.dock_item_rect(1000.0, 800.0, large.len(), 1);
    assert_eq!(
        shell.dock_hit(
            1000.0,
            800.0,
            large.len(),
            pt(
                (icon.x + icon.w / 2.0) as f64,
                (icon.y + icon.h / 2.0) as f64
            )
        ),
        Some(1)
    );
    shell.sync_dock(1000.0, &small);
    assert_eq!(
        shell.dock_rect(1000.0, 800.0, small.len()).w,
        growing,
        "retargeting preserves the current pose"
    );
    for _ in 0..180 {
        shell.tick(1.0 / 60.0);
    }
    assert!((shell.dock_rect(1000.0, 800.0, small.len()).w - start).abs() < 0.01);
    shell.sync_dock(1000.0, &large);
    shell.set_reduce_motion(true);
    assert_eq!(
        shell.dock_rect(1000.0, 800.0, large.len()).w,
        dock_rect(1000.0, 800.0, large.len()).w
    );
    assert!(!shell.dock_animating());
}

#[test]
fn launcher_scroll_reaches_last_app_and_does_not_scroll_the_search_field() {
    let apps = (0..100).map(|i| app(&format!("App {i:03}"))).collect();
    let mut shell = Shell::new(apps, ThemeSpec::default());
    shell.set_reduce_motion(true);
    let (w, h) = (1000.0, 700.0);
    let mut fonts = Fonts::load().unwrap();
    let theme = ThemeSpec::default().light();
    shell.render_launcher(&theme, &mut fonts, w, h, 1.0, None);
    let before = shell.launcher_layer.as_ref().unwrap().surface.duplicate();
    shell.scroll_launcher(w, h, 100000.0);
    let max = launcher_scroll_max(w, h, shell.apps.len());
    assert!(max > 0.0);
    assert_eq!(shell.launcher_scroll(), max);
    let card = launcher_card(w, h);
    let mut origin = launcher_grid_origin(w, h);
    origin.1 -= max;
    let (x, y) = launcher_tile_pos(origin, 99, launcher_columns(card.w));
    assert_eq!(
        launcher_hit(w, h, 100, max, pt((x + 56.0) as f64, (y + 56.0) as f64)),
        LauncherHit::Tile(99)
    );
    let search = search_rect(w, h);
    assert_eq!(
        launcher_hit(
            w,
            h,
            100,
            max,
            pt((search.x + 30.0) as f64, (search.y + 30.0) as f64)
        ),
        LauncherHit::Input
    );
    shell.render_launcher(&theme, &mut fonts, w, h, 1.0, None);
    let after = &shell.launcher_layer.as_ref().unwrap().surface;
    // Header raster rows are identical even after scrolling to the very end.
    let header_rows = (launcher_viewport(w, h).y - card.y + 20.0).floor() as usize;
    let bytes = header_rows * before.width() as usize * 4;
    assert_eq!(
        &before.pixmap.data()[..bytes],
        &after.pixmap.data()[..bytes]
    );
    shell.launcher_push('9');
    assert_eq!(
        shell.launcher_scroll(),
        0.0,
        "a new query starts at the top"
    );
}

#[test]
fn feedback_for_a_partially_visible_tile_is_clipped_to_the_grid() {
    let mut shell = Shell::new(
        (0..60).map(|i| app(&format!("App {i}"))).collect(),
        ThemeSpec::default(),
    );
    shell.set_reduce_motion(true);
    let (w, h) = (1000.0, 700.0);
    shell.scroll_launcher(w, h, 84.0);
    shell.press_begin(PressKey::Launcher(0), (56.0, 36.0), 64.0, 0.9);
    let mut fonts = Fonts::load().unwrap();
    let pieces = shell.launcher_press_surfaces(&mut fonts, &ThemeSpec::default().light(), w, h);
    assert_eq!(pieces.len(), 1);
    let viewport = launcher_viewport(w, h);
    for (surface, (x, y)) in pieces {
        assert!(x >= viewport.x && y >= viewport.y);
        assert!(x + surface.width() as f32 <= viewport.right());
        assert!(y + surface.pixmap.height() as f32 <= viewport.bottom());
        assert!(surface.pixmap.height() < crate::config::design().launcher.tile_size as u32);
    }
}

#[test]
fn manual_unpinning_including_the_last_pin_keeps_terminal_shortcuts() {
    let mut shell = Shell::new(vec![], ThemeSpec::default());
    let terminal = shell.terminal.clone();
    let initial = shell.pinned[0].clone();
    shell.toggle_pin(initial.clone()).unwrap();
    assert!(shell.pinned.is_empty());
    assert_eq!(shell.terminal, terminal);
    shell.toggle_pin(initial).unwrap();
    assert_eq!(shell.pinned.len(), 1);
}

#[test]
fn theme_transition_reuses_both_raster_endpoints_for_every_cached_sheet() {
    let spec = ThemeSpec::default();
    let mut shell = Shell::new(vec![], spec);
    let mut fonts = Fonts::load().expect("test system font");
    fn paint(sheet: usize, shell: &mut Shell, fonts: &mut Fonts, theme: &Theme) -> usize {
        match sheet {
            0 => {
                shell.render_launcher(theme, fonts, 1280.0, 800.0, 1.0, None);
            }
            1 => {
                shell.render_quick_settings(
                    theme,
                    fonts,
                    1280.0,
                    800.0,
                    1.0,
                    0.5,
                    ColorVariant::Neutral,
                    None,
                );
            }
            2 => {
                shell.render_brightness(theme, fonts, 1280.0, 0.5, 1.0, None);
            }
            3 => {
                shell.render_power_menu(theme, fonts, 1280.0, 1.0, None);
            }
            _ => {
                shell.render_calendar(theme, fonts, 1280.0, 800.0, None);
            }
        }
        let cache = match sheet {
            0 => &shell.launcher_layer,
            1 => &shell.qs_layer,
            2 => &shell.brightness_layer,
            3 => &shell.power_layer,
            _ => &shell.calendar_layer,
        };
        cache.as_ref().unwrap().surface.pixmap.data().as_ptr() as usize
    }
    for sheet in 0..5 {
        let light = paint(sheet, &mut shell, &mut fonts, &spec.light());
        let dark = paint(sheet, &mut shell, &mut fonts, &spec.dark());
        assert_ne!(light, dark);
        for _ in 0..3 {
            assert_eq!(paint(sheet, &mut shell, &mut fonts, &spec.light()), light);
            assert_eq!(paint(sheet, &mut shell, &mut fonts, &spec.dark()), dark);
        }
    }
}

#[test]
fn tablet_notifications_scroll_independently_and_keep_header_and_tiles_cached() {
    let mut shell = Shell::new(vec![], ThemeSpec::default());
    shell.set_reduce_motion(true);
    shell.notifications.items = (0..16)
        .map(|id| crate::notifications::Notice {
            id,
            app: "Mail".into(),
            summary: "Message".into(),
            body: String::new(),
        })
        .collect();
    let mut fonts = Fonts::load().unwrap();
    let theme = ThemeSpec::default().light();
    let first = shell.render_quick_settings(
        &theme,
        &mut fonts,
        1280.0,
        800.0,
        1.0,
        0.5,
        ColorVariant::Neutral,
        None,
    );
    let left = shell
        .qs_layer
        .as_ref()
        .unwrap()
        .surface
        .pixmap
        .data()
        .as_ptr();
    let right = shell
        .notice_layer
        .as_ref()
        .unwrap()
        .surface
        .pixmap
        .data()
        .as_ptr();
    shell.scroll_notifications(104.0, 1280.0, 800.0);
    let second = shell.render_quick_settings(
        &theme,
        &mut fonts,
        1280.0,
        800.0,
        1.0,
        0.5,
        ColorVariant::Neutral,
        None,
    );
    assert_eq!(shell.shade_scroll, 0.0);
    assert_eq!(
        left,
        shell
            .qs_layer
            .as_ref()
            .unwrap()
            .surface
            .pixmap
            .data()
            .as_ptr()
    );
    assert_eq!(
        right,
        shell
            .notice_layer
            .as_ref()
            .unwrap()
            .surface
            .pixmap
            .data()
            .as_ptr()
    );
    assert_eq!(first.pieces[3].0.source, second.pieces[3].0.source);
    assert_eq!(
        second.pieces[4].0.source.unwrap().y - first.pieces[4].0.source.unwrap().y,
        104.0
    );
    let pane = notification_panel(1280.0, 800.0);
    let point = (
        f64::from(pane.right() - config::design().quick_settings.padding - 10.0),
        f64::from(pane.y + notification_top(1280.0) + config::design().power.row_height + 20.0),
    )
        .into();
    assert_eq!(
        shell.notification_hit(1280.0, 800.0, point),
        Some(NotificationHit::Dismiss(14))
    );
    shell.notifications.items.clear();
    shell.notifications.revision += 1;
    shell.clamp_shade_scroll(1280.0, 800.0);
    assert_eq!(shell.notice_scroll.value(), 0.0);
}
