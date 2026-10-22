use super::*;
use std::path::{Path, PathBuf};
fn temp(name: &str) -> PathBuf {
    let p = std::env::temp_dir().join(format!("materialand-icons-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&p);
    std::fs::create_dir_all(&p).unwrap();
    p
}
fn png(path: &Path) {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    let mut s = Surface::new(16, 16);
    s.clear(tiny_skia::Color::from_rgba8(200, 100, 50, 128));
    s.pixmap.save_png(path).unwrap();
}
#[test]
fn lookup_respects_theme_inheritance_cycles_hicolor_and_absolute_icons() {
    let root = temp("lookup");
    for (theme, parent) in [("custom", "parent"), ("parent", "custom"), ("hicolor", "")] {
        std::fs::create_dir_all(root.join(theme)).unwrap();
        std::fs::write(root.join(theme).join("index.theme"),format!("[Icon Theme]\nDirectories=16x16/apps,64x64/apps\nInherits={parent}\n[16x16/apps]\nSize=16\nType=Fixed\n[64x64/apps]\nSize=64\nType=Fixed\n")).unwrap();
    }
    let inherited = root.join("parent/64x64/apps/browser.png");
    png(&inherited);
    let fallback = root.join("hicolor/16x16/apps/terminal.png");
    png(&fallback);
    let unthemed = root.join("pixmaps/editor.png");
    png(&unthemed);
    let mut r = lookup::Resolver::test(root.clone(), "custom");
    assert_eq!(r.find("browser", 56), Some(inherited));
    assert_eq!(r.find("terminal", 56), Some(fallback));
    assert_eq!(r.find("editor", 56), Some(unthemed.clone()));
    assert_eq!(r.find(unthemed.to_str().unwrap(), 56), Some(unthemed));
    assert!(r.find("not-installed", 56).is_none());
    let small = root.join("custom/16x16/apps/browser.png");
    png(&small);
    assert_eq!(
        r.find("browser", 56),
        Some(small),
        "an inherited icon cannot override the selected theme"
    );
    std::fs::remove_dir_all(root).unwrap();
}
#[test]
fn png_and_svg_preserve_alpha_and_aspect_without_requiring_a_display() {
    let root = temp("pixels");
    let path = root.join("icon.png");
    png(&path);
    let raster = pixels::load(&path, 32).unwrap();
    let px = raster.pixmap.pixel(8, 8).unwrap();
    assert_eq!(raster.width(), 32);
    assert_eq!(px.alpha(), 128);
    assert!((i32::from(px.red()) - 100).abs() <= 1);
    let path = root.join("icon.svg");
    std::fs::write(&path,r##"<svg xmlns="http://www.w3.org/2000/svg" width="80" height="40"><rect width="80" height="40" fill="#ff0000" fill-opacity="0.5"/></svg>"##).unwrap();
    let svg = pixels::load(&path, 32).expect("librsvg must decode a valid SVG");
    assert_eq!((svg.width(), svg.pixmap.height()), (32, 16));
    let px = svg.pixmap.pixel(8, 8).unwrap();
    assert!((i32::from(px.alpha()) - 128).abs() <= 1);
    assert_eq!(px.red(), px.alpha());
    std::fs::write(&path, "invalid image").unwrap();
    assert!(pixels::load(&path, 32).is_none());
    std::fs::remove_dir_all(root).unwrap();
}
