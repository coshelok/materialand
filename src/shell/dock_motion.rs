//! Dock width and icon positions share the same live geometry as hit testing.
use super::*;
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
enum DockId {
    Launcher,
    App(String),
    Window(Window),
}
impl DockItem {
    fn id(&self) -> DockId {
        match self {
            Self::Launcher => DockId::Launcher,
            Self::Launch(app) => DockId::App(app.identity()),
            Self::Focus {
                app: Some(app),
                pinned: true,
                ..
            } => DockId::App(app.identity()),
            Self::Focus { window, .. } => DockId::Window(window.clone()),
        }
    }
    pub fn application(&self) -> Option<&AppEntry> {
        match self {
            Self::Launch(app) | Self::Focus { app: Some(app), .. } => Some(app),
            _ => None,
        }
    }
}
struct Slot {
    x: Spring,
    size: Spring,
    appear: Spring,
}
#[derive(Default)]
pub(super) struct DockMotion {
    width: Option<Spring>,
    order: Vec<DockId>,
    slots: HashMap<DockId, Slot>,
}
impl DockMotion {
    fn sync(&mut self, w: f32, items: &[DockItem], reduced: bool) -> bool {
        let ids: Vec<_> = items.iter().map(DockItem::id).collect();
        let first = self.width.is_none();
        let changed = ids != self.order;
        self.order = ids;
        let width = dock_rect(w, 0.0, items.len()).w;
        let spring = self.width.get_or_insert_with(|| Spring::spatial(width));
        spring.set_target(width);
        if reduced {
            spring.snap(width);
        }
        self.slots.retain(|id, _| self.order.contains(id));
        for (i, id) in self.order.iter().enumerate() {
            let rect = dock_item_rect(w, 0.0, items.len(), i);
            let x = rect.x + rect.w / 2.0 - w / 2.0;
            let slot = self.slots.entry(id.clone()).or_insert_with(|| Slot {
                x: Spring::spatial(x),
                size: Spring::spatial(rect.w),
                appear: Spring::effects(if first || reduced { 1.0 } else { 0.0 }),
            });
            slot.x.set_target(x);
            slot.size.set_target(rect.w);
            slot.appear.set_target(1.0);
            if reduced {
                slot.x.snap(x);
                slot.size.snap(rect.w);
                slot.appear.snap(1.0);
            }
        }
        changed
    }
    pub(super) fn tick(&mut self, dt: f32) -> bool {
        let mut active = self.width.as_mut().is_some_and(|s| s.step(dt));
        for slot in self.slots.values_mut() {
            active |= slot.x.step(dt);
            active |= slot.size.step(dt);
            active |= slot.appear.step(dt);
        }
        active
    }
    pub(super) fn snap(&mut self) {
        if let Some(width) = &mut self.width {
            width.finish();
        }
        for slot in self.slots.values_mut() {
            slot.x.finish();
            slot.size.finish();
            slot.appear.finish();
        }
    }

    fn active(&self) -> bool {
        self.width.is_some_and(|s| !s.settled())
            || self
                .slots
                .values()
                .any(|s| !s.x.settled() || !s.size.settled() || !s.appear.settled())
    }
}
impl Shell {
    pub fn sync_dock(&mut self, w: f32, items: &[DockItem]) -> bool {
        let changed = self.dock_motion.sync(w, items, self.reduce_motion);
        if changed {
            self.press_dock = None;
            self.hover_dock = None;
            self.press
                .retain(|key, _| !matches!(key, PressKey::Dock(_)));
        }
        changed
    }
    pub fn dock_rect(&self, w: f32, h: f32, count: usize) -> RectF {
        let design = crate::config::design();
        let mut rect = dock_rect(w, h, count);
        if let Some(width) = &self.dock_motion.width {
            rect.w = width.value().clamp(
                2.0 * design.dock.padding,
                (w - 2.0 * design.dock.margin).max(2.0 * design.dock.padding),
            );
            rect.x = (w - rect.w) / 2.0;
        }
        rect
    }
    pub fn dock_item_rect(&self, w: f32, h: f32, count: usize, index: usize) -> RectF {
        let design = crate::config::design();
        let slot = self
            .dock_motion
            .order
            .get(index)
            .and_then(|id| self.dock_motion.slots.get(id));
        if let Some(slot) = slot {
            let size = (slot.size.value() * slot.appear.value().clamp(0.0, 1.0)).max(0.0);
            RectF {
                x: w / 2.0 + slot.x.value() - size / 2.0,
                y: h - design.dock.margin - design.dock.height / 2.0 - size / 2.0,
                w: size,
                h: size,
            }
        } else {
            dock_item_rect(w, h, count, index)
        }
    }
    pub fn dock_hit(
        &self,
        w: f32,
        h: f32,
        count: usize,
        point: Point<f64, Logical>,
    ) -> Option<usize> {
        if self.dock_motion.width.is_none() {
            return dock_hit(w, h, count, point);
        }
        (0..count).find(|&i| {
            let rect = self.dock_item_rect(w, h, count, i);
            rect.w > 8.0 && rect.contains(point)
        })
    }
    pub(super) fn dock_animating(&self) -> bool {
        self.dock_motion.active()
    }
}
