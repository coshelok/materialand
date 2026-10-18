//! Dock.
use super::*;
pub fn dock_items(state: &State) -> Vec<DockItem> {
    let mut items: Vec<DockItem> = vec![DockItem::Launcher];
    items.extend(state.shell.pinned.iter().cloned().map(DockItem::Launch));
    for window in state.windows.space.elements() {
        // A toplevel without a buffer paints nothing (see `window_has_content`)
        // and must not occupy a dock slot.
        if !state.window_has_content(window) {
            continue;
        }
        let meta = state.window_meta(window);
        let key = if meta.app_id.is_empty() {
            meta.title
        } else {
            meta.app_id
        };
        items.push(DockItem::Focus {
            window: window.clone(),
            key,
        });
    }
    // Minimized windows are unmapped but remain launchable from the dock.
    for (window, props) in &state.windows.props {
        if !props.minimized {
            continue;
        }
        let meta = state.window_meta(window);
        let key = if meta.app_id.is_empty() {
            meta.title
        } else {
            meta.app_id
        };
        items.push(DockItem::Focus {
            window: window.clone(),
            key,
        });
    }
    items
}

pub fn dock_item_count(state: &State) -> usize {
    let minimized = state.windows.props.values().filter(|p| p.minimized).count();
    let running = state
        .windows
        .space
        .elements()
        .filter(|window| state.window_has_content(window))
        .count();
    1 + state.shell.pinned.len() + running + minimized
}
