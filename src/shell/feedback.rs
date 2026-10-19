//! Feedback.
use super::*;

impl Shell {
    pub fn cancel_presses(&mut self) {
        self.press.clear();
        self.press_top = None;
        self.press_dock = None;
        self.press_tile = None;
        self.press_chip = None;
        self.press_launch = None;
        self.press_power = None;
        self.calendar.pressed = None;
    }

    pub fn press_begin(
        &mut self,
        key: PressKey,
        center: (f32, f32),
        max_r: f32,
        pressed_scale: f32,
    ) {
        let entry = self.press.entry(key).or_insert_with(PressAnim::new);
        entry.pressed = true;
        entry.scale.set_target(pressed_scale);
        if self.reduce_motion {
            entry.scale.snap(pressed_scale);
        }
        entry.ripple = Some(Ripple {
            cx: center.0,
            cy: center.1,
            max_r: max_r.max(2.0),
            progress: Tween::new(
                0.0,
                1.0,
                duration::MEDIUM3,
                CubicBezier::STANDARD_DECELERATE,
            ),
        });
    }

    pub fn press_end(&mut self, key: PressKey) {
        if let Some(entry) = self.press.get_mut(&key) {
            entry.pressed = false;
            entry.scale.set_target(1.0);
            if self.reduce_motion {
                entry.scale.snap(1.0);
            }
        }
    }

    pub(super) fn press_scale(&self, key: PressKey) -> f32 {
        self.press.get(&key).map(|p| p.scale.value()).unwrap_or(1.0)
    }

    pub fn set_hover_top(&mut self, action: Option<TopAction>) {
        self.hover_top = action;
    }

    pub fn set_hover_dock(&mut self, index: Option<usize>) -> bool {
        let changed = self.hover_dock != index;
        self.hover_dock = index;
        changed
    }

    pub fn set_hover_power(&mut self, index: Option<usize>) {
        self.hover_power = index;
    }

    pub(super) fn switch_spring(&self, action: QuickAction) -> f32 {
        self.switches[quick_action_index(action)].value()
    }

    pub(super) fn ripple_draw(&self, key: PressKey) -> Option<(f32, f32, f32, f32)> {
        let ripple = self.press.get(&key)?.ripple.as_ref()?;
        if ripple.progress.is_done() {
            return None;
        }
        let t = ripple.progress.value().clamp(0.0, 1.0);
        let alpha = 0.12 * (1.0 - t) * (1.0 - t);
        Some((ripple.cx, ripple.cy, ripple.max_r * t, alpha))
    }

    pub(super) fn draw_ripple(&self, s: &mut Surface, key: PressKey, clip: &Path, color: Color) {
        let Some((cx, cy, radius, alpha)) = self.ripple_draw(key) else {
            return;
        };
        if radius <= 0.5 {
            return;
        }
        let mut pb = PathBuilder::new();
        pb.push_circle(cx, cy, radius);
        if let Some(path) = pb.finish() {
            s.fill_path_clipped(&path, clip, Theme::with_alpha(color, alpha));
        }
    }

    pub(super) fn swatches(&mut self, dark: bool) -> &[(Color, Color, Color); 9] {
        let seed = self.spec.seed;
        let slot = if dark {
            &mut self.swatches_dark
        } else {
            &mut self.swatches_light
        };
        slot.get_or_insert_with(|| {
            std::array::from_fn(|i| {
                crate::theme::variant_swatch(
                    ThemeSpec {
                        seed,
                        variant: VARIANTS[i],
                    },
                    dark,
                )
            })
        })
    }
}
