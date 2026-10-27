use super::*;
use crate::{
    config::{Appearance, AppearanceStore},
    theme::ColorVariant,
};
use std::{
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};
use toml_edit::DocumentMut;

struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root =
            std::env::temp_dir().join(format!("materialand-design-{}-{nonce}", std::process::id()));
        fs::create_dir_all(root.join("materialand")).unwrap();
        Self(root)
    }
    fn path(&self) -> PathBuf {
        self.0.join("materialand/config.toml")
    }
    fn write(&self, text: &str) {
        fs::write(self.path(), text).unwrap();
    }
    fn read(&self) -> Design {
        parse_document(&load_document(&self.path(), defaults()).unwrap()).unwrap()
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
fn defaults() -> DocumentMut {
    DEFAULT_CONFIG.parse().unwrap()
}

#[test]
fn built_in_document_is_valid_and_partial_tables_inherit_defaults() {
    parse_document(&defaults()).unwrap();
    let fixture = Fixture::new();
    fixture
        .write("dock = { height = 100 }\n[typography.body_medium]\nsize = 17\nline_height = 24\n");
    let d = fixture.read();
    assert_eq!(d.dock.height, 100.0);
    assert_eq!(d.dock.icon_size, built_in_design().dock.icon_size);
    assert_eq!(d.typography.body_medium.size, 17.0);
    assert_eq!(
        d.typography.body_medium.kind,
        built_in_design().typography.body_medium.kind
    );
}

#[test]
fn compact_configuration_preserves_every_custom_value_and_numeric_equivalence() {
    let fixture = Fixture::new();
    let mut doc = defaults();
    doc["appearance"]["theme"] = toml_edit::value("dark");
    doc["appearance"]["variant"] = toml_edit::value("Neutral");
    doc["calendar"]["height"] = toml_edit::value(380);
    doc["calendar"]["grid_padding"] = toml_edit::value(10);
    doc["calendar"]["min_row_height"] = toml_edit::value(30);
    doc["dock"]["height"] = toml_edit::value(80);
    doc["input"]["touchpad"]["invert_scroll"] = toml_edit::value(true);
    doc["colors"]["dark"]["primary"] = toml_edit::value("#112233");
    let before = parse_document(&doc).unwrap();
    let compact = compact::compact_document(&doc).to_string();
    assert!(compact.lines().count() < 40, "{compact}");
    assert!(!compact.contains("[dock]") && !compact.contains("[motion]"));
    assert!(compact.contains("invert_scroll = true"));
    fixture.write(&compact);
    assert_eq!(fixture.read(), before);
}

#[test]
fn initialization_and_ui_saves_create_full_commented_files_and_migrate_preferences() {
    let fixture = Fixture::new();
    fs::write(
        fixture.path().with_file_name("appearance"),
        "theme=dark\nvariant=Vibrant\n",
    )
    .unwrap();
    load_or_create(&fixture.path()).unwrap();
    let text = fs::read_to_string(fixture.path()).unwrap();
    assert!(text.contains("[motion]") && text.contains("[typography.body_medium]"));
    assert!(text.contains("# Spring stiffness") && text.contains("# Font size"));
    assert!(text.contains("[input.touchpad]") && text.contains("invert_scroll = false"));
    assert_eq!(fixture.read().appearance.variant, "Vibrant");
    AppearanceStore::at(Some(fixture.path()))
        .save(Appearance {
            dark: false,
            variant: ColorVariant::Neutral,
            reduce_motion: true,
        })
        .unwrap();
    let saved = fs::read_to_string(fixture.path()).unwrap();
    assert!(saved.contains("# Initial mode:") && saved.contains("# Palette:"));
    assert!(saved.contains("# true snaps animations") && saved.contains("[motion]"));
    assert_eq!(fixture.read().appearance.theme, "light");
    assert!(fixture.read().appearance.reduce_motion);
    load_or_create(&fixture.path()).unwrap();
    assert_eq!(fs::read_to_string(fixture.path()).unwrap(), saved);
    let second = Fixture::new();
    AppearanceStore::at(Some(second.path()))
        .save(Appearance {
            dark: true,
            variant: ColorVariant::Neutral,
            reduce_motion: false,
        })
        .unwrap();
    let text = fs::read_to_string(second.path()).unwrap();
    assert!(text.contains("[motion]") && text.contains("[typography.body_medium]"));
    assert!(text.contains("# Font size") && text.contains("# Palette:"));
    assert_eq!(second.read().appearance.theme, "dark");
}

#[test]
fn malformed_unknown_and_unsafe_values_are_rejected_without_panics() {
    let fixture = Fixture::new();
    for text in [
        "[dock",
        "unknown = 4",
        "[dock]\nheigth = 64",
        "dock = false",
        "[dock]\nheight = -1",
        "[dock]\nheight = nan",
        "[home]\ncell_width = 0",
        "[home]\ncell_height = 0",
        "[launcher]\ntile_size = 0",
        "[window]\ncascade_count = 0",
        "[window]\ncascade_count = 100000",
        "[motion]\nspatial_stiffness = inf",
        "[motion]\neffects_damping = 0.4",
        "[shadow]\ninner_alpha = 0.9\nouter_alpha = 0.9",
        "[shadow]\nlayers = 0",
        "[cursor]\nsize = 1000",
        "[cursor]\nfill_dark = 'oops'",
        "[calendar]\nmin_row_height = 35\nmax_row_height = 10",
        "[easing]\nstandard_decelerate = [2,0,0,1]",
        "[wallpaper]\nglow_one = [0,0,-1,1]",
        "[colors.dark]\nunknown_role = '#112233'",
        "[colors.dark]\nprimary = '#xyzxyz'",
        "[typography.body_medium]\nline_height = 2",
        "[navigation]\nmode = 'phone'",
        "[navigation]\nwidth = 100\nheight = 44",
        "[split]\nhandle_width = 30\ndivider_width = 12",
        "[drag]\nthreshold = 0",
        "[drag]\npage_delay = 0.01",
        "[shade]\nwidth = 0",
        "[statusbar]\nheight = 12\nicon_size = 18",
        "[input.touchpad]\ntap_to_click = 'yes'",
        "[overview]\ncard_width = nan",
        "[overview]\ntransition_stiffness = 0",
        "[overview]\ntransition_damping = 0.1",
        "[overview]\ndismiss_damping = 0.8",
        "[overview]\nreflow_stiffness = 0",
        "[overview]\nclose_response_timeout = 0",
        "[split]\ndivider_hit_width = 2",
        "[appearance]\ntheme = 'auto'",
        "[appearance]\nvariant = 'invalid'",
        "schema_version = 999",
    ] {
        fixture.write(text);
        let result =
            load_document(&fixture.path(), defaults()).and_then(|doc| parse_document(&doc));
        assert!(result.is_err(), "accepted invalid configuration: {text}");
        assert_eq!(fs::read_to_string(fixture.path()).unwrap(), text);
    }
}

#[test]
fn saving_ui_choices_preserves_partial_layout_color_settings_and_comments() {
    let fixture = Fixture::new();
    fixture.write("# My desktop\n[dock]\nheight = 96 # roomy dock\n[colors.dark]\nprimary = '#123456'\n[appearance]\ntheme = 'light' # keep this comment\n");
    let chosen = Appearance {
        dark: true,
        variant: ColorVariant::Vibrant,
        reduce_motion: true,
    };
    AppearanceStore::at(Some(fixture.path()))
        .save(chosen)
        .unwrap();
    let text = fs::read_to_string(fixture.path()).unwrap();
    assert!(
        text.contains("# My desktop")
            && text.contains("96 # roomy dock")
            && text.contains("# keep this comment")
    );
    let d = fixture.read();
    assert_eq!(d.dock.height, 96.0);
    assert_eq!(d.colors.dark["primary"], "#123456");
    assert_eq!(d.appearance.theme, "dark");
    assert_eq!(d.appearance.variant, "Vibrant");
    assert!(d.appearance.reduce_motion);
}

#[test]
fn missing_document_migrates_legacy_choices_and_existing_document_takes_priority() {
    let fixture = Fixture::new();
    let legacy = fixture.path().with_file_name("appearance");
    let text = "# Old settings\ntheme=dark\nvariant=Vibrant\n";
    fs::write(&legacy, text).unwrap();
    let d = fixture.read();
    assert_eq!(d.appearance.theme, "dark");
    assert_eq!(d.appearance.variant, "Vibrant");
    assert_eq!(fs::read_to_string(&legacy).unwrap(), text);
    fixture.write("[appearance]\ntheme = 'light'\nvariant = 'Neutral'\n");
    let d = fixture.read();
    assert_eq!(d.appearance.theme, "light");
    assert_eq!(d.appearance.variant, "Neutral");
    assert_eq!(fs::read_to_string(legacy).unwrap(), text);
}

#[test]
fn configured_session_shares_geometry_pixels_and_preferences() {
    // Immutable process configuration is tested in a separate process so other
    // parallel tests keep deterministic defaults and never read the user's HOME.
    if std::env::var_os("MATERIALAND_CONFIG_TEST_CHILD").is_none() {
        let fixture = Fixture::new();
        fixture.write(CUSTOM);
        let output = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "config::design::tests::configured_session_shares_geometry_pixels_and_preferences",
                "--nocapture",
            ])
            .env("MATERIALAND_CONFIG_TEST_CHILD", "1")
            .env("XDG_CONFIG_HOME", &fixture.0)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        return;
    }
    configured_session();
}

const CUSTOM: &str = r##"
# Custom dimensions must affect both drawing and input.
[navigation]
mode = "desktop"
[appearance]
theme = "dark"
variant = "Vibrant"
reduce_motion = true
seed = "#ab85df"
[topbar]
background_alpha = 1
height = 64
margin = 20
[dock]
height = 96
icon_size = 64
gap = 16
[window]
titlebar_height = 56
[launcher]
tile_size = 128
[home]
cell_height = 140
tile_height = 120
[apps]
medallion_radius = 40
icon_center_y = 42
label_y = 92
[typography.body_medium]
size = 18
line_height = 24
[colors.dark]
primary = "#123456"
surface_container_low = "#102030"
"##;

fn configured_session() {
    use crate::{
        desktop::State,
        graphics::text::Fonts,
        shell::{self, TopAction},
        theme::ThemeSpec,
    };
    use smithay::{reexports::wayland_server::Display, utils::Rectangle};
    initialize_design();
    let spec = ThemeSpec::from_config().0;
    let mut display = Display::new().unwrap();
    let mut state = State::new(&mut display, Fonts::load().unwrap(), vec![], vec![], spec);
    state.load_appearance();
    assert!(state.theme.is_dark && state.shell.reduce_motion);
    assert_eq!(
        state.theme.primary,
        crate::theme::parse_role_color("#123456").unwrap()
    );
    assert_eq!(state.available_area().loc.y, 104);
    let bar = shell::topbar_rect(1280.0);
    assert_eq!((bar.x, bar.y, bar.w, bar.h), (20.0, 20.0, 1240.0, 64.0));
    for action in [
        TopAction::Home,
        TopAction::Clock,
        TopAction::Theme,
        TopAction::Brightness,
        TopAction::QuickSettings,
        TopAction::Power,
    ] {
        let r = shell::topbar_button_rect(1280.0, action);
        assert_eq!(
            shell::topbar_hit(
                1280.0,
                (f64::from(r.x + r.w / 2.0), f64::from(r.y + r.h / 2.0)).into()
            ),
            Some(action)
        );
    }
    let theme = state.theme;
    let (mut buffer, _) = state
        .shell
        .render_topbar(&theme, &mut state.fonts, 1280.0, "", None);
    buffer
        .render()
        .draw::<_, ()>(|pixels| {
            assert_eq!(pixels.len(), 1240 * 64 * 4);
            let offset = (32 * 1240 + 620) * 4;
            assert_eq!(&pixels[offset..offset + 4], &[16, 32, 48, 255]);
            Ok(vec![])
        })
        .unwrap();
    for i in 0..4 {
        let r = shell::dock_item_rect(1280.0, 800.0, 4, i);
        assert_eq!(r.w, 64.0);
        assert_eq!(
            shell::dock_hit(
                1280.0,
                800.0,
                4,
                (f64::from(r.x + r.w / 2.0), f64::from(r.y + r.h / 2.0)).into()
            ),
            Some(i)
        );
    }
    let tile = shell::RectF {
        x: 100.0,
        y: 200.0,
        w: 128.0,
        h: 128.0,
    };
    let icon = shell::app_icon_rect(tile);
    assert_eq!((icon.x, icon.y, icon.w, icon.h), (124.0, 202.0, 80.0, 80.0));
    let frame = shell::decor_layout(Rectangle::new((100, 100).into(), (900, 540).into()));
    assert_eq!(frame.titlebar.h, 56.0);
    assert_eq!(
        shell::decor_hit(
            &frame,
            (f64::from(frame.close.0), f64::from(frame.close.1)).into()
        ),
        shell::DecorHit::Close
    );
    state.toggle_theme((500.0, 16.0).into());
    state.set_variant(ColorVariant::Neutral);
    state.toggle_reduce_motion();
    state.flush_appearance();
    let path = super::appearance_path().unwrap();
    let reloaded = parse_document(&load_document(&path, defaults()).unwrap()).unwrap();
    assert_eq!(reloaded.appearance.theme, "light");
    assert_eq!(reloaded.appearance.variant, "Neutral");
    assert!(!reloaded.appearance.reduce_motion);
    assert_eq!(reloaded.topbar.height, 64.0);
    assert_eq!(reloaded.typography.body_medium.size, 18.0);
    assert!(fs::read_to_string(path)
        .unwrap()
        .contains("# Custom dimensions"));
}
