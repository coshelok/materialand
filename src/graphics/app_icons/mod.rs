//! Desktop-entry icons decoded once, before the first shell frame.
use super::canvas::Surface;
use crate::config::AppEntry;
use std::collections::HashMap;
mod lookup;
mod pixels;
mod svg;
#[derive(Default)]
pub struct AppIcons {
    images: HashMap<String, Surface>,
}
impl AppIcons {
    pub fn load(apps: &[AppEntry]) -> Self {
        let mut resolver = lookup::Resolver::system();
        let mut images = HashMap::new();
        let size = (crate::config::design().apps.medallion_radius * 2.0)
            .ceil()
            .clamp(16.0, 256.0) as u32;
        for app in apps {
            if app.icon.is_empty() || images.contains_key(&app.icon) {
                continue;
            }
            if let Some(image) = resolver
                .find(&app.icon, size)
                .and_then(|p| pixels::load(&p, size))
            {
                images.insert(app.icon.clone(), image);
            }
        }
        Self { images }
    }
    pub fn get(&self, name: &str) -> Option<&Surface> {
        self.images.get(name)
    }
}
#[cfg(test)]
mod tests;
