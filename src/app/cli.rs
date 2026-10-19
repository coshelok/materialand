//! Command-line selection has no display/device side effects.
use crate::Result;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    Direct,
    Nested,
    Probe,
    Help,
    PrintConfig,
    CheckConfig,
    InitConfig,
    CompactConfig,
}
pub fn parse(args: impl IntoIterator<Item = String>, nested_env: bool) -> Result<Mode> {
    let mut selected = None;
    for arg in args {
        let next = match arg.as_str() {
            "--nested" => Mode::Nested,
            "--check" => Mode::Probe,
            "--help" | "-h" => Mode::Help,
            "--print-default-config" => Mode::PrintConfig,
            "--check-config" => Mode::CheckConfig,
            "--init-config" => Mode::InitConfig,
            "--compact-config" => Mode::CompactConfig,
            _ => return Err(format!("unknown option: {arg}").into()),
        };
        if next == Mode::Help {
            return Ok(next);
        }
        if selected.is_some_and(|previous| previous != next) {
            return Err("choose a single mode; see --help".into());
        }
        selected = Some(next);
    }
    Ok(selected.unwrap_or(if nested_env {
        Mode::Nested
    } else {
        Mode::Direct
    }))
}
pub fn help() {
    println!(
        r#"materialand {} — Material 3 Wayland compositor

Usage: materialand [MODE]

  --nested                run in a window on the current desktop
  --check                 inspect DRM connectors without modesetting
  --init-config           create config.toml if missing, migrating saved appearance
  --compact-config        remove redundant defaults, keeping a backup
  --check-config          validate the design file without starting a session
  --print-default-config  print the complete built-in design as TOML
  --help                  show this help

Without options: DRM/KMS session for SDDM or a local TTY.
Design: $XDG_CONFIG_HOME/materialand/config.toml, otherwise ~/.config/materialand/config.toml.
Layout, typography and motion changes apply on the next session start.
MATERIALAND_THEME, MATERIALAND_SEED, MATERIALAND_WALLPAPER, MATERIALAND_VARIANT and
MATERIALAND_FONT_DIR supply defaults for settings omitted from the file.
Session options: MATERIALAND_DRM_DEVICE, MATERIALAND_XKB_LAYOUT, MATERIALAND_XKB_OPTIONS."#,
        env!("CARGO_PKG_VERSION")
    );
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
    #[test]
    fn configuration_modes_override_nested_default_and_reject_conflicting_flags() {
        for (arg, mode) in [
            ("--init-config", Mode::InitConfig),
            ("--compact-config", Mode::CompactConfig),
            ("--check-config", Mode::CheckConfig),
            ("--print-default-config", Mode::PrintConfig),
        ] {
            assert_eq!(parse([arg.into()], true).unwrap(), mode);
            assert!(parse([arg.into(), "--nested".into()], true).is_err());
        }
    }
}
