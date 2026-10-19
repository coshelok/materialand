//! Real panel backlight. Slow permission helpers run off the render thread.
use std::{
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc, Arc,
    },
    time::{Duration, Instant},
};

pub(super) struct Backlight {
    path: PathBuf,
    max: u32,
    requests: mpsc::Sender<(u64, u32)>,
    results: mpsc::Receiver<(u64, Result<(), String>)>,
    active: Arc<AtomicBool>,
    requested: Option<u64>,
    sequence: u64,
    last_poll: Instant,
    last_value: u32,
}

fn read(path: &Path, name: &str) -> Option<u32> {
    std::fs::read_to_string(path.join(name))
        .ok()?
        .trim()
        .parse()
        .ok()
}

/// Associate the control with the output being driven, never another GPU.
fn discover(root: &Path, card: &Path, connector: &str) -> Option<(PathBuf, u32)> {
    let card = card.file_name()?.to_str()?;
    let output = format!("{card}-{connector}");
    let mut entries: Vec<_> = std::fs::read_dir(root).ok()?.flatten().collect();
    entries.sort_by_key(|e| e.file_name());
    entries.into_iter().find_map(|entry| {
        let path = std::fs::canonicalize(entry.path()).ok()?;
        if !path
            .components()
            .any(|part| part.as_os_str() == output.as_str())
        {
            return None;
        }
        let max = read(&path, "max_brightness")?;
        (max > 0).then_some((entry.path(), max))
    })
}

fn level(fraction: f32, max: u32) -> u32 {
    (fraction.clamp(crate::shell::BRIGHTNESS_MIN, 1.0) * max as f32)
        .round()
        .max(1.0)
        .min(max as f32) as u32
}

fn write(path: &Path, name: &str, value: u32) -> Result<(), String> {
    if std::fs::write(path.join("brightness"), value.to_string()).is_ok() {
        return Ok(());
    }
    // /session/self resolves to this process's login session, also from a TTY.
    // See org.freedesktop.login1.Session.SetBrightness; no interactive polkit.
    let status = Command::new("busctl")
        .args([
            "--system",
            "--allow-interactive-authorization=no",
            "--timeout=2s",
            "call",
            "org.freedesktop.login1",
            "/org/freedesktop/login1/session/self",
            "org.freedesktop.login1.Session",
            "SetBrightness",
            "ssu",
            "backlight",
            name,
        ])
        .arg(value.to_string())
        .stdin(Stdio::null())
        .output();
    match status {
        Ok(out) if out.status.success() => Ok(()),
        Ok(out) => Err(String::from_utf8_lossy(&out.stderr).trim().to_string()),
        Err(error) => Err(error.to_string()),
    }
}

impl Backlight {
    pub(super) fn new(card: &Path, connector: &str) -> Option<Self> {
        let (path, max) = discover(Path::new("/sys/class/backlight"), card, connector)?;
        Self::at(path, max, write)
    }

    fn at(
        path: PathBuf,
        max: u32,
        apply: impl Fn(&Path, &str, u32) -> Result<(), String> + Send + 'static,
    ) -> Option<Self> {
        let last_value = level(read(&path, "brightness")? as f32 / max as f32, max);
        let name = path.file_name()?.to_str()?.to_string();
        let (requests, receive) = mpsc::channel::<(u64, u32)>();
        let (send, results) = mpsc::channel();
        let active = Arc::new(AtomicBool::new(true));
        let worker_active = active.clone();
        let worker_path = path.clone();
        std::thread::Builder::new()
            .name("backlight".into())
            .spawn(move || {
                while let Ok(mut request) = receive.recv() {
                    // A fast drag only needs its most recent requested position.
                    for newer in receive.try_iter() {
                        request = newer;
                    }
                    if !worker_active.load(Ordering::Acquire) {
                        break;
                    }
                    let (sequence, value) = request;
                    let result = apply(&worker_path, &name, value);
                    if send.send((sequence, result)).is_err() {
                        break;
                    }
                }
            })
            .ok()?;
        Some(Self {
            path,
            max,
            requests,
            results,
            active,
            requested: None,
            sequence: 0,
            last_poll: Instant::now(),
            last_value,
        })
    }

    pub(super) fn current(&self) -> Option<f32> {
        Some(read(&self.path, "brightness")? as f32 / self.max as f32)
    }

    pub(super) fn sync(&mut self, state: &mut crate::desktop::State) -> bool {
        let mut dirty = false;
        let target = level(state.brightness, self.max);
        if self.last_value != target {
            self.sequence += 1;
            if self.requests.send((self.sequence, target)).is_ok() {
                self.requested = Some(self.sequence);
                self.last_value = target;
                dirty |= state.shell.brightness_error;
                state.shell.brightness_error = false;
            } else {
                dirty |= !state.shell.brightness_error;
                state.shell.brightness_error = true;
            }
        }
        for (sequence, result) in self.results.try_iter() {
            if self.requested != Some(sequence) {
                continue;
            }
            self.requested = None;
            if let Err(error) = result {
                tracing::warn!(%error, "panel backlight update failed");
                state.shell.brightness_error = true;
            }
            if let Some(actual) = self.current() {
                state.brightness = actual;
                self.last_value = level(actual, self.max);
            }
            dirty = true;
        }
        // Pick up hardware keys and changes made outside the compositor.
        if self.requested.is_none() && self.last_poll.elapsed() >= Duration::from_secs(1) {
            self.last_poll = Instant::now();
            if let Some(actual) = self.current() {
                dirty |= (actual - state.brightness).abs() > 0.0001;
                state.brightness = actual;
                self.last_value = level(actual, self.max);
            }
        }
        dirty
    }
}

impl Drop for Backlight {
    fn drop(&mut self) {
        self.active.store(false, Ordering::Release);
    }
}

#[cfg(test)]
mod tests;
