//! Client metadata normalization.
use super::*;
pub(super) fn display_title(title: Option<String>) -> String {
    let Some(title) = title else {
        return "Window".to_string();
    };
    let trimmed = title.trim();
    if trimmed.starts_with('/') && !trimmed.chars().any(char::is_whitespace) {
        if let Some(name) = std::path::Path::new(trimmed).file_name() {
            let name = name.to_string_lossy();
            if !name.is_empty() {
                return name.into_owned();
            }
        }
    }
    title
}

pub(super) fn default_meta(window: &Window) -> WindowMeta {
    let data = window.toplevel().and_then(|t| {
        let surface = t.wl_surface().clone();
        compositor::with_states(&surface, |states| {
            states
                .data_map
                .get::<XdgToplevelSurfaceData>()
                .and_then(|data| {
                    let guard = data.lock().ok()?;
                    Some((guard.title.clone(), guard.app_id.clone()))
                })
        })
    });
    let (title, app_id) = data.unwrap_or((None, None));
    WindowMeta {
        title: display_title(title),
        app_id: app_id.unwrap_or_default(),
    }
}
