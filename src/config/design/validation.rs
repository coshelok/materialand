//! Reject invalid designs before they can reach allocation, geometry or springs.
use super::*;
use crate::theme::{parse_role_color, parse_seed, valid_role, ColorVariant};
use toml_edit::{DocumentMut, Item};

pub(super) fn validate(d: &Design, doc: &DocumentMut) -> io::Result<()> {
    fn require(ok: bool, message: &str) -> io::Result<()> {
        if ok {
            Ok(())
        } else {
            Err(invalid(message))
        }
    }
    require(d.schema_version == 1, "unsupported design schema_version")?;
    require(
        matches!(d.appearance.theme.as_str(), "light" | "dark"),
        "appearance.theme must be light or dark",
    )?;
    require(
        ColorVariant::from_name(&d.appearance.variant).is_some(),
        "unknown appearance.variant",
    )?;
    require(
        d.appearance.seed.is_empty() || parse_seed(&d.appearance.seed).is_some(),
        "invalid appearance.seed",
    )?;
    numbers(doc.as_item(), "")?;
    require(
        d.dock.icon_size <= d.dock.height,
        "dock.icon_size must fit dock.height",
    )?;
    require(
        d.shadow.padding >= d.shadow.spread,
        "shadow.padding must cover shadow.spread",
    )?;
    require(
        d.launcher.min_width <= d.launcher.width && d.launcher.min_height <= d.launcher.max_height,
        "launcher minimum size exceeds maximum size",
    )?;
    require(
        d.calendar.min_row_height <= d.calendar.max_row_height,
        "calendar row height limits are reversed",
    )?;
    require(
        d.apps.medallion_radius <= d.apps.icon_center_y
            && 2.0 * d.apps.medallion_radius <= d.launcher.tile_size,
        "app medallion must fit its tile",
    )?;
    require(
        d.home.cell_height >= d.home.tile_height,
        "home.cell_height must cover tile_height",
    )?;
    require(
        d.switch.handle_off_radius <= d.switch.height / 2.0
            && d.switch.handle_on_radius <= d.switch.height / 2.0,
        "switch handles must fit the track",
    )?;
    require(
        d.switch.width >= d.switch.height,
        "switch.width must be at least switch.height",
    )?;
    require(
        d.window.cascade_count >= 1 && d.window.cascade_count <= 256,
        "window.cascade_count must be in 1..256",
    )?;
    require(
        d.motion.effects_damping >= 1.0,
        "motion.effects_damping must be at least 1 to keep opacity bounded",
    )?;
    require(
        d.shadow.inner_alpha + d.shadow.outer_alpha <= 1.0,
        "combined shadow opacity exceeds 1",
    )?;
    require(
        (0.5..=1.0).contains(&d.motion.press_scale),
        "motion.press_scale must be in 0.5..1",
    )?;
    require(
        (0.0..=1.0).contains(&d.motion.window_initial_alpha)
            && (0.0..=0.5).contains(&d.motion.window_close_shrink),
        "invalid window animation opacity or shrink",
    )?;
    require(d.cursor.size <= 256.0, "cursor.size must not exceed 256")?;
    for color in [
        &d.cursor.fill_light,
        &d.cursor.fill_dark,
        &d.cursor.outline_light,
        &d.cursor.outline_dark,
    ] {
        require(
            parse_role_color(color).is_some(),
            "cursor colors must be #RRGGBB",
        )?;
    }
    for glow in [d.wallpaper.glow_one, d.wallpaper.glow_two] {
        require(
            glow[2] > 0.0 && (0.0..=1.0).contains(&glow[3]),
            "wallpaper glow needs a positive radius and opacity in 0..1",
        )?;
    }
    for (name, color) in d.colors.light.iter().chain(d.colors.dark.iter()) {
        require(
            valid_role(name),
            &format!("unknown Material color role: {name}"),
        )?;
        require(
            parse_role_color(color).is_some(),
            &format!("colors.{name} must be #RRGGBB"),
        )?;
    }
    for (name, item) in doc["typography"]
        .as_table_like()
        .expect("typed typography")
        .iter()
    {
        let size = item["size"]
            .as_float()
            .or_else(|| item["size"].as_integer().map(|n| n as f64))
            .unwrap_or_default();
        let height = item["line_height"]
            .as_float()
            .or_else(|| item["line_height"].as_integer().map(|n| n as f64))
            .unwrap_or_default();
        require(
            size >= 1.0 && height >= size && height <= 512.0,
            &format!("typography.{name}: line_height must cover a positive size"),
        )?;
    }
    Ok(())
}
fn numbers(item: &Item, path: &str) -> io::Result<()> {
    if let Some(table) = item.as_table_like() {
        for (key, item) in table.iter() {
            numbers(item, &format!("{path}.{key}"))?;
        }
    } else if let Some(value) = item
        .as_float()
        .or_else(|| item.as_integer().map(|n| n as f64))
    {
        let signed = path.ends_with(".offset_y")
            || path.ends_with("_offset_y")
            || path.ends_with(".tracking");
        let (min, max) = if path.starts_with(".motion.") {
            if path.ends_with("_damping") {
                (0.3, 2.0)
            } else if path.ends_with("_stiffness") {
                (1.0, 4000.0)
            } else if path.ends_with("_duration") {
                (0.01, 5.0)
            } else if path.ends_with("_scale") {
                (0.01, 1.0)
            } else if path.ends_with("overshoot_limit") {
                (1.0, 1.15)
            } else {
                (0.0, 256.0)
            }
        } else if path.starts_with(".state.")
            || path.ends_with("_alpha")
            || path.contains("_mix")
            || path.ends_with("_tint")
        {
            (0.0, 1.0)
        } else if path.starts_with(".shadow.layers") {
            (1.0, 32.0)
        } else if signed {
            (-4096.0, 4096.0)
        } else {
            (0.0, 4096.0)
        };
        if !value.is_finite() || !(min..=max).contains(&value) {
            return Err(invalid(format!(
                "{} must be finite and in {min}..={max}",
                path.trim_start_matches('.')
            )));
        }
        let size_field = path.ends_with("width")
            || path.ends_with("height")
            || path.ends_with("size")
            || path.ends_with("gap")
            || path.ends_with("row_h");
        if size_field && value < 1.0 && !path.ends_with("gap") {
            return Err(invalid(format!("{path} must be at least 1")));
        }
    } else if let Some(array) = item.as_array() {
        if path.ends_with("_icons") {
            return Ok(());
        }
        for value in array {
            let number = value
                .as_float()
                .or_else(|| value.as_integer().map(|n| n as f64))
                .ok_or_else(|| invalid(format!("{path} needs numbers")))?;
            if !number.is_finite() || !(-2.0..=2.0).contains(&number) {
                return Err(invalid(format!("{path} has invalid coordinates")));
            }
        }
        if path.starts_with(".easing.") {
            let values: Vec<_> = array
                .iter()
                .filter_map(|v| v.as_float().or_else(|| v.as_integer().map(|n| n as f64)))
                .collect();
            if values.len() != 4
                || !(0.0..=1.0).contains(&values[0])
                || !(0.0..=1.0).contains(&values[2])
            {
                return Err(invalid("easing x coordinates must be in 0..1"));
            }
        }
    }
    Ok(())
}
