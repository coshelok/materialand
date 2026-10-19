//! Probe.
use super::*;
pub fn probe() -> Result<(), Box<dyn std::error::Error>> {
    let (mut session, _notifier) = open_session()?;
    let seat = session.seat();
    println!("seat: {seat}");

    let mut cards = Vec::new();
    if let Ok(Some(primary)) = udev::primary_gpu(&seat) {
        println!("primary GPU: {}", primary.display());
        cards.push(primary);
    }
    for path in udev::all_gpus(&seat)? {
        if !cards.contains(&path) {
            cards.push(path);
        }
    }

    for path in cards {
        println!("\n{}:", path.display());
        let fd = match session.open(&path, OFlags::RDONLY | OFlags::CLOEXEC | OFlags::NOCTTY) {
            Ok(fd) => fd,
            Err(err) => {
                println!("  cannot open: {err:?}");
                continue;
            }
        };
        let device = probe_device::ProbeDevice::new(fd);
        let resources = match device.resource_handles() {
            Ok(resources) => resources,
            Err(err) => {
                println!("  cannot query resources: {err}");
                continue;
            }
        };
        for handle in resources.connectors() {
            match device.get_connector(*handle, false) {
                Ok(info) => {
                    println!(
                        "  connector {handle:?}: {:?} {:?}",
                        info.interface(),
                        info.state()
                    );
                    let mode = pick_mode(&info);
                    if let Some((w, h)) = info.size().filter(|(w, h)| *w > 0 && *h > 0) {
                        println!(
                            "    physical size: {w}x{h} mm ({:.1} inches)",
                            f64::from(w).hypot(f64::from(h)) / 25.4
                        );
                    }
                    match mode {
                        Some(mode) => {
                            let (w, h) = mode.size();
                            println!("    mode: {w}x{h}@{}", mode.vrefresh());
                        }
                        None => println!("    no modes reported"),
                    }
                }
                Err(err) => println!("  connector {handle:?}: {err}"),
            }
        }
    }
    Ok(())
}

mod probe_device {
    use std::os::fd::{AsFd, BorrowedFd, OwnedFd};

    /// Marker impls only: every method comes from the `drm` crate's trait
    /// providers, which issue pure getter ioctls that need no DRM master.
    pub struct ProbeDevice(OwnedFd);

    impl ProbeDevice {
        pub fn new(fd: OwnedFd) -> Self {
            Self(fd)
        }
    }

    impl AsFd for ProbeDevice {
        fn as_fd(&self) -> BorrowedFd<'_> {
            self.0.as_fd()
        }
    }

    impl smithay::reexports::drm::Device for ProbeDevice {}
    impl smithay::reexports::drm::control::Device for ProbeDevice {}
}
