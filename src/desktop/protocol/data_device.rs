//! Smithay DataDeviceHandler adapter.
use super::super::*;
impl DataDeviceHandler for State {
    fn data_device_state(&self) -> &DataDeviceState {
        &self.data_device_state
    }
}
