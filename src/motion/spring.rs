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

    /// Expressive shell geometry: a gentle, visible overshoot and soft landing.
    /// Tuned for desktop panel sizes; not an exact Android platform token.
    pub fn spatial(value: f32) -> Self {
        let design = crate::config::design();
        Self::at(
            value,
            design.motion.spatial_stiffness,
            design.motion.spatial_damping,
        )
    }

    /// Short press/squeeze feedback returns faster than a large panel.
    pub fn responsive(value: f32) -> Self {
        let design = crate::config::design();
        Self::at(
            value,
            design.motion.responsive_stiffness,
            design.motion.responsive_damping,
        )
    }

    /// Alpha and color effects should settle without bouncing.
    pub fn effects(value: f32) -> Self {
        let design = crate::config::design();
        Self::at(
            value,
            design.motion.effects_stiffness,
            design.motion.effects_damping,
        )
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

    /// Carry measured drag velocity into the released spring (units per second).
    pub fn set_velocity(&mut self, velocity: f32) {
        self.velocity = if velocity.is_finite() { velocity } else { 0.0 };
    }

    /// Set a stable value without simulating intermediate frames. Used when
    /// the user enables Material 3's reduced-motion accessibility preference.
    pub fn snap(&mut self, value: f32) {
        self.value = value;
        self.target = value;
        self.velocity = 0.0;
    }

    /// Finish at the configured target, discarding velocity.
    pub fn finish(&mut self) {
        self.snap(self.target);
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
