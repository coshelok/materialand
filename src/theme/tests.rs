use super::*;

#[test]
fn schemes_are_opaque_and_tagged() {
    for t in [ThemeSpec::default().dark(), ThemeSpec::default().light()] {
        assert_eq!(t.background.alpha(), 1.0);
        assert_eq!(t.on_surface.alpha(), 1.0);
        assert_eq!(t.primary.alpha(), 1.0);
        assert_eq!(t.surface_container_highest.alpha(), 1.0);
        assert_eq!(t.primary_fixed.alpha(), 1.0);
        assert_eq!(t.tertiary_fixed_dim.alpha(), 1.0);
    }
    assert!(ThemeSpec::default().dark().is_dark);
    assert!(!ThemeSpec::default().light().is_dark);
}

#[test]
fn with_alpha_keeps_rgb_changes_alpha() {
    let t = ThemeSpec::default().dark();
    let c = Theme::with_alpha(t.primary, 0.5);
    assert_eq!(c.red(), t.primary.red());
    assert_eq!(c.green(), t.primary.green());
    assert_eq!(c.blue(), t.primary.blue());
    assert!((c.alpha() - 0.5).abs() < 1e-4);
}

#[test]
fn elevation_levels_clamp() {
    let t = ThemeSpec::default().dark();
    assert_eq!(t.surface_for_elevation(0), t.surface);
    assert_eq!(t.surface_for_elevation(4), t.surface_container_highest);
    assert_eq!(t.surface_for_elevation(9), t.surface_for_elevation(5));
}

#[test]
fn seed_is_deterministic_and_changes_palette() {
    let a = ThemeSpec::default().dark();
    let b = ThemeSpec::default().dark();
    assert_eq!(a, b, "same seed must reproduce the same scheme");
    let other = Theme::from_spec(
        ThemeSpec {
            seed: 0xff00_ff00,
            variant: ColorVariant::TonalSpot,
        },
        true,
    );
    assert_ne!(a.primary, other.primary, "different seeds differ");
}

#[test]
fn mix_endpoints_are_exact() {
    let t = ThemeSpec::default().dark();
    assert_eq!(Theme::mix(t.primary, t.secondary, 0.0), t.primary);
    assert_eq!(Theme::mix(t.primary, t.secondary, 1.0), t.secondary);
}

#[test]
fn cache_key_changes_when_palette_changes() {
    let dark = ThemeSpec::default().dark();
    let light = ThemeSpec::default().light();
    assert_ne!(dark.cache_key(), light.cache_key());
    assert_eq!(dark.cache_key(), dark.cache_key());
}

#[test]
fn all_nine_variants_build_distinct_schemes() {
    let mut primaries = std::collections::HashSet::new();
    for variant in VARIANTS {
        let t = Theme::from_spec(
            ThemeSpec {
                seed: ThemeSpec::default().seed,
                variant,
            },
            true,
        );
        primaries.insert((
            (t.primary.red() * 255.0) as u8,
            (t.primary.green() * 255.0) as u8,
            (t.primary.blue() * 255.0) as u8,
        ));
        assert_eq!(t.primary.alpha(), 1.0);
    }
    assert!(
        primaries.len() >= 5,
        "variants must actually change the palette, got {} distinct primaries",
        primaries.len()
    );
}

#[test]
fn variant_names_round_trip() {
    for variant in VARIANTS {
        assert_eq!(ColorVariant::from_name(variant.name()), Some(variant));
    }
    assert_eq!(
        ColorVariant::from_name("fruit_salad"),
        Some(ColorVariant::FruitSalad)
    );
    assert_eq!(ColorVariant::from_name("nonsense"), None);
}

#[test]
fn parse_seed_accepts_common_forms() {
    assert_eq!(parse_seed("#6750A4"), Some(0xff67_50a4));
    assert_eq!(parse_seed("6750a4"), Some(0xff67_50a4));
    assert_eq!(parse_seed("0x6750A4"), Some(0xff67_50a4));
    assert_eq!(parse_seed("FF6750A4"), Some(0xff67_50a4));
    assert_eq!(parse_seed(""), None);
    assert_eq!(parse_seed("zzz"), None);
    assert_eq!(parse_seed("#123456789"), None);
}

#[test]
fn wallpaper_extraction_fails_softly() {
    assert_eq!(
        seed_from_wallpaper(Path::new("/nonexistent/wall.png")),
        None
    );
    // A readable file that is not an image must also be rejected.
    let path = std::env::temp_dir().join("materialand-not-an-image.txt");
    std::fs::write(&path, "definitely not an image").unwrap();
    assert_eq!(seed_from_wallpaper(&path), None);
    let _ = std::fs::remove_file(&path);
}

#[test]
fn fixed_roles_differ_between_light_and_dark() {
    let (light, dark) = (ThemeSpec::default().light(), ThemeSpec::default().dark());
    // Fixed roles are *fixed*: the same tonal value in both schemes.
    assert_eq!(light.primary_fixed, dark.primary_fixed);
    assert_eq!(light.on_primary_fixed, dark.on_primary_fixed);
    assert_eq!(light.secondary_fixed, dark.secondary_fixed);
    assert_eq!(light.tertiary_fixed, dark.tertiary_fixed);
    // ...but each container family differs from its neighbours.
    assert_ne!(light.primary_fixed, light.secondary_fixed);
}
