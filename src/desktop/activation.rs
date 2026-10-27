//! Only recent input actually delivered to a client can authorize focus transfer.
use super::{State, WlSurface};
use smithay::{reexports::wayland_server::Resource, utils::Serial};
use std::{
    collections::VecDeque,
    time::{Duration, Instant},
};

pub(super) const ACTIVATION_TIMEOUT: Duration = Duration::from_secs(5);
const INPUT_LIMIT: usize = 32;

pub(super) struct InputHistory<T> {
    entries: VecDeque<(Serial, T, Instant)>,
}
impl<T> Default for InputHistory<T> {
    fn default() -> Self {
        Self {
            entries: VecDeque::new(),
        }
    }
}
impl<T: PartialEq> InputHistory<T> {
    pub fn record(&mut self, serial: Serial, owner: T, now: Instant) {
        self.entries.retain(|(key, _, time)| {
            *key != serial && now.saturating_duration_since(*time) < ACTIVATION_TIMEOUT
        });
        self.entries.push_back((serial, owner, now));
        while self.entries.len() > INPUT_LIMIT {
            self.entries.pop_front();
        }
    }
    pub fn claim(&mut self, serial: Serial, owner: &T, now: Instant) -> Option<Instant> {
        self.entries
            .retain(|(_, _, time)| now.saturating_duration_since(*time) < ACTIVATION_TIMEOUT);
        let index = self
            .entries
            .iter()
            .position(|(key, client, _)| *key == serial && client == owner)?;
        self.entries.remove(index).map(|(_, _, time)| time)
    }
    pub fn clear(&mut self) {
        self.entries.clear();
    }
}

impl State {
    pub(crate) fn clear_activation_input(&mut self) {
        self.activation_inputs.clear();
        self.activation_state.retain_tokens(|_, _| false);
    }
    pub(crate) fn record_activation_input(&mut self, serial: Serial, surface: &WlSurface) {
        if let Some(client) = surface.client() {
            self.activation_inputs
                .record(serial, client.id(), Instant::now());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn activation_requires_recent_delivered_input_owned_by_the_requester_once() {
        let mut history = InputHistory::default();
        let now = Instant::now();
        history.record(Serial::from(12), "editor", now);
        assert!(history.claim(Serial::from(11), &"editor", now).is_none());
        assert!(history
            .claim(Serial::from(12), &"background app", now)
            .is_none());
        assert_eq!(history.claim(Serial::from(12), &"editor", now), Some(now));
        assert!(history.claim(Serial::from(12), &"editor", now).is_none());
        history.record(Serial::from(13), "editor", now);
        assert!(history
            .claim(Serial::from(13), &"editor", now + ACTIVATION_TIMEOUT)
            .is_none());
    }
    #[test]
    fn focus_changes_and_input_floods_do_not_leave_old_authorizations() {
        let mut history = InputHistory::default();
        let now = Instant::now();
        for serial in 1..=100 {
            history.record(Serial::from(serial), "editor", now);
        }
        assert!(history.claim(Serial::from(1), &"editor", now).is_none());
        assert!(history.claim(Serial::from(100), &"editor", now).is_some());
        history.clear();
        assert!(history.claim(Serial::from(99), &"editor", now).is_none());
    }
}
