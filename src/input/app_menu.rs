//! Context menus only use known launchable desktop entries, never window titles as commands.
use super::target::Control;
use super::*;
pub(super) fn open(state: &mut State, control: Control, point: Point<f64, Logical>) {
    let app = match control {
        Control::Dock(i) => dock_items(state)
            .get(i)
            .and_then(DockItem::application)
            .cloned(),
        Control::Launcher(LauncherHit::Tile(slot)) => state
            .shell
            .filtered_apps()
            .get(slot)
            .and_then(|i| state.shell.apps.get(*i))
            .cloned(),
        _ => None,
    };
    if let Some(app) = app {
        state.shell.open_app_menu(app, point);
    }
}
