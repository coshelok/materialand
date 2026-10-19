//! Stable desktop identities and conservative matching of Wayland app IDs.
use super::{AppEntry, Path};
impl AppEntry {
    pub fn identity(&self) -> String {
        self.desktop_file
            .as_ref()
            .and_then(|p| p.file_name())
            .map(|id| format!("desktop:{}", id.to_string_lossy()))
            .unwrap_or_else(|| format!("exec:{}", self.exec))
    }
    pub fn matches_app_id(&self, app_id: &str) -> bool {
        if app_id.is_empty() {
            return false;
        }
        let want = app_id.trim_end_matches(".desktop").to_lowercase();
        let desktop = self
            .desktop_file
            .as_ref()
            .and_then(|p| p.file_stem())
            .map(|s| s.to_string_lossy().to_lowercase());
        if desktop.as_deref() == Some(&want) {
            return true;
        }
        let command = self.command();
        let program = command
            .first()
            .and_then(|s| Path::new(s).file_name())
            .map(|s| s.to_string_lossy().to_lowercase());
        self.name.to_lowercase() == want
            || program
                .as_deref()
                .is_some_and(|p| p == want || want.rsplit('.').next() == Some(p))
    }
}
