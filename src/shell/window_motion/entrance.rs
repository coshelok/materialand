//! Uniform GPU transforms keep the client's aspect ratio and protocol size intact.
use super::*;
#[derive(Clone, Copy, Debug)]
pub struct WindowPose {
    pub source: RectF,
    pub target: RectF,
    pub alpha: f32,
}
impl WindowPose {
    pub(super) fn centered(frame: RectF, scale: f32, alpha: f32) -> Self {
        Self {
            source: frame,
            target: RectF {
                x: frame.x + frame.w * (1.0 - scale) / 2.0,
                y: frame.y + frame.h * (1.0 - scale) / 2.0,
                w: frame.w * scale,
                h: frame.h * scale,
            },
            alpha,
        }
    }
    pub fn scale(&self) -> f64 {
        f64::from(self.target.w / self.source.w.max(1.0))
    }
}
pub(in crate::shell) struct Entrance {
    pub(super) origin: Option<RectF>,
    progress: Spring,
    built: bool,
    presented: bool,
    first_tick: bool,
}
impl Entrance {
    pub(super) fn new(origin: Option<RectF>) -> Self {
        let design = crate::config::design();
        let mut progress = Spring::at(
            0.0,
            design.motion.window_stiffness,
            design.motion.window_damping,
        );
        progress.set_target(1.0);
        Self {
            origin,
            progress,
            built: false,
            presented: false,
            first_tick: true,
        }
    }
    pub(super) fn built(&mut self) {
        self.built = true;
    }
    pub(super) fn presented(&self) -> bool {
        self.presented
    }
    pub(super) fn present(&mut self) {
        self.presented |= self.built;
    }
    pub(in crate::shell) fn step(&mut self, dt: f32) -> bool {
        if !self.presented {
            return true;
        }
        // Start moving on the very next frame. Bound the first step more tightly
        // so stale compositor idle time cannot jump ahead in a new entrance.
        let max_step = if std::mem::take(&mut self.first_tick) {
            1.0 / 60.0
        } else {
            1.0 / 30.0
        };
        let active = self.progress.step(dt.min(max_step));
        if self.progress.value() >= 0.999 {
            self.progress.finish();
            return false;
        }
        active
    }
    pub(in crate::shell) fn settled(&self) -> bool {
        self.progress.settled()
    }
    pub(super) fn pose(&self, frame: RectF) -> WindowPose {
        let design = crate::config::design();
        let t = self.progress.value().clamp(0.0, 1.0);
        let alpha =
            (design.motion.window_initial_alpha + t * design.motion.window_alpha_gain).min(1.0);
        let Some(icon) = self.origin else {
            return WindowPose::centered(
                frame,
                design.motion.window_initial_scale + (1.0 - design.motion.window_initial_scale) * t,
                alpha,
            );
        };
        let start_scale = (icon.w / frame.w.max(1.0))
            .min(icon.h / frame.h.max(1.0))
            .clamp(0.01, 1.0);
        let scale = start_scale + (1.0 - start_scale) * t;
        let cx = (icon.x + icon.w / 2.0) * (1.0 - t) + (frame.x + frame.w / 2.0) * t;
        let cy = (icon.y + icon.h / 2.0) * (1.0 - t) + (frame.y + frame.h / 2.0) * t;
        WindowPose {
            source: frame,
            target: RectF {
                x: cx - frame.w * scale / 2.0,
                y: cy - frame.h * scale / 2.0,
                w: frame.w * scale,
                h: frame.h * scale,
            },
            alpha,
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn frame() -> RectF {
        RectF {
            x: 300.0,
            y: 120.0,
            w: 900.0,
            h: 600.0,
        }
    }
    #[test]
    fn slow_startup_cannot_consume_animation_before_the_first_presented_frame() {
        let mut entrance = Entrance::new(None);
        for _ in 0..180 {
            entrance.step(1.0 / 30.0);
        }
        assert_eq!(entrance.pose(frame()).target.w, 774.0);
        entrance.built();
        for _ in 0..60 {
            entrance.step(1.0 / 30.0);
        }
        assert_eq!(entrance.pose(frame()).target.w, 774.0);
        entrance.present();
        entrance.step(1.0); // stale idle time is bounded, without pausing a frame
        let first = entrance.pose(frame());
        assert!(first.target.w > 774.0 && first.target.w < 850.0 && first.alpha < 0.5);
        assert!(!entrance.settled());
    }
    #[test]
    fn icon_entrance_preserves_aspect_ratio_and_lands_exactly_at_native_geometry() {
        let icon = RectF {
            x: 70.0,
            y: 680.0,
            w: 68.0,
            h: 68.0,
        };
        let mut entrance = Entrance::new(Some(icon));
        let start = entrance.pose(frame());
        assert!((start.target.x + start.target.w / 2.0 - 104.0).abs() < 0.01);
        assert!((start.target.y + start.target.h / 2.0 - 714.0).abs() < 0.01);
        assert!(start.target.w <= icon.w && start.target.h <= icon.h);
        entrance.built();
        entrance.present();
        let mut previous = start.target.w;
        let mut frames = 0;
        while !entrance.settled() {
            entrance.step(1.0 / 60.0);
            let pose = entrance.pose(frame());
            assert!(pose.target.w >= previous);
            assert!((pose.target.w / pose.target.h - 1.5).abs() < 0.001);
            previous = pose.target.w;
            frames += 1;
            assert!(frames < 180);
        }
        assert!(frames > 15);
        let end = entrance.pose(frame());
        assert_eq!(
            (
                end.target.x,
                end.target.y,
                end.target.w,
                end.target.h,
                end.alpha
            ),
            (300.0, 120.0, 900.0, 600.0, 1.0)
        );
    }
}
