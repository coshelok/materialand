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
fn center(r: RectF) -> Point<f64, Logical> {
    (f64::from(r.x + r.w / 2.0), f64::from(r.y + r.h / 2.0)).into()
}
#[test]
fn sparse_positions_and_missing_apps_survive_restart_without_compaction() {
    let dir = std::env::temp_dir().join(format!("materialand-workspace-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("home-apps");
    std::fs::write(
        &path,
        "desktop:app-0.desktop\ndesktop:missing.desktop\ndesktop:app-1.desktop\n",
    )
    .unwrap();
    let mut s = Shell::new((0..6).map(app).collect(), ThemeSpec::default());
    s.load_home_at(path.clone()).unwrap();
    let layout = home_layout(1280.0, 800.0);
    let far = layout.per_page - 1;
    s.place_home_app(app(0), far).unwrap();
    let position = s.home_tile_rect(1280.0, 800.0, 0);
    s.toggle_home_app(app(1)).unwrap();
    let mut restarted = Shell::new((0..6).map(app).collect(), ThemeSpec::default());
    restarted.load_home_at(path.clone()).unwrap();
    assert_eq!(restarted.home.apps.len(), 1);
    assert_eq!(position, restarted.home_tile_rect(1280.0, 800.0, 0));
    assert_eq!(
        restarted.home_drop_index(1280.0, 800.0, center(position)),
        Some(far)
    );
    assert!(restarted.home.workspace.contains("desktop:missing.desktop"));
    assert_eq!(
        std::fs::read_to_string(path).unwrap(),
        "desktop:app-0.desktop\ndesktop:missing.desktop\ndesktop:app-1.desktop\n"
    );
    assert!(dir.join("workspace.json").exists());
    std::fs::remove_dir_all(dir).unwrap();
}
#[test]
fn occupied_drop_swaps_only_two_apps_and_failed_write_changes_nothing() {
    let mut s = Shell::new(vec![], ThemeSpec::default());
    for i in 0..3 {
        s.toggle_home_app(app(i)).unwrap();
    }
    let old = s.home_reorder_snapshot(1280.0, 800.0);
    let target = s
        .home_drop_index(1280.0, 800.0, center(old[&app(1).identity()]))
        .unwrap();
    s.place_home_app(app(0), target).unwrap();
    assert_eq!(s.home_tile_rect(1280.0, 800.0, 0), old[&app(1).identity()]);
    assert_eq!(s.home_tile_rect(1280.0, 800.0, 1), old[&app(0).identity()]);
    assert_eq!(s.home_tile_rect(1280.0, 800.0, 2), old[&app(2).identity()]);
    let saved = s.home_reorder_snapshot(1280.0, 800.0);
    s.home.workspace.set_path("/dev/null/workspace.json".into());
    assert!(s.place_home_app(app(2), target).is_err());
    assert!(s.toggle_home_app(app(3)).is_err());
    assert_eq!(s.home_reorder_snapshot(1280.0, 800.0), saved);
    assert!(!s.is_on_home(&app(3)));
}
#[test]
fn full_width_grid_keeps_shortcuts_reachable_after_output_resize() {
    let mut s = Shell::new(vec![], ThemeSpec::default());
    s.set_reduce_motion(true);
    for i in 0..95 {
        s.toggle_home_app(app(i)).unwrap();
    }
    for (w, h) in [
        (320.0, 640.0),
        (640.0, 480.0),
        (1280.0, 800.0),
        (1920.0, 1080.0),
    ] {
        let layout = home_layout(w, h);
        assert!(layout.grid.w >= w - 2.0 * config::design().home.margin_x - 0.01);
        let cells = s.home.cells(layout);
        let unique: std::collections::HashSet<_> = cells.values().collect();
        assert_eq!(unique.len(), cells.len());
        for i in 0..s.home.apps.len() {
            let cell = cells[&s.home.apps[i].identity()];
            s.turn_home_page(w, h, cell.page as i32 - s.home.page as i32);
            let rect = s.home_tile_rect(w, h, i);
            assert!(rect.x >= layout.grid.x - 0.01 && rect.right() <= layout.grid.right() + 0.01);
            assert!(rect.y >= layout.grid.y - 0.01 && rect.bottom() <= layout.grid.bottom() + 0.01);
            assert_eq!(s.home_hit(w, h, center(rect)), Some(HomeHit::App(i)));
        }
    }
}
#[test]
fn dragging_can_create_a_page_and_empty_pages_clamp_after_cancel() {
    let mut s = Shell::new(vec![], ThemeSpec::default());
    s.set_reduce_motion(true);
    s.toggle_home_app(app(0)).unwrap();
    s.dragged_identity = Some(app(0).identity());
    s.turn_home_page(1280.0, 800.0, 1);
    assert_eq!(s.home.page, 1);
    s.clamp_home_page(1280.0, 800.0);
    assert_eq!(s.home.page, 1);
    let layout = home_layout(1280.0, 800.0);
    s.place_home_app(app(0), layout.per_page + layout.columns + 3)
        .unwrap();
    s.dragged_identity = None;
    s.clamp_home_page(1280.0, 800.0);
    assert_eq!(s.home.page, 1);
    assert_eq!(s.home_pages(1280.0, 800.0), 2);
    s.dragged_identity = Some(app(0).identity());
    s.turn_home_page(1280.0, 800.0, 1);
    assert_eq!(s.home.page, 2);
    s.dragged_identity = None;
    s.clamp_home_page(1280.0, 800.0);
    assert_eq!(s.home.page, 1);
}
#[test]
fn former_clock_cells_accept_shortcuts_without_reserving_space() {
    let mut s = Shell::new(vec![], ThemeSpec::default());
    s.toggle_home_app(app(0)).unwrap();
    s.place_home_app(app(0), 0).unwrap();
    let r = s.home_tile_rect(1280.0, 800.0, 0);
    assert_eq!(s.home_drop_index(1280.0, 800.0, center(r)), Some(0));
    assert_eq!(s.home_hit(1280.0, 800.0, center(r)), Some(HomeHit::App(0)));
}
