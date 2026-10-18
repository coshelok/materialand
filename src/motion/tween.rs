use super::CubicBezier;
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
