//! Window entrance geometry starts at its launch icon and advances after presentation.
use super::*;
mod entrance;
mod launches;
pub(super) use entrance::Entrance;
pub use entrance::WindowPose;
pub(super) use launches::LaunchOrigins;

impl Shell {
    pub fn remember_app_launch(&mut self, app: &AppEntry, origin: RectF) {
        if !self.reduce_motion {
            self.launch_origins.remember(app.clone(), origin);
        }
    }
    pub fn on_window_mapped(&mut self, window: &Window, app_id: &str) {
        self.window_out.remove(window);
        if self.reduce_motion {
            return;
        }
        let origin = self.launch_origins.take(app_id);
        self.window_in.insert(window.clone(), Entrance::new(origin));
    }
    pub fn set_window_launch_origin(&mut self, window: &Window, origin: RectF) {
        if !self.reduce_motion {
            self.window_in
                .insert(window.clone(), Entrance::new(Some(origin)));
        }
    }
    pub fn on_window_destroyed(&mut self, window: &Window) {
        self.window_in.remove(window);
        self.window_out.remove(window);
    }
    pub fn cancel_window_open(&mut self, window: &Window) {
        self.window_in.remove(window);
    }
    pub fn bind_window_launch_origin(&mut self, window: &Window, app_id: &str) {
        if let Some(entrance) = self.window_in.get_mut(window) {
            if entrance.origin.is_none() && !entrance.presented() {
                entrance.origin = self.launch_origins.take(app_id);
            }
        }
    }
    pub fn window_frame_built(&mut self, window: &Window) {
        if let Some(entrance) = self.window_in.get_mut(window) {
            entrance.built();
        }
    }
    pub fn window_frame_presented(&mut self) {
        for entrance in self.window_in.values_mut() {
            entrance.present();
        }
    }
    pub fn is_opening(&self, window: &Window) -> bool {
        self.window_in.contains_key(window)
    }
    pub fn window_presentation(&self, window: &Window, frame: RectF) -> WindowPose {
        if let Some(entrance) = self.window_in.get(window) {
            return entrance.pose(frame);
        }
        if let Some(s) = self.window_out.get(window) {
            let t = s.value().clamp(0.0, 1.0);
            return WindowPose::centered(frame, 1.0 - 0.06 * t, 1.0 - t);
        }
        WindowPose::centered(frame, 1.0, 1.0)
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
        let done: Vec<_> = self
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
