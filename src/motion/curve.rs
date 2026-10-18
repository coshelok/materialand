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
