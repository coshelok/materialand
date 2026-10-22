//! Sparse placement transactions preserve gaps and never reorder unrelated apps.
use super::*;
impl Shell {
    pub fn load_home(&mut self) {
        if let Some(path) = config::pins_path().map(|p| p.with_file_name("home-apps")) {
            if let Err(error) = self.load_home_at(path) {
                tracing::warn!(%error, "could not load home workspace; keeping file untouched");
                // Refuse writes after a malformed file instead of overwriting user data.
                self.home
                    .workspace
                    .set_path("/dev/null/workspace.json".into());
            }
        }
    }
    fn load_home_at(&mut self, path: std::path::PathBuf) -> std::io::Result<()> {
        self.home.workspace = config::Workspace::load(path)?;
        self.home.apps = self
            .home
            .workspace
            .entries
            .iter()
            .filter_map(|s| {
                self.apps
                    .iter()
                    .chain(std::iter::once(&self.terminal_app))
                    .find(|a| a.identity() == s.id)
                    .cloned()
            })
            .collect();
        self.home.cache.clear();
        Ok(())
    }
    pub fn is_on_home(&self, app: &AppEntry) -> bool {
        self.home.workspace.contains(&app.identity())
    }
    pub fn toggle_home_app(&mut self, app: AppEntry) -> std::io::Result<()> {
        let id = app.identity();
        let (w, h) = self.home.last_size;
        let layout = home_layout(w, h);
        let mut entries = self.home.anchored(layout);
        if self.is_on_home(&app) {
            entries.retain(|s| s.id != id);
            self.home.workspace.commit(entries)?;
            self.home.apps.retain(|a| a.identity() != id);
        } else {
            let cells = self.home.cells(layout);
            let cell = (0..)
                .map(|i| layout.cell_at_slot(i))
                .find(|c| !cells.values().any(|v| v == c))
                .unwrap();
            entries.push(config::Shortcut {
                id,
                anchor: Some(layout.anchor(cell)),
            });
            self.home.workspace.commit(entries)?;
            self.home.apps.push(app);
        }
        self.home.cache.clear();
        Ok(())
    }
    pub fn place_home_app(&mut self, app: AppEntry, slot: usize) -> std::io::Result<()> {
        let (w, h) = self.home.last_size;
        let layout = home_layout(w, h);
        let cell = layout.cell_at_slot(slot);
        let id = app.identity();
        let cells = self.home.cells(layout);
        let previous = cells.get(&id).copied();
        let occupied = cells
            .iter()
            .find(|(other, c)| **other != id && **c == cell)
            .map(|(id, _)| id.clone());
        let mut entries = self.home.anchored(layout);
        if let Some(other) = occupied {
            let replacement = previous.unwrap_or_else(|| {
                (0..)
                    .map(|i| layout.cell_at_slot(i))
                    .find(|c| *c != cell && !cells.values().any(|v| v == c))
                    .unwrap()
            });
            entries.iter_mut().find(|s| s.id == other).unwrap().anchor =
                Some(layout.anchor(replacement));
        }
        if let Some(entry) = entries.iter_mut().find(|s| s.id == id) {
            entry.anchor = Some(layout.anchor(cell));
        } else {
            entries.push(config::Shortcut {
                id: id.clone(),
                anchor: Some(layout.anchor(cell)),
            });
        }
        self.home.workspace.commit(entries)?;
        if !self.home.apps.iter().any(|a| a.identity() == id) {
            self.home.apps.push(app);
        }
        self.home.cache.clear();
        Ok(())
    }
    pub fn home_reorder_snapshot(&self, w: f32, h: f32) -> HashMap<String, RectF> {
        self.home
            .apps
            .iter()
            .enumerate()
            .map(|(i, a)| (a.identity(), self.home_tile_rect(w, h, i)))
            .collect()
    }
    pub fn animate_home_reorder(&mut self, w: f32, h: f32, before: HashMap<String, RectF>) {
        self.home.icon_motion.clear();
        if self.reduce_motion {
            return;
        }
        for (i, app) in self.home.apps.iter().enumerate() {
            if let Some(from) = before.get(&app.identity()) {
                let target = self.home_tile_rect(w, h, i);
                let mut x = Spring::spatial(from.x - target.x);
                let mut y = Spring::spatial(from.y - target.y);
                x.set_target(0.0);
                y.set_target(0.0);
                self.home.icon_motion.insert(app.identity(), (x, y));
            }
        }
    }
    pub fn home_drop_index(&self, w: f32, h: f32, pos: Point<f64, Logical>) -> Option<usize> {
        let layout = home_layout(w, h);
        if !layout.grid.contains(pos) {
            return None;
        }
        let c = workspace::Cell {
            page: self.home.page,
            column: (((pos.x as f32 - layout.grid.x) / layout.grid.w * layout.columns as f32)
                as usize)
                .min(layout.columns - 1),
            row: (((pos.y as f32 - layout.grid.y) / layout.grid.h * layout.rows as f32) as usize)
                .min(layout.rows - 1),
        };
        Some(layout.slot(c))
    }
    pub fn reveal_home(&mut self) {
        self.home
            .reveal
            .snap(if self.reduce_motion { 1.0 } else { 0.0 });
        self.home.reveal.set_target(1.0);
    }
    pub fn home_offset(&self) -> f32 {
        let d = config::design();
        d.motion.home_offset
            * (1.0
                - self
                    .home
                    .reveal
                    .value()
                    .clamp(0.0, d.motion.overshoot_limit))
    }
    pub fn turn_home_page(&mut self, w: f32, h: f32, delta: i32) {
        let extra = usize::from(self.dragged_identity.is_some());
        self.home.page = (self.home.page as i64 + i64::from(delta))
            .clamp(0, (self.home_pages(w, h) + extra - 1) as i64) as usize;
        self.home.position.set_target(self.home.page as f32);
        self.home.pressed = None;
        if self.reduce_motion {
            self.home.position.finish();
        }
    }
    pub fn clamp_home_page(&mut self, w: f32, h: f32) {
        self.home.last_size = (w, h);
        if self.dragged_identity.is_none() && self.home.page >= self.home_pages(w, h) {
            self.turn_home_page(w, h, 0);
        }
    }
}
#[cfg(test)]
#[path = "tests.rs"]
mod tests;
