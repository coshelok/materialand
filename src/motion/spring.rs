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
