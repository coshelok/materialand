use super::*;

#[test]
fn backlight_levels_keep_panel_on_and_respect_driver_range() {
    assert_eq!(level(0.0, 96000), 1920);
    assert_eq!(level(0.5, 96000), 48000);
    assert_eq!(level(4.0, 96000), 96000);
    assert_eq!(level(0.02, 4), 1);
}

use crate::{desktop::State, graphics::text::Fonts, theme::ThemeSpec};
use smithay::reexports::wayland_server::Display;
use std::sync::atomic::AtomicU32;
static FIXTURE: AtomicU32 = AtomicU32::new(0);
struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "materialand-backlight-{}-{}",
            std::process::id(),
            FIXTURE.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir(&path).unwrap();
        std::fs::write(path.join("brightness"), "50").unwrap();
        Self(path)
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.0).unwrap();
    }
}
fn state() -> (Display<State>, State) {
    let mut display = Display::new().unwrap();
    let state = State::new(
        &mut display,
        Fonts::load().unwrap(),
        vec![],
        vec![],
        ThemeSpec::DEFAULT,
    );
    (display, state)
}
fn complete(backlight: &mut Backlight, state: &mut State) {
    let start = Instant::now();
    loop {
        backlight.sync(state);
        if backlight.requested.is_none() {
            break;
        }
        assert!(start.elapsed() < Duration::from_secs(2));
        std::thread::sleep(Duration::from_millis(2));
    }
}
#[test]
fn panel_changes_are_real_and_external_changes_are_not_overwritten() {
    let fixture = Fixture::new();
    let mut backlight = Backlight::at(fixture.0.clone(), 100, |path, _, value| {
        std::fs::write(path.join("brightness"), value.to_string()).map_err(|e| e.to_string())
    })
    .unwrap();
    let (_display, mut state) = state();
    state.brightness = backlight.current().unwrap();
    complete(&mut backlight, &mut state);
    assert_eq!(read(&fixture.0, "brightness"), Some(50));
    state.brightness = 0.75;
    complete(&mut backlight, &mut state);
    assert_eq!(read(&fixture.0, "brightness"), Some(75));
    std::fs::write(fixture.0.join("brightness"), "30").unwrap();
    backlight.last_poll = Instant::now() - Duration::from_secs(2);
    complete(&mut backlight, &mut state);
    assert_eq!(state.brightness, 0.30);
    assert_eq!(read(&fixture.0, "brightness"), Some(30));
}
#[test]
fn permission_failure_restores_actual_slider_without_retry_loop() {
    let fixture = Fixture::new();
    let calls = Arc::new(AtomicU32::new(0));
    let counter = calls.clone();
    let mut backlight = Backlight::at(fixture.0.clone(), 100, move |_, _, _| {
        counter.fetch_add(1, Ordering::Relaxed);
        Err("Permission denied".into())
    })
    .unwrap();
    let (_display, mut state) = state();
    state.brightness = 0.75;
    complete(&mut backlight, &mut state);
    assert!(state.shell.brightness_error);
    assert_eq!(state.brightness, 0.5);
    for _ in 0..10 {
        backlight.sync(&mut state);
    }
    assert_eq!(calls.load(Ordering::Relaxed), 1);
}
#[test]
fn discovery_only_selects_backlight_for_the_driven_connector() {
    use std::os::unix::fs::symlink;
    let fixture = Fixture::new();
    let root = fixture.0.join("class");
    std::fs::create_dir(&root).unwrap();
    let output = fixture.0.join("card1-eDP-1/panel");
    std::fs::create_dir_all(&output).unwrap();
    std::fs::write(output.join("max_brightness"), "96000").unwrap();
    symlink(&output, root.join("intel_backlight")).unwrap();
    assert!(discover(&root, Path::new("/dev/dri/card0"), "eDP-1").is_none());
    assert!(discover(&root, Path::new("/dev/dri/card1"), "HDMI-A-1").is_none());
    assert_eq!(
        discover(&root, Path::new("/dev/dri/card1"), "eDP-1")
            .unwrap()
            .1,
        96000
    );
}
