//! Bare Super is a Home action; using it in a chord cancels that action.
use smithay::input::keyboard::Keycode;
use std::collections::HashSet;

#[derive(Default)]
pub(crate) struct SuperTap {
    down: HashSet<Keycode>,
    candidate: Option<Keycode>,
}
impl SuperTap {
    pub fn cancel(&mut self) {
        self.candidate = None;
    }
    pub fn key(&mut self, code: Keycode, pressed: bool, is_super: bool, alone: bool) -> bool {
        if !is_super {
            if pressed {
                self.cancel();
            }
            return false;
        }
        if pressed {
            // A repeated modifier press must not rearm a previously used chord.
            if self.down.insert(code) {
                self.candidate = (alone && self.down.len() == 1).then_some(code);
            }
            return false;
        }
        let fire = self.down.remove(&code) && self.candidate == Some(code);
        self.cancel();
        fire
    }
}
