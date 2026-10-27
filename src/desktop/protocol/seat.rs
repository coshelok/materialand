//! Smithay SeatHandler adapter.
use super::super::*;
impl SeatHandler for State {
    fn cursor_image(&mut self, _: &Seat<Self>, image: smithay::input::pointer::CursorImageStatus) {
        self.cursor_status = image;
    }
    type KeyboardFocus = WlSurface;
    type PointerFocus = WlSurface;
    type TouchFocus = WlSurface;

    fn seat_state(&mut self) -> &mut SeatState<Self> {
        &mut self.seat_state
    }
    fn focus_changed(&mut self, seat: &Seat<Self>, focused: Option<&WlSurface>) {
        self.clear_activation_input();
        use smithay::reexports::wayland_server::Resource;
        let client = focused.and_then(|surface| self.display_handle.get_client(surface.id()).ok());
        smithay::wayland::selection::data_device::set_data_device_focus(
            &self.display_handle,
            seat,
            client,
        );
    }
}
