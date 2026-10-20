//! Linux power-supply state; unknown readings stay unknown.
use super::Battery;
use std::{fs, path::Path};
pub(super) fn read(root: &Path) -> Option<Battery> {
    let mut levels = Vec::new();
    let mut charging = false;
    let mut full = true;
    let mut ac = false;
    for entry in fs::read_dir(root).ok()?.flatten() {
        let p = entry.path();
        let text = |key: &str| {
            fs::read_to_string(p.join(key))
                .unwrap_or_default()
                .trim()
                .to_owned()
        };
        let number = |key: &str| text(key).parse::<f64>().ok();
        match text("type").as_str() {
            "Battery" if text("present") != "0" => {
                let percent = number("capacity").or_else(|| {
                    for (now, total) in
                        [("energy_now", "energy_full"), ("charge_now", "charge_full")]
                    {
                        if let (Some(now), Some(total)) = (number(now), number(total)) {
                            if total > 0.0 {
                                return Some(now / total * 100.0);
                            }
                        }
                    }
                    None
                });
                if let Some(level) = percent.filter(|v| v.is_finite()) {
                    levels.push(level.clamp(0.0, 100.0));
                }
                charging |= text("status") == "Charging";
                full &= text("status") == "Full";
            }
            "Mains" | "USB" | "USB_C" => ac |= text("online") == "1",
            _ => {}
        }
    }
    if levels.is_empty() {
        None
    } else {
        Some(Battery {
            percent: (levels.iter().sum::<f64>() / levels.len() as f64).round() as u8,
            charging,
            plugged: ac || charging || full,
        })
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn battery_tracks_charge_and_absence_without_fake_values() {
        let root = std::env::temp_dir().join(format!("materialand-battery-{}", std::process::id()));
        let b = root.join("BAT0");
        fs::create_dir_all(&b).unwrap();
        fs::write(b.join("type"), "Battery").unwrap();
        fs::write(b.join("capacity"), "57").unwrap();
        fs::write(b.join("status"), "Charging").unwrap();
        assert_eq!(
            read(&root),
            Some(Battery {
                percent: 57,
                charging: true,
                plugged: true
            })
        );
        fs::write(b.join("present"), "0").unwrap();
        assert_eq!(read(&root), None);
        fs::remove_dir_all(root).unwrap();
    }
}
