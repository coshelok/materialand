//! Card-owned motion survives button release and changes to the recent-app list.
use crate::{config, motion::Spring};

pub(super) fn row_spring(value: f32) -> Spring {
    let d = &config::design().overview;
    Spring::at(value, d.reflow_stiffness, d.reflow_damping)
}

#[derive(Clone)]
pub(super) struct CardMotion {
    slot: Spring,
    slot_target: f32,
    offset: Spring,
    drag_sample: Option<(f32, u32)>,
    drag_velocity: f32,
    exit_edge: Option<f32>,
    waiting: Option<f32>,
    close_ready: bool,
}
impl CardMotion {
    pub fn new(slot: f32) -> Self {
        let d = &config::design().overview;
        Self {
            slot: row_spring(slot),
            slot_target: slot,
            offset: Spring::at(0.0, d.dismiss_stiffness, d.dismiss_damping),
            drag_sample: None,
            drag_velocity: 0.0,
            exit_edge: None,
            waiting: None,
            close_ready: false,
        }
    }
    pub fn slot(&self) -> f32 {
        self.slot.value()
    }
    pub fn slot_target(&self) -> f32 {
        self.slot_target
    }
    pub fn offset(&self) -> f32 {
        self.offset.value()
    }
    pub fn dismissing(&self) -> bool {
        self.exit_edge.is_some() || self.waiting.is_some()
    }
    pub fn occupies_slot(&self) -> bool {
        self.waiting.is_none()
    }
    pub fn retarget_slot(&mut self, slot: f32, reduced: bool) {
        self.slot_target = slot;
        self.slot.set_target(slot);
        if reduced {
            self.slot.finish();
        }
    }
    pub fn hold(&mut self) {
        self.offset.snap(self.offset.value());
        self.drag_sample = None;
        self.drag_velocity = 0.0;
    }
    pub fn drag_to(&mut self, offset: f32, time: u32) {
        if let Some((last, last_time)) = self.drag_sample {
            let elapsed = time.wrapping_sub(last_time);
            if (1..=100).contains(&elapsed) {
                let velocity = (offset - last) * 1000.0 / elapsed as f32;
                self.drag_velocity =
                    (self.drag_velocity * 0.4 + velocity * 0.6).clamp(-4000.0, 4000.0);
            } else {
                self.drag_velocity = 0.0;
            }
        }
        self.offset.snap(offset.min(0.0));
        self.drag_sample = Some((offset, time));
    }
    pub fn return_from_drag(&mut self, reduced: bool) {
        self.offset.set_target(0.0);
        self.offset.set_velocity(self.drag_velocity);
        self.drag_sample = None;
        self.drag_velocity = 0.0;
        if reduced {
            self.offset.finish();
        }
    }
    pub fn dismiss(&mut self, edge: f32, reduced: bool) {
        if self.dismissing() {
            return;
        }
        self.exit_edge = Some(edge);
        self.offset
            .set_target(edge - config::design().overview.card_height);
        self.offset.set_velocity(self.drag_velocity.min(0.0));
        self.drag_sample = None;
        self.drag_velocity = 0.0;
        if reduced {
            self.offset.finish();
        }
    }
    pub fn take_close_request(&mut self) -> bool {
        std::mem::take(&mut self.close_ready)
    }
    pub fn tick(&mut self, dt: f32, reduced: bool) -> bool {
        if reduced {
            self.slot.finish();
            self.offset.finish();
        }
        let active = self.slot.step(dt) | self.offset.step(dt);
        if self
            .exit_edge
            .is_some_and(|edge| self.offset.value() <= edge)
        {
            // Everything, including the footer/shadow, is outside the output.
            self.exit_edge = None;
            self.offset.finish();
            self.waiting = Some(0.0);
            self.close_ready = true;
            return true;
        }
        if let Some(elapsed) = &mut self.waiting {
            *elapsed += dt;
            if *elapsed >= config::design().overview.close_response_timeout {
                // A normal close can be declined or ignored. Keep such clients
                // reachable; do not kill them or lose their overview entry.
                self.waiting = None;
                self.offset.set_target(0.0);
            }
            return true;
        }
        active
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn released_card_keeps_moving_up_and_requests_close_only_after_leaving_the_screen() {
        for hz in [60, 144, 240] {
            let mut card = CardMotion::new(1.0);
            card.drag_to(-70.0, 100);
            card.drag_to(-120.0, 116);
            card.dismiss(-620.0, false);
            assert_eq!(card.offset(), -120.0, "release must not reset the drag");
            assert!(!card.take_close_request());
            let mut previous = card.offset();
            for _ in 0..hz {
                card.tick(1.0 / hz as f32, false);
                assert!(
                    card.offset() <= previous + 0.001,
                    "dismissal moved downward"
                );
                previous = card.offset();
                if card.take_close_request() {
                    assert!(card.offset() <= -620.0);
                    assert!(!card.occupies_slot());
                    break;
                }
                assert!(card.occupies_slot());
            }
            assert!(!card.occupies_slot());
            assert!(!card.take_close_request(), "close must only be sent once");
        }
    }
    #[test]
    fn neighboring_card_keeps_its_position_when_reindexed_and_moves_slowly_into_the_gap() {
        let mut next = CardMotion::new(2.0);
        next.retarget_slot(1.0, false);
        assert_eq!(next.slot(), 2.0);
        next.tick(1.0 / 144.0, false);
        assert!(next.slot() > 1.99 && next.slot() < 2.0);
        for _ in 0..144 * 3 {
            next.tick(1.0 / 144.0, false);
        }
        assert_eq!(next.slot(), 1.0);
    }
    #[test]
    fn camera_and_card_keep_the_selected_app_stationary_when_an_earlier_card_disappears() {
        let mut camera = row_spring(2.0);
        let mut selected = CardMotion::new(2.0);
        camera.set_target(1.0);
        selected.retarget_slot(1.0, false);
        for _ in 0..240 {
            camera.step(1.0 / 120.0);
            selected.tick(1.0 / 120.0, false);
            assert!((selected.slot() - camera.value()).abs() < 0.001);
        }
    }
    #[test]
    fn short_drag_returns_smoothly_and_an_ignored_close_returns_a_reachable_card() {
        let mut card = CardMotion::new(0.0);
        card.drag_to(-30.0, 100);
        card.return_from_drag(false);
        assert_eq!(card.offset(), -30.0);
        for _ in 0..240 {
            card.tick(1.0 / 120.0, false);
        }
        assert_eq!(card.offset(), 0.0);
        assert!(!card.take_close_request());
        card.dismiss(-600.0, true);
        card.tick(0.0, true);
        assert!(card.take_close_request());
        assert!(!card.occupies_slot());
        for _ in 0..600 {
            card.tick(1.0 / 120.0, false);
        }
        assert!(card.occupies_slot() && !card.dismissing());
        assert_eq!(card.offset(), 0.0);
        assert!(!card.take_close_request());
    }
}
