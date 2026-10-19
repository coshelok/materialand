//! Modes.
use super::*;
pub(super) fn pick_mode(connector: &connector::Info) -> Option<control::Mode> {
    select_mode(connector.modes())
}

#[derive(Clone, Copy)]
struct ModeCandidate {
    size: (u16, u16),
    refresh: i32,
    preferred: bool,
    progressive: bool,
}
fn select_mode(modes: &[control::Mode]) -> Option<control::Mode> {
    let candidates: Vec<_> = modes
        .iter()
        .map(|mode| ModeCandidate {
            size: mode.size(),
            refresh: to_output_mode(*mode).refresh,
            preferred: mode.mode_type().contains(ModeTypeFlags::PREFERRED),
            progressive: !mode
                .flags()
                .intersects(control::ModeFlags::INTERLACE | control::ModeFlags::DBLSCAN),
        })
        .collect();
    select_index(&candidates).map(|index| modes[index])
}
fn select_index(modes: &[ModeCandidate]) -> Option<usize> {
    // Keep the preferred resolution; a faster low-resolution timing must not
    // shrink the desktop. With no preferred mode use the largest resolution.
    let valid = |mode: &&ModeCandidate| mode.refresh > 0 && mode.size.0 > 0 && mode.size.1 > 0;
    let native = modes
        .iter()
        .filter(valid)
        .find(|mode| mode.preferred)
        .or_else(|| {
            modes
                .iter()
                .filter(valid)
                .max_by_key(|mode| u32::from(mode.size.0) * u32::from(mode.size.1))
        })?
        .size;
    modes
        .iter()
        .enumerate()
        .filter(|(_, mode)| mode.size == native && mode.refresh > 0)
        .max_by_key(|(_, mode)| (mode.progressive, mode.refresh, mode.preferred))
        .map(|(index, _)| index)
}

pub(super) fn pick_connector(
    device: &DrmDevice,
) -> Option<(connector::Handle, crtc::Handle, control::Mode)> {
    let resources = device.resource_handles().ok()?;
    for connector_handle in resources.connectors() {
        let Ok(info) = device.get_connector(*connector_handle, false) else {
            continue;
        };
        if info.state() != connector::State::Connected {
            continue;
        }
        let Some(mode) = pick_mode(&info) else {
            continue;
        };
        let crtc = info
            .encoders()
            .iter()
            .filter_map(|encoder| device.get_encoder(*encoder).ok())
            .flat_map(|encoder| resources.filter_crtcs(encoder.possible_crtcs()))
            .next();
        if let Some(crtc) = crtc {
            return Some((*connector_handle, crtc, mode));
        }
    }
    None
}

pub(super) fn to_output_mode(mode: control::Mode) -> OutputMode {
    let (w, h) = mode.size();
    OutputMode {
        size: (i32::from(w), i32::from(h)).into(),
        refresh: refresh_millihz(
            mode.clock(),
            mode.hsync().2,
            mode.vsync().2,
            mode.flags(),
            mode.vscan(),
        ),
    }
}
fn refresh_millihz(
    clock: u32,
    htotal: u16,
    vtotal: u16,
    flags: control::ModeFlags,
    vscan: u16,
) -> i32 {
    let divisor = u64::from(htotal) * u64::from(vtotal);
    if divisor == 0 {
        return 0;
    }
    let mut numerator = u64::from(clock) * 1_000_000;
    let mut denominator = divisor;
    if flags.contains(control::ModeFlags::INTERLACE) {
        numerator *= 2;
    }
    if flags.contains(control::ModeFlags::DBLSCAN) {
        denominator *= 2;
    }
    if vscan > 1 {
        denominator *= u64::from(vscan);
    }
    ((numerator + denominator / 2) / denominator).min(i32::MAX as u64) as i32
}
pub(super) fn is_card(path: &Path) -> bool {
    path.parent()
        .map(|dir| dir == Path::new("/dev/dri"))
        .unwrap_or(false)
        && path
            .file_name()
            .and_then(|name| name.to_str())
            .map(|name| name.starts_with("card"))
            .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn native_resolution_uses_fastest_progressive_refresh_without_dropping_resolution() {
        let mode = |size, refresh, preferred, progressive| ModeCandidate {
            size,
            refresh,
            preferred,
            progressive,
        };
        let modes = [
            mode((2560, 1440), 60_000, true, true),
            mode((1920, 1080), 360_000, false, true),
            mode((2560, 1440), 144_000, false, true),
            mode((2560, 1440), 165_000, false, true),
            mode((2560, 1440), 240_000, false, false),
            mode((2560, 1440), 0, true, true),
        ];
        assert_eq!(select_index(&modes), Some(3));
        let fractional = [
            mode((1920, 1080), 59_940, true, true),
            mode((1920, 1080), 60_000, false, true),
        ];
        assert_eq!(select_index(&fractional), Some(1));
        let no_preferred = [
            mode((1920, 1080), 240_000, false, true),
            mode((3840, 2160), 120_000, false, true),
        ];
        assert_eq!(select_index(&no_preferred), Some(1));
        assert_eq!(select_index(&[]), None);
        assert_eq!(select_index(&[mode((1920, 1080), 0, true, true)]), None);
    }
    #[test]
    fn refresh_preserves_fractional_rates_in_protocol_units() {
        assert_eq!(
            refresh_millihz(148352, 2200, 1125, control::ModeFlags::empty(), 1),
            59940
        );
        assert_eq!(
            refresh_millihz(148500, 2200, 1125, control::ModeFlags::empty(), 1),
            60000
        );
    }
    #[test]
    fn scan_flags_and_invalid_timings_are_handled() {
        assert_eq!(
            refresh_millihz(148500, 2200, 1125, control::ModeFlags::INTERLACE, 1),
            120000
        );
        assert_eq!(
            refresh_millihz(148500, 2200, 1125, control::ModeFlags::DBLSCAN, 1),
            30000
        );
        assert_eq!(
            refresh_millihz(148500, 0, 1125, control::ModeFlags::empty(), 1),
            0
        );
    }
}
