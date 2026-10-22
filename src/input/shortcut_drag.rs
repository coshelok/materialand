//! Thresholded pointer dragging never turns a drag release into an app launch.
use super::{target::Control, *};
pub struct ShortcutDrag {
    pub app: config::AppEntry,
    pub active: bool,
    start: Point<f64, Logical>,
    edge_time: f32,
}
pub(super) fn candidate(state: &mut State, control: Control, pos: Point<f64, Logical>) {
    let app = match control {
        Control::Home(shell::HomeHit::App(i)) => state.shell.home.apps.get(i).cloned(),
        Control::Launcher(LauncherHit::Tile(slot)) => state
            .shell
            .filtered_apps()
            .get(slot)
            .and_then(|&i| state.shell.apps.get(i))
            .cloned(),
        _ => None,
    };
    state.shortcut_drag = app.map(|app| ShortcutDrag {
        app,
        active: false,
        start: pos,
        edge_time: 0.0,
    });
}
pub(super) fn motion(state: &mut State, pos: Point<f64, Logical>) -> bool {
    let Some(mut drag) = state.shortcut_drag.take() else {
        return false;
    };
    if !drag.active
        && (pos.x - drag.start.x).hypot(pos.y - drag.start.y)
            >= f64::from(config::design().drag.threshold)
    {
        if state.shell.launcher_open || state.windows.space.elements().next().is_some() {
            show_home(state);
        }
        state.shell.shortcut_error = None;
        state.shell.dragged_identity = Some(drag.app.identity());
        state.shell.cancel_presses();
        drag.active = true;
    }
    let active = drag.active;
    if active {
        state.shell.home.drop_hint = state.shell.home_drop_index(
            state.display.size.w as f32,
            state.display.size.h as f32,
            pos,
        );
    }
    state.shortcut_drag = Some(drag);
    active
}
pub(super) fn release(state: &mut State, pos: Point<f64, Logical>) -> bool {
    let Some(drag) = state.shortcut_drag.take() else {
        return false;
    };
    if !drag.active {
        return false;
    }
    let (w, h) = (state.display.size.w as f32, state.display.size.h as f32);
    let mut before = state.shell.home_reorder_snapshot(w, h);
    let size = config::design().launcher.tile_size;
    before.insert(
        drag.app.identity(),
        shell::RectF {
            x: pos.x as f32 - size / 2.0,
            y: pos.y as f32 - config::design().apps.icon_center_y,
            w: size,
            h: size,
        },
    );
    let result = if !shell::tablet_mode()
        && state
            .shell
            .dock_rect(w, h, dock_item_count(state))
            .contains(pos)
    {
        if !state.shell.is_pinned(&drag.app) {
            state.shell.toggle_pin(drag.app)
        } else {
            Ok(())
        }
    } else if let Some(index) = state.shell.home_drop_index(w, h, pos) {
        state.shell.place_home_app(drag.app, index)
    } else {
        Ok(())
    };
    if let Err(error) = result {
        state.shell.shortcut_error = Some(format!("Could not save shortcut: {error}"));
    }
    state.shell.dragged_identity = None;
    state.shell.home.drop_hint = None;
    state.shell.animate_home_reorder(w, h, before);
    state.shell.cancel_presses();
    true
}
pub(super) fn cancel(state: &mut State) {
    state.shortcut_drag = None;
    state.shell.dragged_identity = None;
    state.shell.home.drop_hint = None;
}
pub(crate) fn tick(state: &mut State, dt: f32) -> bool {
    let Some(drag) = &mut state.shortcut_drag else {
        return false;
    };
    if !drag.active {
        return false;
    }
    let d = &config::design().drag;
    let w = state.display.size.w as f32;
    let direction = if (state.cursor.x as f32) < d.page_edge {
        -1
    } else if state.cursor.x as f32 > w - d.page_edge {
        1
    } else {
        0
    };
    if direction == 0 {
        drag.edge_time = 0.0;
        return false;
    }
    drag.edge_time += dt;
    if drag.edge_time >= d.page_delay {
        drag.edge_time = 0.0;
        state
            .shell
            .turn_home_page(w, state.display.size.h as f32, direction);
        state.shell.home.drop_hint =
            state
                .shell
                .home_drop_index(w, state.display.size.h as f32, state.cursor);
    }
    true
}
