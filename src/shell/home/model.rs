//! Shortcuts survive restarts independently of the dock and installed catalog.
use super::*;
impl Shell {
    pub fn load_home(&mut self) {
        if let Some(path) = config::pins_path().map(|p| p.with_file_name("home-apps")) {
            if let Err(error) = self.load_home_at(path.clone()) {
                tracing::warn!(%error, "could not read home screen shortcuts");
                self.home.pins.set_path(path);
            }
        }
    }
    fn load_home_at(&mut self, path: std::path::PathBuf) -> std::io::Result<()> {
        self.home.pins = config::Pins::load(path, vec![])?;
        self.home.apps = self
            .home
            .pins
            .ids()
            .iter()
            .filter_map(|id| {
                self.apps
                    .iter()
                    .chain(std::iter::once(&self.terminal_app))
                    .find(|a| &a.identity() == id)
                    .cloned()
            })
            .collect();
        Ok(())
    }
    pub fn is_on_home(&self, app: &AppEntry) -> bool {
        self.home.pins.contains(&app.identity())
    }
    pub fn toggle_home_app(&mut self, app: AppEntry) -> std::io::Result<()> {
        let id = app.identity();
        self.home.pins.toggle(id.clone())?;
        if self.home.pins.contains(&id) {
            self.home.apps.push(app);
        } else {
            self.home.apps.retain(|a| a.identity() != id);
        }
        self.home.cache = None;
        Ok(())
    }
    pub fn reveal_home(&mut self) {
        self.home
            .reveal
            .snap(if self.reduce_motion { 1.0 } else { 0.0 });
        self.home.reveal.set_target(1.0);
    }
    pub fn home_offset(&self) -> f32 {
        18.0 * (1.0 - self.home.reveal.value().clamp(0.0, 1.06))
    }
    pub fn turn_home_page(&mut self, w: f32, h: f32, delta: i32) {
        self.home.page = (self.home.page as i64 + i64::from(delta))
            .clamp(0, self.home_pages(w, h) as i64 - 1) as usize;
        self.home.position.set_target(self.home.page as f32);
        self.home.pressed = None;
        if self.reduce_motion {
            self.home.position.finish();
        }
    }
    pub fn clamp_home_page(&mut self, w: f32, h: f32) {
        if self.home.page >= self.home_pages(w, h) {
            self.turn_home_page(w, h, 0);
        }
    }
}
#[cfg(test)]
#[path = "tests.rs"]
mod tests;
