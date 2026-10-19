use super::*;
fn app(index: usize) -> AppEntry {
    AppEntry {
        name: format!("App {index}"),
        exec: "/bin/true".into(),
        icon: "apps".into(),
        terminal: false,
        desktop_file: Some(format!("/applications/app-{index}.desktop").into()),
    }
}
#[test]
fn home_shortcuts_survive_restart_independently_of_dock_pins() {
    let dir = std::env::temp_dir().join(format!("materialand-home-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("home-apps");
    std::fs::write(&path, "desktop:app-3.desktop\ndesktop:missing.desktop\n").unwrap();
    let mut shell = Shell::new((0..8).map(app).collect(), ThemeSpec::default());
    shell.load_home_at(path.clone()).unwrap();
    assert_eq!(shell.home.apps[0].identity(), app(3).identity());
    let dock: Vec<_> = shell.pinned.iter().map(AppEntry::identity).collect();
    shell.toggle_home_app(app(5)).unwrap();
    shell.toggle_home_app(app(3)).unwrap();
    assert_eq!(
        shell
            .pinned
            .iter()
            .map(AppEntry::identity)
            .collect::<Vec<_>>(),
        dock
    );
    let mut restarted = Shell::new((0..8).map(app).collect(), ThemeSpec::default());
    restarted.load_home_at(path.clone()).unwrap();
    assert_eq!(restarted.home.apps.len(), 1);
    assert_eq!(restarted.home.apps[0].identity(), app(5).identity());
    assert!(std::fs::read_to_string(&path)
        .unwrap()
        .contains("desktop:missing.desktop"));
    restarted.toggle_home_app(app(5)).unwrap();
    restarted.load_home_at(path).unwrap();
    assert!(restarted.home.apps.is_empty());
    std::fs::remove_dir_all(dir).unwrap();
}
#[test]
fn failed_home_save_does_not_add_a_shortcut() {
    let mut shell = Shell::new(vec![], ThemeSpec::default());
    shell.home.pins.set_path("/dev/null/home-apps".into());
    assert!(shell.toggle_home_app(app(1)).is_err());
    assert!(!shell.is_on_home(&app(1)) && shell.home.apps.is_empty());
}
#[test]
fn every_home_shortcut_can_be_reached_and_pages_clamp_after_removal() {
    let mut shell = Shell::new(vec![], ThemeSpec::default());
    shell.set_reduce_motion(true);
    for i in 0..95 {
        shell.toggle_home_app(app(i)).unwrap();
    }
    let (w, h) = (1280.0, 800.0);
    let layout = home_layout(w, h);
    for i in 0..shell.home.apps.len() {
        let page = i / layout.per_page;
        shell.turn_home_page(w, h, page as i32 - shell.home.page as i32);
        let r = shell.home_tile_rect(w, h, i);
        assert_eq!(
            shell.home_hit(
                w,
                h,
                (f64::from(r.x + r.w / 2.0), f64::from(r.y + r.h / 2.0)).into()
            ),
            Some(HomeHit::App(i))
        );
    }
    for i in 1..95 {
        shell.toggle_home_app(app(i)).unwrap();
    }
    shell.clamp_home_page(w, h);
    assert_eq!(shell.home.page, 0);
    let r = shell.home_tile_rect(w, h, 0);
    assert!(r.x >= layout.grid.x && r.right() <= layout.grid.right());
}
#[test]
fn ticking_does_not_cancel_home_clicks_and_moving_icons_use_visible_positions() {
    let mut shell = Shell::new(vec![], ThemeSpec::default());
    for i in 0..50 {
        shell.toggle_home_app(app(i)).unwrap();
    }
    let (w, h) = (1280.0, 800.0);
    shell.home.pressed = Some(HomeHit::App(0));
    shell.clamp_home_page(w, h);
    shell.tick(1.0 / 60.0);
    assert_eq!(shell.home.pressed, Some(HomeHit::App(0)));
    shell.turn_home_page(w, h, 1);
    for _ in 0..10 {
        shell.tick(1.0 / 120.0);
    }
    let i = home_layout(w, h).per_page;
    let r = shell.home_tile_rect(w, h, i);
    let point = (f64::from(r.x + r.w / 2.0), f64::from(r.y + r.h / 2.0)).into();
    assert!(home_layout(w, h).grid.contains(point));
    assert_eq!(shell.home_hit(w, h, point), Some(HomeHit::App(i)));
}

#[test]
fn home_keeps_both_theme_textures_without_rasterizing_on_each_transition_frame() {
    let spec = ThemeSpec::default();
    let mut shell = Shell::new(vec![], spec);
    let mut fonts = Fonts::load().unwrap();
    let mut pointers = Vec::new();
    for theme in [spec.light(), spec.dark(), spec.light(), spec.dark()] {
        shell.render_home(&theme, &mut fonts, 1280.0, 800.0, None);
        pointers.push(
            shell
                .home
                .cache
                .as_ref()
                .unwrap()
                .surface
                .pixmap
                .data()
                .as_ptr(),
        );
    }
    assert_ne!(pointers[0], pointers[1]);
    assert_eq!(pointers[0], pointers[2]);
    assert_eq!(pointers[1], pointers[3]);
}
