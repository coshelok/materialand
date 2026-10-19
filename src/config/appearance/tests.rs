use super::*;
use crate::theme::VARIANTS;
use std::{
    os::unix::fs::PermissionsExt,
    time::{SystemTime, UNIX_EPOCH},
};

struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        Self(std::env::temp_dir().join(format!(
            "materialand-appearance-{}-{nonce}",
            std::process::id()
        )))
    }
    fn store(&self) -> AppearanceStore {
        AppearanceStore::at(Some(self.0.join("appearance")))
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

const DEFAULTS: Appearance = Appearance {
    dark: false,
    variant: ColorVariant::TonalSpot,
};

#[test]
fn every_mode_and_palette_survives_restart_and_replaces_previous_choices() {
    let fixture = Fixture::new();
    assert_eq!(fixture.store().load(DEFAULTS).unwrap(), DEFAULTS);
    for dark in [true, false] {
        for variant in VARIANTS {
            let chosen = Appearance { dark, variant };
            fixture.store().save(chosen).unwrap();
            let different_startup_defaults = Appearance {
                dark: !dark,
                variant: if variant == ColorVariant::Vibrant {
                    ColorVariant::Neutral
                } else {
                    ColorVariant::Vibrant
                },
            };
            assert_eq!(
                fixture.store().load(different_startup_defaults).unwrap(),
                chosen
            );
        }
    }
    let path = fixture.0.join("appearance");
    assert_eq!(
        fs::metadata(path).unwrap().permissions().mode() & 0o777,
        0o600
    );
    assert_eq!(
        fs::read_dir(&fixture.0).unwrap().count(),
        1,
        "no temporary files left behind"
    );
}

#[test]
fn incomplete_or_invalid_preferences_keep_valid_fields_and_startup_defaults() {
    let fixture = Fixture::new();
    fs::create_dir_all(&fixture.0).unwrap();
    let path = fixture.0.join("appearance");
    fs::write(
        &path,
        "# theme=dark\ninvalid line\nvariant = fruit_salad\nunknown=value\ntheme=invalid\n",
    )
    .unwrap();
    assert_eq!(
        fixture.store().load(DEFAULTS).unwrap(),
        Appearance {
            dark: false,
            variant: ColorVariant::FruitSalad,
        }
    );
    fs::write(path, "theme=dark\nvariant=unknown\n").unwrap();
    assert_eq!(
        fixture.store().load(DEFAULTS).unwrap(),
        Appearance {
            dark: true,
            ..DEFAULTS
        }
    );
}

#[test]
fn failed_replacement_preserves_destination_and_cleans_temporary_file() {
    let fixture = Fixture::new();
    let destination = fixture.0.join("appearance");
    fs::create_dir_all(&destination).unwrap();
    fs::write(destination.join("keep"), "existing data").unwrap();
    assert!(fixture.store().save(DEFAULTS).is_err());
    assert_eq!(
        fs::read_to_string(destination.join("keep")).unwrap(),
        "existing data"
    );
    assert_eq!(fs::read_dir(&fixture.0).unwrap().count(), 1);
    assert!(fixture.store().load(DEFAULTS).is_err());
}
