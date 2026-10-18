//! A private D-Bus session for applications launched by this compositor.
//!
//! Reusing the parent desktop's session bus lets D-Bus activation hand a request
//! to an application process that belongs to the parent compositor. That process
//! retains its original WAYLAND_DISPLAY, even when the requesting child had the
//! correct one. A private bus keeps activation scoped to this compositor.

use std::{
    io::{BufRead, BufReader},
    process::{Child, Command, Stdio},
};

/// Owns the private session bus for as long as this compositor instance runs.
pub struct IsolatedSessionBus {
    child: Child,
}

/// Start a private session bus and publish its address to future child processes.
///
/// If dbus-daemon is unavailable or fails to start, we replace the inherited
/// host address with a deliberately unreachable Unix-socket address. Some
/// D-Bus libraries discover a default bus when the variable is unset; merely
/// removing it would therefore not guarantee isolation. D-Bus-dependent apps
/// may fail to start, but they cannot silently use the parent's bus.
pub fn start() -> Option<IsolatedSessionBus> {
    let child_result = Command::new("dbus-daemon")
        .args(["--session", "--nofork", "--print-address=1"])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn();

    let mut child = match child_result {
        Ok(child) => child,
        Err(error) => {
            tracing::warn!(%error, "could not start a private D-Bus session; disabling D-Bus activation for clients");
            disable_inherited_bus();
            return None;
        }
    };

    let mut address = String::new();
    let address_result = match child.stdout.take() {
        Some(stdout) => BufReader::new(stdout).read_line(&mut address),
        None => Err(std::io::Error::new(
            std::io::ErrorKind::BrokenPipe,
            "dbus-daemon did not provide stdout",
        )),
    };

    if let Err(error) = address_result {
        tracing::warn!(%error, "could not read the private D-Bus address");
        let _ = child.kill();
        let _ = child.wait();
        disable_inherited_bus();
        return None;
    }

    let address = address.trim();
    if address.is_empty() || !address.starts_with("unix:") {
        tracing::warn!("dbus-daemon returned an invalid session-bus address");
        let _ = child.kill();
        let _ = child.wait();
        clear_inherited_bus();
        return None;
    }

    // Child applications inherit this new address; do not leak the bus-starter
    // metadata for the host's bus into activatable services.
    std::env::set_var("DBUS_SESSION_BUS_ADDRESS", address);
    std::env::remove_var("DBUS_STARTER_ADDRESS");
    std::env::remove_var("DBUS_STARTER_BUS_TYPE");
    std::env::remove_var("DBUS_SESSION_BUS_PID");

    tracing::info!("started a private D-Bus session for compositor clients");
    Some(IsolatedSessionBus { child })
}

fn disable_inherited_bus() {
    // Do not leave the address unset: GLib/other libraries may discover the
    // user's default bus socket and reconnect to the host desktop.
    std::env::set_var("DBUS_SESSION_BUS_ADDRESS", "unix:path=/dev/null");
    std::env::remove_var("DBUS_STARTER_ADDRESS");
    std::env::remove_var("DBUS_STARTER_BUS_TYPE");
    std::env::remove_var("DBUS_SESSION_BUS_PID");
}

impl Drop for IsolatedSessionBus {
    fn drop(&mut self) {
        // --nofork keeps the daemon as our direct child. Reap it on normal
        // compositor shutdown so a private bus is not left behind.
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}
