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
    let s = Shell::new(vec![], ThemeSpec::DEFAULT);
    assert_eq!(s.pinned.len(), 1);
    let s = Shell::new(
        vec![app("Foot Terminal"), app("Firefox")],
        ThemeSpec::DEFAULT,
    );
    assert_eq!(s.pinned.len(), 1);
    assert_eq!(s.pinned[0].name, "Foot Terminal");
}

#[test]
fn terminal_command_follows_discovered_app() {
    let s = Shell::new(
        vec![app("Kitty Terminal"), app("Firefox")],
        ThemeSpec::DEFAULT,
    );
    assert_eq!(s.terminal, vec!["/bin/true".to_string()]);
    let s = Shell::new(vec![], ThemeSpec::DEFAULT);
    assert!(!s.terminal.is_empty());
}

#[test]
fn kitty_wins_over_other_terminals() {
    let s = Shell::new(
        vec![app("Foot Terminal"), app("kitty"), app("Firefox")],
        ThemeSpec::DEFAULT,
    );
    assert_eq!(s.pinned[0].name, "kitty");
    let s = Shell::new(
        vec![app("Foot Terminal"), app("Alacritty"), app("Firefox")],
        ThemeSpec::DEFAULT,
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
    assert!(card.h >= POWER_ACTIONS.len() as f32 * POWER_ROW_H);
}

#[test]
fn topbar_hit_buttons() {
    let w = 1000.0;
    assert_eq!(topbar_hit(w, pt(30.0, 30.0)), Some(TopAction::Launcher));
    assert_eq!(topbar_hit(w, pt(960.0, 30.0)), Some(TopAction::Power));
    assert_eq!(topbar_hit(w, pt(910.0, 30.0)), Some(TopAction::Theme));
    assert_eq!(topbar_hit(w, pt(860.0, 30.0)), Some(TopAction::Brightness));
    assert_eq!(topbar_hit(w, pt(500.0, 30.0)), None);
    assert_eq!(topbar_hit(w, pt(300.0, 5.0)), None);
    assert_eq!(topbar_hit(w, pt(30.0, 40.0)), None, "below the bar");
}

#[test]
fn quick_settings_tiles_have_actionable_hit_regions() {
    let (w, h) = (1000.0, 800.0);
    let card = quick_settings_rect(w, h);
    let tile_w = (card.w - 44.0) / 2.0;
    let theme_center = pt(
        (card.x + 16.0 + tile_w / 2.0) as f64,
        (card.y + 48.0 + 36.0) as f64,
    );
    let night_center = pt(
        (card.x + 16.0 + tile_w + 12.0 + tile_w / 2.0) as f64,
        (card.y + 48.0 + 36.0) as f64,
    );
    assert_eq!(
        quick_settings_hit(w, h, theme_center, 1.0),
        Some(QuickAction::Theme)
    );
    assert_eq!(
        quick_settings_hit(w, h, night_center, 1.0),
        Some(QuickAction::NightLight)
    );
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
    assert!(rect.h >= QS_SHEET_HEIGHT - 1.0);
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
    assert_eq!(launcher_hit(w, h, 30, pt(10.0, 10.0)), LauncherHit::Scrim);
    assert_eq!(launcher_hit(w, h, 30, pt(500.0, 90.0)), LauncherHit::Input);
    assert_eq!(
        launcher_hit(w, h, 1, pt(200.0, 190.0)),
        LauncherHit::Tile(0)
    );
    assert_eq!(
        launcher_hit(w, h, 30, pt(200.0, 590.0)),
        LauncherHit::Inside
    );
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
        matches!(decor_hit(&l, pt(300.0, 170.0)), DecorHit::Resize(e) if e.top),
        "the top edge must be resizable, not only movable"
    );
    assert_eq!(decor_hit(&l, pt(300.0, 350.0)), DecorHit::None);
}

#[test]
fn press_visuals_animate_and_settle() {
    let mut shell = Shell::new(vec![], ThemeSpec::DEFAULT);
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
    let mut shell = Shell::new(vec![], ThemeSpec::DEFAULT);
    assert!(shell.switch_spring(QuickAction::NightLight) < 0.5);
    shell.toggle_night_light();
    for _ in 0..180 {
        shell.tick(1.0 / 60.0);
    }
    assert!(shell.switch_spring(QuickAction::NightLight) > 0.99);
}

#[test]
fn reduce_motion_snaps_the_switch() {
    let mut shell = Shell::new(vec![], ThemeSpec::DEFAULT);
    shell.set_reduce_motion(true);
    shell.toggle_night_light();
    assert_eq!(shell.switch_spring(QuickAction::NightLight), 1.0);
    assert!(!shell.tick(1.0 / 60.0), "nothing may be left animating");
}

#[test]
fn reduce_motion_engages_its_own_switch() {
    let mut shell = Shell::new(vec![], ThemeSpec::DEFAULT);
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
    let mut shell = Shell::new(vec![], ThemeSpec::DEFAULT);
    let old = crate::theme::ThemeSpec::DEFAULT.dark();
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
fn launcher_layer_caches_across_the_animation() {
    let mut shell = Shell::new(vec![], ThemeSpec::DEFAULT);
    let mut fonts = crate::graphics::text::Fonts::load().expect("test system font");
    let theme = crate::theme::ThemeSpec::DEFAULT.dark();
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
    let mut shell = Shell::new(vec![], ThemeSpec::DEFAULT);
    let mut fonts = crate::graphics::text::Fonts::load().expect("test system font");
    let theme = crate::theme::ThemeSpec::DEFAULT.dark();
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
