//! Output LUT adjustment: multiplicative warmth preserves blacks and detail.
use super::*;

type Ramp = [Vec<u16>; 3];
pub(super) struct NightLight {
    fd: DrmDeviceFd,
    crtc: crtc::Handle,
    original: Ramp,
    last: f32,
}

fn ramp(size: usize, warmth: f32) -> Ramp {
    // Deliberately gentle white balance. Never add an orange pedestal to black.
    let gains = [1.0, 1.0 - 0.10 * warmth, 1.0 - 0.26 * warmth];
    gains.map(|gain| {
        (0..size)
            .map(|i| (i as f32 / (size - 1) as f32 * gain * u16::MAX as f32).round() as u16)
            .collect()
    })
}

impl NightLight {
    pub(super) fn new(device: &DrmDevice, crtc: crtc::Handle) -> Option<Self> {
        let size = device.get_crtc(crtc).ok()?.gamma_length() as usize;
        if size < 2 {
            return None;
        }
        let mut original: Ramp = std::array::from_fn(|_| vec![0; size]);
        let [red, green, blue] = &mut original;
        device.get_gamma(crtc, red, green, blue).ok()?;
        let neutral = ramp(size, 0.0);
        // Some drivers report a zero-filled LUT for the implicit bypass state.
        // Restoring zeros would black out the output; use identity for bypass.
        if original
            .iter()
            .all(|channel| channel.iter().all(|v| *v == 0))
        {
            original = neutral.clone();
        }
        device
            .set_gamma(crtc, &neutral[0], &neutral[1], &neutral[2])
            .ok()?;
        Some(Self {
            fd: device.device_fd().clone(),
            crtc,
            original,
            last: 0.0,
        })
    }

    pub(super) fn update(&mut self, warmth: f32) -> std::io::Result<()> {
        let warmth = warmth.clamp(0.0, 1.0);
        if (warmth - self.last).abs() < 0.001 && warmth != 0.0 && warmth != 1.0 {
            return Ok(());
        }
        if warmth == self.last {
            return Ok(());
        }
        let lut = ramp(self.original[0].len(), warmth);
        self.fd.set_gamma(self.crtc, &lut[0], &lut[1], &lut[2])?;
        self.last = warmth;
        Ok(())
    }
}
impl Drop for NightLight {
    fn drop(&mut self) {
        if let Err(error) = self.fd.set_gamma(
            self.crtc,
            &self.original[0],
            &self.original[1],
            &self.original[2],
        ) {
            tracing::debug!(%error, "could not restore output LUT after releasing session");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn warmth_preserves_black_monotonicity_and_neutral_endpoint() {
        let neutral = ramp(256, 0.0);
        assert!(neutral.iter().all(|c| c[0] == 0 && c[255] == 65535));
        assert_eq!(neutral[0], neutral[1]);
        assert_eq!(neutral[1], neutral[2]);
        let warm = ramp(256, 1.0);
        assert!(warm
            .iter()
            .all(|c| c[0] == 0 && c.windows(2).all(|w| w[0] <= w[1])));
        assert!(warm[0][255] > warm[1][255] && warm[1][255] > warm[2][255]);
        assert_eq!(ramp(256, 0.0), neutral);
    }
}
