//! Smithay ShmHandler adapter.
use super::super::*;
impl ShmHandler for State {
    fn shm_state(&self) -> &ShmState {
        &self.shm_state
    }
}
