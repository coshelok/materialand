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

    /// Analytic damped motion: equal elapsed time produces the same pose at
    /// every refresh rate, including irregular frame intervals.
    pub fn step(&mut self, dt: f32) -> bool {
        if !dt.is_finite() || dt <= 0.0 {
            return !self.settled();
        }
        // This is an analytic solution, so a long frame does not require a
        // stability clamp. Dropping elapsed time would slow the animation down
        // exactly when the compositor is already missing frames.
        let t = f64::from(dt);
        let x = f64::from(self.value - self.target);
        let v = f64::from(self.velocity);
        let a = f64::from(self.damping) / 2.0;
        let k = f64::from(self.stiffness);
        let discriminant = a * a - k;
        let (x, v) = if discriminant.abs() < k * 1e-6 {
            let b = v + a * x;
            let decay = (-a * t).exp();
            ((x + b * t) * decay, (v - a * b * t) * decay)
        } else if discriminant < 0.0 {
            let w = (-discriminant).sqrt();
            let (sin, cos) = (w * t).sin_cos();
            let b = (v + a * x) / w;
            let decay = (-a * t).exp();
            (
                decay * (x * cos + b * sin),
                decay * (v * cos - (a * b + w * x) * sin),
            )
        } else {
            let root = discriminant.sqrt();
            let r1 = -a + root;
            let r2 = -a - root;
            let c1 = (v - r2 * x) / (r1 - r2);
            let c2 = x - c1;
            let e1 = (r1 * t).exp();
            let e2 = (r2 * t).exp();
            (c1 * e1 + c2 * e2, r1 * c1 * e1 + r2 * c2 * e2)
        };
        self.value = self.target + x as f32;
        self.velocity = v as f32;
        if self.settled() {
            self.value = self.target;
            self.velocity = 0.0;
            return false;
        }
        true
    }
}
