//! Session acquisition and EGL construction.
use super::*;
/// A start-up error whose `Debug` view is its message.
///
/// Rust prints `Debug` for the error returned from `main`, so wrapping the
/// text in `io::Error::other` would bury it in `Custom { kind: Other, .. }`.
struct SessionOpenError(String);

impl std::fmt::Display for SessionOpenError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::fmt::Debug for SessionOpenError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for SessionOpenError {}

/// Open a libseat session, with a readable error.
///
/// libseat reports `FailedToOpenSession` when neither a running `seatd` nor a
/// logind session on a seat could be used, and the raw error gives no hint
/// about that. The README has the full troubleshooting notes.
pub(super) fn open_session(
) -> Result<(LibSeatSession, LibSeatSessionNotifier), Box<dyn std::error::Error>> {
    LibSeatSession::new().map_err(|error| {
        SessionOpenError(format!(
            "could not open a libseat session ({error}); libseat needs a logind session on \
             the seat or a running seatd daemon. Start the compositor from a display-manager \
             session or a TTY login instead of a systemd user service, or enable seatd; use \
             --nested to run inside another compositor"
        ))
        .into()
    })
}

pub(super) fn create_renderer(
    gbm: GbmDevice<DrmDeviceFd>,
) -> Result<GlesRenderer, Box<dyn std::error::Error>> {
    // SAFETY: the GBM device fd stays alive for at least as long as the EGL
    // display and there is only one EGL display per GBM device in this process.
    let display = unsafe { EGLDisplay::new(gbm)? };
    let context = EGLContext::new(&display)?;
    // SAFETY: the context is fresh and not current in any other thread.
    let renderer = unsafe { GlesRenderer::new(context)? };
    Ok(renderer)
}
