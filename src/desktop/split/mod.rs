//! Two real Wayland clients share one work area; buffer commits remain client-owned.
use super::*;
use crate::{
    motion::Spring,
    shell::{self, RectF, WindowPose},
};
use smithay::reexports::wayland_protocols::xdg::shell::server::xdg_toplevel::State as XdgState;
pub struct Split {
    pub left: Window,
    pub right: Window,
    pub dragging: bool,
    drag_offset: f32,
    originals: [Rectangle<i32, Logical>; 2],
    frames: [RectF; 2],
    ratio: Spring,
    entrance: Spring,
    lower: f32,
    upper: f32,
}
pub fn pane_frames(area: Rectangle<i32, Logical>, ratio: f32, gap: f32) -> [RectF; 2] {
    let width = area.size.w.max(2) as f32;
    let gap = gap.min((width - 2.0).max(0.0));
    let left = (width * ratio - gap / 2.0).clamp(1.0, (width - gap - 1.0).max(1.0));
    [
        RectF {
            x: area.loc.x as f32,
            y: area.loc.y as f32,
            w: left,
            h: area.size.h as f32,
        },
        RectF {
            x: area.loc.x as f32 + left + gap,
            y: area.loc.y as f32,
            w: (width - left - gap).max(1.0),
            h: area.size.h as f32,
        },
    ]
}
fn rounded_frame(r: RectF) -> Rectangle<i32, Logical> {
    Rectangle::new(
        (r.x.round() as i32, r.y.round() as i32).into(),
        (
            (r.right().round() - r.x.round()).max(1.0) as i32,
            (r.bottom().round() - r.y.round()).max(1.0) as i32,
        )
            .into(),
    )
}
impl State {
    pub fn start_split(&mut self, left: Window, right: Window) -> bool {
        if left == right || !self.window_has_content(&left) || !self.window_has_content(&right) {
            return false;
        }
        let area = self.available_area();
        let gap = config::design().split.divider_width;
        let min = |state: &Self, w: &Window| {
            state
                .constrained_size(w, (1, 1).into())
                .w
                .max(config::design().split.minimum_width as i32) as f32
                + if state.window_server_decor(w) {
                    2.0 * config::design().window.border
                } else {
                    0.0
                }
        };
        let lower = (min(self, &left) + gap / 2.0) / area.size.w as f32;
        let upper = 1.0 - (min(self, &right) + gap / 2.0) / area.size.w as f32;
        let height_fits = [&left, &right].into_iter().all(|window| {
            let content = super::placement::content_area(area, self.window_server_decor(window));
            self.constrained_size(window, (1, 1).into()).h <= content.size.h
        });
        if lower > upper || !height_fits {
            self.overview.notice = Some("These apps need more space to split".into());
            return false;
        }
        self.end_split();
        let originals = [
            self.normal_restore_geometry(&left),
            self.normal_restore_geometry(&right),
        ];
        let frames = std::array::from_fn(|i| {
            let w = if i == 0 { &left } else { &right };
            if let Some(frame) = self.window_frame_rect(w) {
                return frame;
            }
            if self.window_server_decor(w) {
                shell::decor_layout(originals[i]).frame
            } else {
                let r = originals[i];
                RectF {
                    x: r.loc.x as f32,
                    y: r.loc.y as f32,
                    w: r.size.w as f32,
                    h: r.size.h as f32,
                }
            }
        });
        for window in [&left, &right] {
            if self.window_props(window).fullscreen {
                self.set_fullscreen(window, false);
            }
            if self.window_props(window).maximized {
                self.set_maximized(window, false);
            }
            if self.window_props(window).minimized {
                let serial = self.next_serial();
                self.restore_window(window, serial);
            }
            self.shell.cancel_window_open(window);
        }
        let mut entrance = Spring::spatial(0.0);
        entrance.set_target(1.0);
        if self.shell.reduce_motion {
            entrance.finish();
        }
        self.split = Some(Split {
            left: left.clone(),
            right: right.clone(),
            dragging: false,
            drag_offset: 0.0,
            originals,
            frames,
            ratio: Spring::spatial(0.5f32.clamp(lower, upper)),
            entrance,
            lower,
            upper,
        });
        self.configure_split();
        self.windows.space.raise_element(&left, false);
        self.windows.space.raise_element(&right, false);
        let serial = self.next_serial();
        self.focus_window(Some(right), serial);
        self.overview.close(self.shell.reduce_motion);
        true
    }
    pub fn split_area_for(&self, window: &Window) -> Option<Rectangle<i32, Logical>> {
        let split = self.split.as_ref()?;
        let index = if &split.left == window {
            0
        } else if &split.right == window {
            1
        } else {
            return None;
        };
        let r = pane_frames(
            self.available_area(),
            split.ratio.value().clamp(split.lower, split.upper),
            config::design().split.divider_width,
        )[index];
        Some(super::placement::content_area(
            rounded_frame(r),
            self.window_server_decor(window),
        ))
    }
    pub fn split_pose(&self, window: &Window, source: RectF) -> Option<WindowPose> {
        let split = self.split.as_ref()?;
        let index = if &split.left == window {
            0
        } else if &split.right == window {
            1
        } else {
            return None;
        };
        let area = self.split_area_for(window)?;
        let target = if self.window_server_decor(window) {
            shell::decor_layout(area).frame
        } else {
            RectF {
                x: area.loc.x as f32,
                y: area.loc.y as f32,
                w: area.size.w as f32,
                h: area.size.h as f32,
            }
        };
        let from = split.frames[index];
        let t = split.entrance.value().clamp(0.0, 1.0);
        let lerp = |a: f32, b: f32| a + (b - a) * t;
        Some(WindowPose {
            source,
            target: RectF {
                x: lerp(from.x, target.x),
                y: lerp(from.y, target.y),
                w: lerp(from.w, target.w),
                h: lerp(from.h, target.h),
            },
            alpha: self.shell.window_presentation(window, source).alpha,
        })
    }
    pub fn split_divider(&self) -> Option<RectF> {
        let split = self.split.as_ref()?;
        let area = self.available_area();
        let d = config::design().split.divider_width;
        let panes = pane_frames(area, split.ratio.value().clamp(split.lower, split.upper), d);
        Some(RectF {
            x: panes[0].right().round(),
            y: area.loc.y as f32,
            w: (panes[1].x.round() - panes[0].right().round()).max(1.0),
            h: area.size.h as f32,
        })
    }
    pub fn move_split_divider(&mut self, x: f32) {
        let area = self.available_area();
        if let Some(split) = &mut self.split {
            let offset = if split.dragging {
                split.drag_offset
            } else {
                0.0
            };
            split.ratio.set_target(
                ((x - offset - area.loc.x as f32) / area.size.w as f32)
                    .clamp(split.lower, split.upper),
            );
            if self.shell.reduce_motion {
                split.ratio.finish();
            }
        }
    }
    pub fn begin_split_drag(&mut self) {
        if self.split.is_none() {
            return;
        }
        let area = self.available_area();
        if let Some(split) = &mut self.split {
            split.dragging = true;
            let center = area.loc.x as f32
                + area.size.w as f32 * split.ratio.value().clamp(split.lower, split.upper);
            split.drag_offset = self.cursor.x as f32 - center;
            split.entrance.finish();
            self.windows.pending_resize.remove(&split.left);
            self.windows.pending_resize.remove(&split.right);
        }
        self.last_interactive_press = None;
        self.interaction = Interaction::SplitDivider;
        self.configure_split();
    }
    pub fn split_member(&self, window: &Window) -> bool {
        self.split
            .as_ref()
            .is_some_and(|s| &s.left == window || &s.right == window)
    }
    pub fn split_inner_edge(&self, window: &Window, edges: ResizeEdges) -> bool {
        self.split.as_ref().is_some_and(|s| {
            (&s.left == window && edges.right) || (&s.right == window && edges.left)
        })
    }
    pub fn split_divider_hit(&self, point: Point<f64, Logical>) -> bool {
        let Some(mut divider) = self.split_divider() else {
            return false;
        };
        let width = config::design().split.divider_hit_width;
        divider.x -= (width - divider.w) / 2.0;
        divider.w = width;
        if !divider.contains(point) {
            return false;
        }
        // Paired CSD regions and resize borders can overlap the gap. Other
        // windows and actual client popups keep their normal input priority.
        match self.window_under(point) {
            Some(super::hit_test::WindowHit::Client {
                window, surface, ..
            }) => self.split_member(&window) && self.popup_manager.find_popup(&surface).is_none(),
            Some(
                super::hit_test::WindowHit::Decoration { window, .. }
                | super::hit_test::WindowHit::Opening(window),
            ) => self.split_member(&window),
            None => true,
        }
    }
    pub fn reflow_split(&mut self) {
        let Some(split) = &self.split else {
            return;
        };
        let left = split.left.clone();
        let right = split.right.clone();
        let area = self.available_area();
        let d = &config::design().split;
        let min = |window: &Window| {
            self.constrained_size(window, (1, 1).into())
                .w
                .max(d.minimum_width as i32) as f32
                + if self.window_server_decor(window) {
                    2.0 * config::design().window.border
                } else {
                    0.0
                }
        };
        let lower = (min(&left) + d.divider_width / 2.0) / area.size.w as f32;
        let upper = 1.0 - (min(&right) + d.divider_width / 2.0) / area.size.w as f32;
        let height_fits = [&left, &right].into_iter().all(|window| {
            let content = super::placement::content_area(area, self.window_server_decor(window));
            self.constrained_size(window, (1, 1).into()).h <= content.size.h
        });
        if lower > upper || !height_fits {
            self.end_split();
            return;
        }
        if let Some(split) = &mut self.split {
            split.lower = lower;
            split.upper = upper;
            split.ratio.snap(split.ratio.value().clamp(lower, upper));
        }
        self.configure_split();
    }
    pub fn tick_split(&mut self, dt: f32) -> bool {
        let active = if let Some(split) = &mut self.split {
            if self.shell.reduce_motion {
                split.ratio.finish();
                split.entrance.finish();
            }
            split.ratio.step(dt) | split.entrance.step(dt)
        } else {
            false
        };
        if self.split.is_some() {
            self.configure_split();
        }
        active
    }
    pub fn configure_split(&mut self) {
        let Some(split) = &self.split else {
            return;
        };
        let windows = [split.left.clone(), split.right.clone()];
        let dragging = split.dragging;
        for window in windows {
            let Some(area) = self.split_area_for(&window) else {
                continue;
            };
            if let Some(top) = window.toplevel() {
                let changed = top.with_pending_state(|s| {
                    let mut changed = s.size != Some(area.size)
                        || s.bounds != Some(area.size)
                        || !s.states.contains(XdgState::TiledLeft)
                        || s.states.contains(XdgState::Resizing) != dragging;
                    s.size = Some(area.size);
                    s.bounds = Some(area.size);
                    for flag in [
                        XdgState::TiledLeft,
                        XdgState::TiledRight,
                        XdgState::TiledTop,
                        XdgState::TiledBottom,
                    ] {
                        s.states.set(flag);
                    }
                    if dragging {
                        s.states.set(XdgState::Resizing);
                    } else {
                        s.states.unset(XdgState::Resizing);
                    }
                    changed |= s.states.contains(XdgState::Maximized)
                        || s.states.contains(XdgState::Fullscreen);
                    s.states.unset(XdgState::Maximized);
                    s.states.unset(XdgState::Fullscreen);
                    changed
                });
                if changed {
                    top.send_configure();
                }
            }
            self.place_mapped_window(&window, area.loc);
        }
        self.windows.space.refresh();
    }
    pub fn end_split_if_contains(&mut self, window: &Window) {
        if self
            .split
            .as_ref()
            .is_some_and(|split| &split.left == window || &split.right == window)
        {
            self.end_split();
        }
    }
    pub fn end_split(&mut self) {
        let Some(split) = self.split.take() else {
            return;
        };
        for (i, window) in [split.left, split.right].into_iter().enumerate() {
            if !self.windows.props.contains_key(&window) {
                continue;
            }
            let bounds = self.window_available_area(&window).size;
            if let Some(top) = window.toplevel() {
                top.with_pending_state(|s| {
                    for flag in [
                        XdgState::TiledLeft,
                        XdgState::TiledRight,
                        XdgState::TiledTop,
                        XdgState::TiledBottom,
                        XdgState::Resizing,
                    ] {
                        s.states.unset(flag);
                    }
                    s.size = Some(split.originals[i].size);
                    s.bounds = Some(bounds);
                });
                top.send_configure();
            }
            self.place_mapped_window(&window, split.originals[i].loc);
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn fractional_resize_keeps_both_outer_edges_fixed_and_the_gap_open() {
        for width in [1023, 1280, 1921, 2560] {
            let area = Rectangle::new((11, 32).into(), (width, 701).into());
            for step in 250..750 {
                let frames = pane_frames(area, step as f32 / 1000.0, 12.0);
                let [left, right] = frames.map(rounded_frame);
                assert_eq!(left.loc, area.loc);
                assert_eq!(left.loc.x + left.size.w + 12, right.loc.x);
                assert_eq!(right.loc.x + right.size.w, area.loc.x + area.size.w);
                assert_eq!(left.size.h, area.size.h);
                assert_eq!(right.size.h, area.size.h);
            }
        }
    }
    #[test]
    fn panes_and_divider_cover_work_area_without_overlap() {
        let area = Rectangle::new((0, 0).into(), (1280, 744).into());
        for ratio in [0.25, 0.5, 0.75] {
            let panes = pane_frames(area, ratio, 12.0);
            assert_eq!(panes[0].x, 0.0);
            assert_eq!(panes[0].right() + 12.0, panes[1].x);
            assert_eq!(panes[1].right(), 1280.0);
            assert_eq!(panes[1].h, 744.0);
        }
    }
}
