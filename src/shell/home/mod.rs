//! Home screen shortcuts, paged layout and independent persistent choices.
use super::*;
mod layout;
mod model;
mod paint;
mod widgets;
pub use layout::{home_layout, HomeHit};

pub struct Home {
    pins: config::Pins,
    pub apps: Vec<AppEntry>,
    page: usize,
    position: Spring,
    reveal: Spring,
    cache: LayerCache,
    icon_motion: HashMap<String, (Spring, Spring)>,
    pub pressed: Option<HomeHit>,
}
impl Home {
    pub(super) fn new() -> Self {
        Self {
            pins: config::Pins::initial(vec![]),
            apps: vec![],
            page: 0,
            position: Spring::spatial(0.0),
            reveal: Spring::spatial(1.0),
            cache: LayerCache::default(),
            icon_motion: HashMap::new(),
            pressed: None,
        }
    }
    pub(super) fn tick(&mut self, dt: f32) -> bool {
        let mut active = self.position.step(dt) | self.reveal.step(dt);
        for (x, y) in self.icon_motion.values_mut() {
            active |= x.step(dt) | y.step(dt);
        }
        self.icon_motion
            .retain(|_, (x, y)| !x.settled() || !y.settled());
        active
    }
    pub(super) fn snap(&mut self) {
        self.position.finish();
        self.reveal.finish();
        self.icon_motion.clear();
    }
}
