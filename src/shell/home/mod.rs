//! Home screen shortcuts, paged layout and independent persistent choices.
use super::*;
mod layout;
mod model;
mod paint;
pub use layout::{home_layout, HomeHit};

pub struct Home {
    pins: config::Pins,
    pub apps: Vec<AppEntry>,
    page: usize,
    position: Spring,
    reveal: Spring,
    cache: LayerCache,
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
            pressed: None,
        }
    }
    pub(super) fn tick(&mut self, dt: f32) -> bool {
        self.position.step(dt) | self.reveal.step(dt)
    }
    pub(super) fn snap(&mut self) {
        self.position.finish();
        self.reveal.finish();
    }
}
