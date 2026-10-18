//! Smithay ServerDndGrabHandler adapter.
use super::super::*;
impl ServerDndGrabHandler for State {
    fn send(&mut self, _mime_type: String, _fd: std::os::unix::io::OwnedFd, _seat: Seat<Self>) {}
}
