//! Runtime directory and per-instance client socket ownership.
use crate::Result;
use smithay::reexports::wayland_server::ListeningSocket;
use std::{
    os::unix::fs::{MetadataExt, PermissionsExt},
    path::{Path, PathBuf},
};
pub fn valid_directory(path: &Path, uid: u32) -> bool {
    path.is_absolute()
        && std::fs::metadata(path)
            .is_ok_and(|m| m.is_dir() && m.uid() == uid && m.mode() & 0o077 == 0)
}
pub fn prepare() -> Result<()> {
    let uid = smithay::reexports::rustix::process::getuid().as_raw();
    if let Some(path) = std::env::var_os("XDG_RUNTIME_DIR") {
        if valid_directory(Path::new(&path), uid) {
            return Ok(());
        }
    }
    let system = PathBuf::from(format!("/run/user/{uid}"));
    if valid_directory(&system, uid) {
        std::env::set_var("XDG_RUNTIME_DIR", system);
        return Ok(());
    }
    let path = std::env::temp_dir().join(format!("materialand-{uid}-{}", std::process::id()));
    std::fs::create_dir(&path)?;
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o700))?;
    if !valid_directory(&path, uid) {
        return Err("invalid runtime directory".into());
    }
    std::env::set_var("XDG_RUNTIME_DIR", path);
    Ok(())
}
pub fn socket() -> Result<ListeningSocket> {
    let name = format!("materialand-{}", std::process::id());
    let socket = ListeningSocket::bind(&name)?;
    std::env::set_var("WAYLAND_DISPLAY", &name);
    std::env::remove_var("WAYLAND_SOCKET");
    std::env::set_var("XDG_SESSION_TYPE", "wayland");
    std::env::set_var("XDG_CURRENT_DESKTOP", "Materialand");
    Ok(socket)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn runtime_directory_requires_absolute_private_owned_path() {
        let path =
            std::env::temp_dir().join(format!("materialand-runtime-test-{}", std::process::id()));
        std::fs::create_dir_all(&path).unwrap();
        let uid = smithay::reexports::rustix::process::getuid().as_raw();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o700)).unwrap();
        assert!(valid_directory(&path, uid));
        assert!(!valid_directory(Path::new("."), uid));
        assert!(!valid_directory(&path, uid + 1));
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
        assert!(!valid_directory(&path, uid));
        std::fs::remove_dir(&path).unwrap();
    }
}
