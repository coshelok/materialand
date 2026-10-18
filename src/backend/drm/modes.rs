//! Modes.
use super::*;
pub(super) fn pick_mode(connector: &connector::Info) -> Option<control::Mode> {
    connector
        .modes()
        .iter()
        .find(|mode| mode.mode_type().contains(ModeTypeFlags::PREFERRED))
        .or_else(|| connector.modes().first())
        .copied()
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
