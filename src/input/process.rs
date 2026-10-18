//! Child process environment belongs to the compositor instance.
use std::process::{Command, Stdio};
pub fn spawn_program(args: &[String]) {
    let Some((program, args)) = args.split_first() else {
        return;
    };
    let mut command = Command::new(program);
    command
        .args(args)
        .env_remove("DISPLAY")
        .env_remove("WAYLAND_SOCKET")
        .env_remove("XDG_ACTIVATION_TOKEN")
        .env_remove("DBUS_STARTER_ADDRESS")
        .env_remove("DBUS_STARTER_BUS_TYPE")
        .env("XDG_SESSION_TYPE", "wayland")
        .env("XDG_CURRENT_DESKTOP", "Materialand");
    launch(command);
}
pub(super) fn spawn_system(args: &[&str]) {
    let Some((program, args)) = args.split_first() else {
        return;
    };
    let mut command = Command::new(program);
    command.args(args);
    launch(command);
}
fn launch(mut command: Command) {
    command
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    match command.spawn() {
        Ok(mut child) => {
            std::thread::spawn(move || {
                let _ = child.wait();
            });
        }
        Err(error) => tracing::warn!(?command,%error,"could not launch process"),
    }
}
