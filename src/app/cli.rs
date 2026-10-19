//! Command-line selection has no display/device side effects.
use crate::Result;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    Direct,
    Nested,
    Probe,
    Help,
}
pub fn parse(args: impl IntoIterator<Item = String>, nested_env: bool) -> Result<Mode> {
    let mut result = if nested_env {
        Mode::Nested
    } else {
        Mode::Direct
    };
    for arg in args {
        let next = match arg.as_str() {
            "--nested" => Mode::Nested,
            "--check" => Mode::Probe,
            "--help" | "-h" => Mode::Help,
            _ => return Err(format!("unknown option: {arg}").into()),
        };
        if next == Mode::Help {
            return Ok(next);
        }
        if result != Mode::Direct && result != next && !nested_env {
            return Err("choose either --nested or --check".into());
        }
        result = next;
    }
    Ok(result)
}
pub fn help() {
    println!("materialand {} — Material 3 Wayland compositor\n\nUsage: materialand [--nested | --check | --help]\n\n  --nested  run in a window on the current desktop\n  --check   inspect DRM connectors without modesetting\n  --help    show this help\n\nWithout options: DRM/KMS session for SDDM or a local TTY.\nConfiguration: MATERIALAND_THEME (light/dark), MATERIALAND_SEED, MATERIALAND_WALLPAPER, MATERIALAND_VARIANT,\nMATERIALAND_FONT_DIR, MATERIALAND_DRM_DEVICE, MATERIALAND_XKB_LAYOUT,\nMATERIALAND_XKB_OPTIONS.",env!("CARGO_PKG_VERSION"));
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_unknown_and_conflicting_modes() {
        assert!(parse(["--typo".into()], false).is_err());
        assert!(parse(["--nested".into(), "--check".into()], false).is_err());
    }
    #[test]
    fn defaults_to_direct_and_accepts_nested_env() {
        assert_eq!(parse([], false).unwrap(), Mode::Direct);
        assert_eq!(parse([], true).unwrap(), Mode::Nested);
    }
}
