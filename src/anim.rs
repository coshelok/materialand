//! Material 3 motion: cubic-bezier easing curves and damped springs.
//!
//! The shell animates with two tools from the M3 spec:
//! * **Easing curves** for time-bound transitions (fades, scrims, page changes),
//!   expressed as CSS-style cubic beziers.
//! * **Springs** for interactive/adaptive motion (launcher, brightness slider,
//!   window open/close) where velocity continuity matters.
//!
//! Both are advanced by a single `dt` per frame; callers redraw while any
//! animation reports itself as active.

/// A cubic-bezier timing function with fixed endpoints `(0,0)` and `(1,1)`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CubicBezier {
    x1: f32,
    y1: f32,
    x2: f32,
    y2: f32,
}

impl CubicBezier {
    pub const fn new(x1: f32, y1: f32, x2: f32, y2: f32) -> Self {
        Self { x1, y1, x2, y2 }
    }

    /// M3 curve for elements entering the screen.
    pub const STANDARD_DECELERATE: Self = Self::new(0.0, 0.0, 0.0, 1.0);
    /// M3 emphasized decelerate curve: fast start, smooth settling.
    pub const EMPHASIZED_DECELERATE: Self = Self::new(0.05, 0.7, 0.1, 1.0);

    fn axis(&self, t: f32, a1: f32, a2: f32) -> f32 {
        let mt = 1.0 - t;
        3.0 * mt * mt * t * a1 + 3.0 * mt * t * t * a2 + t * t * t
    }

    fn axis_derivative(&self, t: f32, a1: f32, a2: f32) -> f32 {
        let mt = 1.0 - t;
        3.0 * mt * mt * a1 + 6.0 * mt * t * (a2 - a1) + 3.0 * t * t * (1.0 - a2)
    }

    /// Evaluate the curve's output for a normalized input `x` in `[0, 1]`.
    pub fn eval(&self, x: f32) -> f32 {
        let x = x.clamp(0.0, 1.0);
        // Newton-Raphson to invert x(t); fall back to bisection when the
        // derivative is too flat (e.g. the flat start of emphasized curves).
        let mut t = x;
        for _ in 0..8 {
            let dx = self.axis(t, self.x1, self.x2) - x;
            if dx.abs() < 1e-5 {
                return self.axis(t, self.y1, self.y2);
            }
            let d = self.axis_derivative(t, self.x1, self.x2);
            if d.abs() < 1e-6 {
                break;
            }
            t -= dx / d;
        }
        let (mut lo, mut hi) = (0.0f32, 1.0f32);
        t = x;
        for _ in 0..24 {
            let cx = self.axis(t, self.x1, self.x2);
            if (cx - x).abs() < 1e-5 {
                break;
            }
            if cx < x {
                lo = t;
            } else {
                hi = t;
            }
            t = (lo + hi) * 0.5;
        }
        self.axis(t, self.y1, self.y2)
    }
}

/// Named Material 3 duration token used by the shell, in seconds.
pub mod duration {
    pub const MEDIUM3: f32 = 0.350;
    pub const LONG1: f32 = 0.450;
    pub const LONG2: f32 = 0.500;
}

/// A critically-ish damped spring driving a single scalar.
#[derive(Clone, Copy, Debug)]
pub struct Spring {
    value: f32,
    target: f32,
    velocity: f32,
    stiffness: f32,
    damping: f32,
}

impl Spring {
    /// A spring that has settled at `value`.
    pub fn at(value: f32, stiffness: f32, damping_ratio: f32) -> Self {
        Self {
            value,
            target: value,
            velocity: 0.0,
            stiffness,
            damping: 2.0 * damping_ratio * stiffness.sqrt(),
        }
    }

    // ---- Official Material 3 Expressive motion-physics tokens -------------
    //
    // Spatial springs drive position and size; they may slightly overshoot and
    // "bounce". Effects springs drive color, alpha and other non-spatial
    // properties; they are critically damped and never overshoot. The full
    // token table — spatial fast/default/slow at stiffness 1400/700/300 with
    // ζ = 0.9, effects fast/default/slow at 3800/1600/800 with ζ = 1.0 — is
    // built through `at`; the shell uses the spatial default directly.

    /// Spatial default: the standard position/size spring (stiffness 700).
    pub fn spatial(value: f32) -> Self {
        Self::at(value, 700.0, 0.9)
    }

    pub fn value(&self) -> f32 {
        self.value
    }

    /// True once the spring has effectively reached its target.
    pub fn settled(&self) -> bool {
        (self.value - self.target).abs() < 1e-3 && self.velocity.abs() < 1e-3
    }

    pub fn set_target(&mut self, target: f32) {
        self.target = target;
    }

    /// Set a stable value without simulating intermediate frames. Used when
    /// the user enables Material 3's reduced-motion accessibility preference.
    pub fn snap(&mut self, value: f32) {
        self.value = value;
        self.target = value;
        self.velocity = 0.0;
    }

    /// Advance the simulation by `dt` seconds using fixed sub-steps for
    /// stability. Returns `true` while the spring is still moving.
    pub fn step(&mut self, dt: f32) -> bool {
        let dt = dt.clamp(0.0, 0.05);
        const SUB: f32 = 1.0 / 240.0;
        let mut remaining = dt;
        while remaining > 0.0 {
            let h = remaining.min(SUB);
            let accel = -self.stiffness * (self.value - self.target) - self.damping * self.velocity;
            self.velocity += accel * h;
            self.value += self.velocity * h;
            remaining -= h;
        }
        if self.settled() {
            self.value = self.target;
            self.velocity = 0.0;
            return false;
        }
        true
    }
}

/// A time-bound eased transition from `from` to `to`.
#[derive(Clone, Copy, Debug)]
pub struct Tween {
    from: f32,
    to: f32,
    elapsed: f32,
    duration: f32,
    curve: CubicBezier,
}

impl Tween {
    pub fn new(from: f32, to: f32, duration: f32, curve: CubicBezier) -> Self {
        Self {
            from,
            to,
            elapsed: 0.0,
            duration: duration.max(1e-4),
            curve,
        }
    }

    pub fn start(&mut self, from: f32, to: f32) {
        self.from = from;
        self.to = to;
        self.elapsed = 0.0;
    }

    /// Complete a tween immediately, without leaving a stale active animation.
    pub fn snap(&mut self, value: f32) {
        self.from = value;
        self.to = value;
        self.elapsed = self.duration;
    }

    pub fn is_done(&self) -> bool {
        self.elapsed >= self.duration
    }

    pub fn value(&self) -> f32 {
        let t = (self.elapsed / self.duration).clamp(0.0, 1.0);
        self.from + (self.to - self.from) * self.curve.eval(t)
    }

    /// Advance by `dt`; returns `true` while the tween is still running.
    pub fn step(&mut self, dt: f32) -> bool {
        self.elapsed += dt;
        !self.is_done()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn curve_has_correct_endpoints() {
        let c = CubicBezier::EMPHASIZED_DECELERATE;
        assert!((c.eval(0.0)).abs() < 1e-3, "f(0) must be 0");
        assert!((c.eval(1.0) - 1.0).abs() < 1e-3, "f(1) must be 1");
    }

    #[test]
    fn curve_is_monotonic() {
        let c = CubicBezier::EMPHASIZED_DECELERATE;
        let mut last = c.eval(0.0);
        for i in 1..=100 {
            let v = c.eval(i as f32 / 100.0);
            assert!(v >= last - 1e-4, "curve must not regress");
            last = v;
        }
    }

    #[test]
    fn spring_settles_at_target() {
        let mut s = Spring::spatial(0.0);
        s.set_target(1.0);
        let mut steps = 0;
        while s.step(1.0 / 60.0) {
            steps += 1;
            assert!(steps < 600, "spring failed to settle");
        }
        assert!((s.value() - 1.0).abs() < 1e-3);
    }

    #[test]
    fn settled_spring_reports_inactive() {
        let mut s = Spring::spatial(0.3);
        assert!(!s.step(1.0 / 60.0), "an idle spring must not request frames");
        assert!(s.settled());
    }

    #[test]
    fn spatial_springs_settle() {
        for mut s in [
            Spring::at(0.0, 1400.0, 0.9), // spatial fast
            Spring::at(0.0, 700.0, 0.9),  // spatial default
            Spring::at(0.0, 300.0, 0.9),  // spatial slow
        ] {
            s.set_target(1.0);
            let mut steps = 0;
            while s.step(1.0 / 60.0) {
                steps += 1;
                assert!(steps < 1200, "spatial spring failed to settle");
            }
            assert!(s.settled());
        }
    }

    #[test]
    fn effects_springs_never_overshoot() {
        for mut s in [
            Spring::at(0.0, 3800.0, 1.0), // effects fast
            Spring::at(0.0, 1600.0, 1.0), // effects default
            Spring::at(0.0, 800.0, 1.0),  // effects slow
        ] {
            s.set_target(1.0);
            for _ in 0..900 {
                s.step(1.0 / 60.0);
                assert!(
                    s.value() <= 1.0 + 1e-3,
                    "critically damped spring overshot: {}",
                    s.value()
                );
            }
        }
    }

    #[test]
    fn tween_reaches_endpoints() {
        let mut t = Tween::new(0.0, 1.0, 0.2, CubicBezier::EMPHASIZED_DECELERATE);
        assert!(t.step(0.0), "tween is active before the duration elapses");
        assert!(!t.step(0.2), "tween must finish at its duration");
        assert!((t.value() - 1.0).abs() < 1e-3);
    }

    #[test]
    fn tween_is_monotonic_for_decelerate() {
        let mut t = Tween::new(0.0, 10.0, 0.5, CubicBezier::EMPHASIZED_DECELERATE);
        let mut last = t.value();
        while t.step(1.0 / 120.0) {
            let v = t.value();
            assert!(v >= last - 1e-4, "decelerate tween must not regress");
            last = v;
        }
    }
}
