//! Own a private session bus so application activation stays on our display.
use std::{
    io::{BufRead, BufReader},
    process::{Child, Command, Stdio},
    sync::mpsc,
    thread,
    time::Duration,
};
pub struct SessionBus {
    child: Child,
}
impl SessionBus {
    pub fn start() -> Option<Self> {
        let result = Command::new("dbus-daemon")
            .args(["--session", "--nofork", "--print-address=1"])
            .env_remove("DBUS_SESSION_BUS_ADDRESS")
            .env_remove("DBUS_STARTER_ADDRESS")
            .env_remove("DBUS_STARTER_BUS_TYPE")
            .env_remove("DISPLAY")
            .env_remove("WAYLAND_SOCKET")
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn();
        let Ok(mut child) = result else {
            disable();
            return None;
        };
        let stdout = child.stdout.take()?;
        let (tx, rx) = mpsc::sync_channel(1);
        let reader = thread::spawn(move || {
            let mut address = String::new();
            let result = BufReader::new(stdout)
                .read_line(&mut address)
                .map(|_| address);
            let _ = tx.send(result);
        });
        let address = rx
            .recv_timeout(Duration::from_secs(2))
            .ok()
            .and_then(|r| r.ok())
            .map(|a| a.trim().to_owned())
            .filter(|a| a.starts_with("unix:"));
        if let Some(address) = address {
            let _ = reader.join();
            std::env::set_var("DBUS_SESSION_BUS_ADDRESS", address);
            Some(Self { child })
        } else {
            let _ = child.kill();
            let _ = child.wait();
            let _ = reader.join();
            disable();
            None
        }
    }
}
fn disable() {
    std::env::set_var("DBUS_SESSION_BUS_ADDRESS", "unix:path=/dev/null");
    tracing::warn!("private D-Bus unavailable; client activation disabled");
}
impl Drop for SessionBus {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}
