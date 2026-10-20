//! Live recent windows include minimized clients, never invented background tasks.
use super::*;
use crate::{
    motion::Spring,
    shell::{self, RectF, WindowPose},
};
mod card_motion;
use card_motion::CardMotion;
#[derive(Clone)]
pub struct OverviewItem {
    pub window: Window,
    /// The presented frame at entry, before cancelling any window entrance.
    pub origin: WindowPose,
    motion: CardMotion,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OverviewAction {
    Select,
    Close,
    Split,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OverviewHit {
    pub index: usize,
    pub action: OverviewAction,
}
pub struct Overview {
    pub notice: Option<String>,
    pub open: bool,
    pub items: Vec<OverviewItem>,
    pub selected: usize,
    pub pending_split: Option<Window>,
    pub pressed: Option<OverviewHit>,
    pub background_pressed: bool,
    scroll_accum: f64,
    pub drag_start: Point<f64, Logical>,
    pub drag_y: f32,
    drag_base: f32,
    progress: Spring,
    position: Spring,
    reduced: bool,
}
impl Default for Overview {
    fn default() -> Self {
        Self {
            notice: None,
            open: false,
            items: vec![],
            selected: 0,
            pending_split: None,
            pressed: None,
            background_pressed: false,
            scroll_accum: 0.0,
            drag_start: (0.0, 0.0).into(),
            drag_y: 0.0,
            drag_base: 0.0,
            progress: Spring::at(
                0.0,
                config::design().overview.transition_stiffness,
                config::design().overview.transition_damping,
            ),
            position: card_motion::row_spring(0.0),
            reduced: false,
        }
    }
}
impl Overview {
    pub fn visible(&self) -> bool {
        self.open || !self.progress.settled()
    }
    pub fn progress(&self) -> f32 {
        self.progress.value().clamp(0.0, 1.0)
    }
    pub fn motion_progress(&self) -> f32 {
        self.progress.value()
    }
    pub fn has_cards(&self) -> bool {
        self.items.iter().any(|i| i.motion.occupies_slot())
    }
    pub fn card_offset(&self, index: usize) -> f32 {
        self.items.get(index).map_or(0.0, |i| i.motion.offset())
    }
    pub fn card_size(&self, w: f32, h: f32) -> (f32, f32) {
        let d = &config::design().overview;
        let width = d.card_width.min((w - 2.0 * d.margin).max(1.0));
        let height = d
            .card_height
            .min(
                (h - d.header_height
                    - d.app_header_height
                    - d.footer_height
                    - shell::bottom_inset()
                    - d.gap)
                    .max(1.0),
            )
            .min(width * d.card_height / d.card_width);
        (height * d.card_width / d.card_height, height)
    }
    pub fn card(&self, w: f32, h: f32, index: usize) -> RectF {
        let d = &config::design().overview;
        let (width, height) = self.card_size(w, h);
        let distance = self
            .items
            .get(index)
            .map_or(index as f32, |i| i.motion.slot())
            - self.position.value();
        let depth = distance.abs().min(1.0);
        let scale = 1.0 - (1.0 - d.side_scale) * depth;
        RectF {
            x: (w - width * scale) / 2.0 + distance * (width + d.gap),
            y: d.header_height
                + d.app_header_height
                + (h - d.header_height
                    - d.app_header_height
                    - shell::bottom_inset()
                    - height
                    - d.footer_height)
                    .max(0.0)
                    / 2.0
                + d.side_offset * depth
                + (height - height * scale) / 2.0
                + self.card_offset(index),
            w: width * scale,
            h: height * scale,
        }
    }
    pub fn hit(&self, w: f32, h: f32, pos: Point<f64, Logical>) -> Option<OverviewHit> {
        (0..self.items.len()).rev().find_map(|index| {
            if self.items[index].motion.dismissing() {
                return None;
            }
            let r = self.card(w, h, index);
            let scale = r.w / self.card_size(w, h).0.max(1.0);
            let full = RectF {
                y: r.y - config::design().overview.app_header_height * scale,
                h: r.h
                    + (config::design().overview.footer_height
                        + config::design().overview.app_header_height)
                        * scale,
                ..r
            };
            if !full.contains(pos) {
                return None;
            }
            let action = if pos.y as f32 >= r.bottom() {
                if pos.x as f32 > r.right() - 56.0 * scale {
                    OverviewAction::Close
                } else if pos.x as f32 > r.right() - 112.0 * scale {
                    OverviewAction::Split
                } else {
                    OverviewAction::Select
                }
            } else {
                OverviewAction::Select
            };
            Some(OverviewHit { index, action })
        })
    }
    pub fn scroll(&mut self, delta: f64, reduced: bool) {
        self.scroll_accum += delta;
        let threshold = (config::design().overview.card_width / 4.0).max(24.0) as f64;
        let steps = (self.scroll_accum / threshold).trunc() as i32;
        if steps != 0 {
            self.scroll_accum -= f64::from(steps) * threshold;
            self.navigate(steps, reduced);
        }
    }
    pub fn navigate(&mut self, delta: i32, reduced: bool) {
        let available: Vec<_> = self
            .items
            .iter()
            .enumerate()
            .filter(|(_, item)| !item.motion.dismissing())
            .map(|(i, _)| i)
            .collect();
        if available.is_empty() {
            return;
        }
        let selected = available
            .iter()
            .position(|i| *i == self.selected)
            .unwrap_or_else(|| {
                available
                    .iter()
                    .position(|i| *i >= self.selected)
                    .unwrap_or(available.len() - 1)
            });
        let next =
            (selected as i64 + i64::from(delta)).clamp(0, available.len() as i64 - 1) as usize;
        self.selected = available[next];
        self.position
            .set_target(self.items[self.selected].motion.slot_target());
        if reduced {
            self.position.finish();
        }
    }
    pub fn close(&mut self, reduced: bool) {
        if let Some(hit) = self.pressed {
            self.cancel_drag(hit.index, reduced);
        }
        self.open = false;
        self.progress.set_target(0.0);
        self.pressed = None;
        self.background_pressed = false;
        self.scroll_accum = 0.0;
        self.drag_y = 0.0;
        if reduced {
            self.progress.finish();
        }
    }
    pub fn tick(&mut self, dt: f32) -> bool {
        let mut active = self.progress.step(dt) | self.position.step(dt);
        for item in &mut self.items {
            active |= item.motion.tick(dt, self.reduced);
        }
        self.relayout(self.reduced);
        active
    }
    fn relayout(&mut self, reduced: bool) {
        let mut slot = 0.0;
        for item in &mut self.items {
            if item.motion.occupies_slot() {
                item.motion.retarget_slot(slot, reduced);
                slot += 1.0;
            }
        }
        // Hold the camera while the selected card is flying out. Choosing its
        // neighbor now would pan the whole row before the gap actually opens.
        if self
            .items
            .get(self.selected)
            .is_some_and(|i| i.motion.dismissing() && i.motion.occupies_slot())
        {
            return;
        }
        self.navigate(0, reduced);
    }
    pub fn start_drag(&mut self, hit: OverviewHit, pos: Point<f64, Logical>) {
        self.pressed = Some(hit);
        self.drag_start = pos;
        self.drag_y = 0.0;
        if let Some(item) = self.items.get_mut(hit.index) {
            self.drag_base = item.motion.offset();
            item.motion.hold();
        }
    }
    pub fn drag_to(&mut self, offset: f32, time: u32) {
        self.drag_y = offset.min(0.0);
        if let Some(item) = self.pressed.and_then(|hit| self.items.get_mut(hit.index)) {
            item.motion.drag_to(self.drag_base + self.drag_y, time);
        }
    }
    pub fn cancel_drag(&mut self, index: usize, reduced: bool) {
        if let Some(item) = self.items.get_mut(index).filter(|i| !i.motion.dismissing()) {
            item.motion.return_from_drag(reduced);
        }
    }
    pub fn cancel_press(&mut self, reduced: bool) {
        if let Some(hit) = self.pressed.take() {
            self.cancel_drag(hit.index, reduced);
        }
        self.background_pressed = false;
        self.drag_y = 0.0;
    }
}
impl State {
    pub fn tick_overview(&mut self, dt: f32) -> bool {
        self.overview.reduced = self.shell.reduce_motion;
        let active = self.overview.tick(dt);
        for item in &mut self.overview.items {
            if item.motion.take_close_request() {
                if let Some(top) = item.window.toplevel() {
                    top.send_close();
                }
            }
        }
        active
    }
    pub fn overview_frame(&self, window: &Window) -> RectF {
        if let Some(frame) = self.window_frame_rect(window) {
            return frame;
        }
        let geometry = Rectangle::new(
            self.window_props(window)
                .minimized_location
                .unwrap_or_default(),
            window.geometry().size,
        );
        if !self.window_props(window).fullscreen && self.window_server_decor(window) {
            shell::decor_layout(geometry).frame
        } else {
            RectF {
                x: geometry.loc.x as f32,
                y: geometry.loc.y as f32,
                w: geometry.size.w.max(1) as f32,
                h: geometry.size.h.max(1) as f32,
            }
        }
    }
    pub fn open_overview(&mut self) {
        crate::input::prepare_shell_overlay(self);
        self.overview.background_pressed = false;
        self.overview.pressed = None;
        self.overview.scroll_accum = 0.0;
        self.shell.cancel_presses();
        self.shell.close_app_menu();
        self.shell.launcher_open = false;
        self.shell.set_launcher_open(false);
        self.shell.set_quick_settings_open(false);
        self.shell.set_calendar_open(false);
        self.shell.set_brightness_open(false);
        self.shell.set_power_open(false);
        self.overview.notice = None;
        if !self.overview.visible() {
            self.overview.items.clear();
        }
        self.overview.open = true;
        self.overview.pending_split = None;
        self.sync_overview();
        let focused = self.windows.focused.as_ref();
        self.overview.selected = self
            .overview
            .items
            .iter()
            .position(|item| Some(&item.window) == focused)
            .unwrap_or(0);
        let slot = self
            .overview
            .items
            .get(self.overview.selected)
            .map_or(0.0, |i| i.motion.slot_target());
        self.overview.position.snap(slot);
        self.overview.progress.set_target(1.0);
        if self.shell.reduce_motion {
            self.overview.progress.finish();
        }
    }
    pub fn sync_overview(&mut self) {
        use smithay::reexports::wayland_protocols::xdg::shell::server::xdg_toplevel::State as XdgState;
        for window in self
            .windows
            .props
            .keys()
            .filter(|window| self.window_props(window).minimized)
        {
            if let Some(top) = window.toplevel() {
                let suspended = !self.overview.visible();
                let changed = top.with_pending_state(|s| {
                    let changed = s.states.contains(XdgState::Suspended) != suspended;
                    if suspended {
                        s.states.set(XdgState::Suspended);
                    } else {
                        s.states.unset(XdgState::Suspended);
                    }
                    changed
                });
                if changed {
                    top.send_configure();
                }
            }
        }
        if !self.overview.visible() {
            return;
        }

        let mut order = self.windows.recent.clone();
        for window in &self.windows.order {
            if !order.contains(window) {
                order.insert(0, window.clone());
            }
        }
        let selected = self
            .overview
            .items
            .get(self.overview.selected)
            .map(|i| i.window.clone());
        let pressed = self.overview.pressed.and_then(|hit| {
            self.overview
                .items
                .get(hit.index)
                .map(|i| (i.window.clone(), hit.action))
        });
        self.overview.items = std::mem::take(&mut self.overview.items)
            .into_iter()
            .filter(|item| {
                self.windows.props.contains_key(&item.window)
                    && self.window_has_content(&item.window)
            })
            .collect();
        for window in order.into_iter().rev() {
            if !self.window_has_content(&window)
                || self.overview.items.iter().any(|item| item.window == window)
            {
                continue;
            }
            let frame = self.overview_frame(&window);
            let origin = self
                .split_pose(&window, frame)
                .unwrap_or_else(|| self.shell.window_presentation(&window, frame));
            self.shell.cancel_window_open(&window);
            let slot = self
                .overview
                .items
                .iter()
                .filter(|i| i.motion.occupies_slot())
                .count() as f32;
            self.overview.items.push(OverviewItem {
                window,
                origin,
                motion: CardMotion::new(slot),
            });
        }
        if let Some(index) = self
            .overview
            .items
            .iter()
            .position(|i| Some(&i.window) == selected.as_ref())
        {
            self.overview.selected = index;
        }
        self.overview.pressed = pressed.and_then(|(window, action)| {
            self.overview
                .items
                .iter()
                .position(|i| i.window == window)
                .map(|index| OverviewHit { index, action })
        });
        self.overview.relayout(self.shell.reduce_motion);
        if self
            .overview
            .pending_split
            .as_ref()
            .is_some_and(|window| !self.windows.props.contains_key(window))
        {
            self.overview.pending_split = None;
        }
    }
    pub fn overview_action(&mut self, hit: OverviewHit) {
        let Some(item) = self.overview.items.get(hit.index).cloned() else {
            return;
        };
        if item.motion.dismissing() {
            return;
        }
        if hit.action != OverviewAction::Close {
            if let Some(first) = self.overview.pending_split.clone() {
                if first != item.window {
                    if self.start_split(first, item.window) {
                        self.overview.pending_split = None;
                        self.overview.notice = None;
                    }
                } else if hit.action == OverviewAction::Split {
                    self.overview.pending_split = None;
                    self.overview.notice = None;
                }
                return;
            }
        }
        match hit.action {
            OverviewAction::Close => {
                let card = self.overview.card(
                    self.display.size.w as f32,
                    self.display.size.h as f32,
                    hit.index,
                );
                let edge = -((self.display.size.h as f32)
                    .max(item.origin.target.bottom())
                    .max(card.bottom() - item.motion.offset())
                    + config::design().overview.footer_height
                    + config::design().shadow.padding);
                self.overview.items[hit.index]
                    .motion
                    .dismiss(edge, self.shell.reduce_motion);
                if self.overview.pending_split.as_ref() == Some(&item.window) {
                    self.overview.pending_split = None;
                    self.overview.notice = None;
                }
                if self.shell.reduce_motion {
                    self.tick_overview(0.0);
                }
            }
            OverviewAction::Split => {
                self.overview.pending_split = Some(item.window);
                self.overview.notice = None;
            }
            OverviewAction::Select => {
                if self.window_props(&item.window).minimized {
                    let serial = self.next_serial();
                    self.restore_window(&item.window, serial);
                }
                self.shell.cancel_window_open(&item.window);
                self.windows.space.raise_element(&item.window, false);
                let serial = self.next_serial();
                self.focus_window(Some(item.window.clone()), serial);
                self.overview.close(self.shell.reduce_motion);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn recent_cards_fit_the_work_area_and_neighbors_have_depth() {
        let overview = Overview::default();
        for (w, h) in [(640.0, 480.0), (1280.0, 800.0), (1920.0, 1080.0)] {
            let center = overview.card(w, h, 0);
            let neighbor = overview.card(w, h, 1);
            let d = &config::design().overview;
            assert!(center.x >= 0.0 && center.right() <= w);
            assert!(center.y - d.app_header_height >= shell::top_inset());
            assert!(center.bottom() + d.footer_height <= h - shell::bottom_inset());
            assert!(neighbor.w < center.w && neighbor.h < center.h);
            assert!(neighbor.y > center.y);
        }
    }
    #[test]
    fn overview_has_a_gentle_start_and_keeps_rendering_its_rebound_at_every_refresh_rate() {
        for hz in [60, 90, 120, 144, 240] {
            let mut overview = Overview {
                open: true,
                ..Default::default()
            };
            overview.progress.set_target(1.0);
            let dt = 1.0 / hz as f32;
            overview.tick(dt);
            assert!(overview.motion_progress() > 0.0 && overview.motion_progress() < 0.05);
            let mut overshot = false;
            for _ in 0..hz * 3 {
                overview.tick(dt);
                overshot |= overview.motion_progress() > 1.001;
                assert!((0.0..=1.0).contains(&overview.progress()));
            }
            assert!(overshot);
            assert_eq!(overview.motion_progress(), 1.0);
            overview.close(false);
            let mut rebound = false;
            for _ in 0..hz * 3 {
                overview.tick(dt);
                if overview.motion_progress() < -0.001 {
                    rebound = true;
                    assert!(
                        overview.visible(),
                        "closing must not cut off the spring tail"
                    );
                }
            }
            assert!(rebound);
            assert!(!overview.visible());
            overview.progress.snap(1.0);
            overview.close(true);
            assert!(!overview.visible());
        }
    }
    #[test]
    fn fine_scroll_accumulates_and_closing_clears_pending_background_click() {
        let mut overview = Overview::default();
        let threshold = f64::from(config::design().overview.card_width / 4.0);
        overview.scroll(threshold / 4.0, true);
        assert!((overview.scroll_accum - threshold / 4.0).abs() < 0.001);
        overview.scroll(threshold, true);
        assert!((overview.scroll_accum - threshold / 4.0).abs() < 0.001);
        overview.background_pressed = true;
        overview.close(true);
        assert!(!overview.background_pressed);
        assert_eq!(overview.scroll_accum, 0.0);
    }
}
