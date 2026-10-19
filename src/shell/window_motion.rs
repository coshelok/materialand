//! Window motion.
use super::*;

impl Shell {
    pub fn on_window_mapped(&mut self, window: &Window) {
        self.window_out.remove(window);
        self.window_in.entry(window.clone()).or_insert_with(|| {
            // Starts hidden at 0.0 and animates to the rest pose.
            let mut spring = Spring::effects(0.0);
            spring.set_target(1.0);
            spring
        });
    }

    pub fn on_window_destroyed(&mut self, window: &Window) {
        self.window_in.remove(window);
        self.window_out.remove(window);
    }

    pub fn begin_window_close(&mut self, window: &Window) -> bool {
        if self.reduce_motion {
            return true;
        }
        if self.window_out.contains_key(window) {
            return false;
        }
        self.window_in.remove(window);
        let mut spring = Spring::effects(0.0);
        spring.set_target(1.0);
        self.window_out.insert(window.clone(), spring);
        false
    }

    pub fn is_closing(&self, window: &Window) -> bool {
        self.window_out.contains_key(window)
    }

    pub fn take_closed_windows(&mut self) -> Vec<Window> {
        let done: Vec<Window> = self
            .window_out
            .iter()
            .filter(|(_, s)| s.value() >= 0.999)
            .map(|(w, _)| w.clone())
            .collect();
        for w in &done {
            self.window_out.remove(w);
        }
        done
    }

    pub fn window_anim(&self, window: &Window) -> Option<(f64, f32)> {
        if let Some(s) = self.window_in.get(window) {
            let t = s.value().clamp(0.0, 1.0);
            if t < 0.999 {
                return Some((0.90 + 0.10 * t as f64, t));
            }
        }
        if let Some(s) = self.window_out.get(window) {
            let t = s.value().clamp(0.0, 1.0);
            return Some((1.0 - 0.06 * t as f64, (1.0 - t).max(0.0)));
        }
        None
    }

    pub fn begin_frame(&mut self) {
        self.frame_epoch = self.frame_epoch.wrapping_add(1);
    }

    pub fn end_frame(&mut self) {
        let epoch = self.frame_epoch;
        self.decor_cache.retain(|_, entries| {
            entries.retain(|e| e.epoch == epoch);
            !entries.is_empty()
        });
    }
}
