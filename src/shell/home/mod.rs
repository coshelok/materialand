//! Home screen shortcuts, paged layout and independent persistent choices.
use super::*;
mod layout;
mod model;
mod paint;
mod widgets;
mod workspace;
pub use layout::{home_layout, HomeHit};

pub struct Home {
    workspace: config::Workspace,
    last_size: (f32, f32),
    pub apps: Vec<AppEntry>,
    page: usize,
    position: Spring,
    reveal: Spring,
    cache: LayerCache,
    pub drop_hint: Option<usize>,
    icon_motion: HashMap<String, (Spring, Spring)>,
    pub pressed: Option<HomeHit>,
}
impl Home {
    pub(super) fn new() -> Self {
        Self {
            workspace: config::Workspace::default(),
            last_size: (1280.0, 800.0),
            apps: vec![],
            page: 0,
            position: Spring::spatial(0.0),
            reveal: Spring::spatial(1.0),
            cache: LayerCache::default(),
            drop_hint: None,
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
