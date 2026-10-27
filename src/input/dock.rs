//! Stable creation order, with running pinned applications occupying their pin slot.
use super::*;
pub fn dock_items(state: &State) -> Vec<DockItem> {
    let running: Vec<_> = state
        .windows
        .order
        .iter()
        .filter(|window| state.window_has_content(window) || state.window_props(window).minimized)
        .map(|window| {
            let meta = state.window_meta(window);
            let app = state.shell.window_app(&meta.app_id).cloned();
            let key = app
                .as_ref()
                .map(|a| a.icon.clone())
                .filter(|key| !key.is_empty())
                .unwrap_or({
                    if meta.app_id.is_empty() {
                        meta.title
                    } else {
                        meta.app_id
                    }
                });
            (window.clone(), key, app)
        })
        .collect();
    let mut items = vec![DockItem::Launcher];
    let mut used = std::collections::HashSet::new();
    for app in &state.shell.pinned {
        if let Some((i, (window, key, _))) =
            running
                .iter()
                .enumerate()
                .rev()
                .find(|(i, (_, _, candidate))| {
                    !used.contains(i)
                        && candidate
                            .as_ref()
                            .is_some_and(|a| a.identity() == app.identity())
                })
        {
            used.insert(i);
            items.push(DockItem::Focus {
                window: window.clone(),
                key: key.clone(),
                app: Some(app.clone()),
                pinned: true,
            });
        } else {
            items.push(DockItem::Launch(app.clone()));
        }
    }
    items.extend(
        running
            .into_iter()
            .enumerate()
            .filter(|(i, _)| !used.contains(i))
            .map(|(_, (window, key, app))| DockItem::Focus {
                window,
                key,
                app,
                pinned: false,
            }),
    );
    items
}
pub fn dock_item_count(state: &State) -> usize {
    dock_items(state).len()
}
