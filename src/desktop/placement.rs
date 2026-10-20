//! Keep client and compositor titlebars reachable through every placement path.
use super::*;
fn title_inset(server_decorated: bool) -> i32 {
    if server_decorated {
        crate::config::design().window.titlebar_height as i32
    } else {
        0
    }
}
pub(super) fn content_area(
    mut area: Rectangle<i32, Logical>,
    server_decorated: bool,
) -> Rectangle<i32, Logical> {
    if server_decorated {
        let border = crate::config::design().window.border as i32;
        let title = title_inset(true);
        area.loc.x += border;
        area.loc.y += title;
        area.size.w = (area.size.w - 2 * border).max(1);
        area.size.h = (area.size.h - title - border).max(1);
    }
    area
}
pub(super) fn reachable_location(
    area: Rectangle<i32, Logical>,
    location: Point<i32, Logical>,
    server_decorated: bool,
) -> Point<i32, Logical> {
    let min_y = area.loc.y + title_inset(server_decorated);
    Point::from((
        location
            .x
            .clamp(area.loc.x, (area.loc.x + area.size.w - 96).max(area.loc.x)),
        location
            .y
            .clamp(min_y, (area.loc.y + area.size.h - 48).max(min_y)),
    ))
}
impl State {
    pub(super) fn normal_restore_geometry(&mut self, window: &Window) -> Rectangle<i32, Logical> {
        let props = self.window_props(window);
        if let Some(saved) = props.saved {
            return saved;
        }
        if let Some(geometry) = self.windows.space.element_geometry(window) {
            return geometry;
        }
        let size = window.geometry().size;
        let size = if size.w > 0 && size.h > 0 {
            size
        } else {
            self.initial_window_size()
        };
        let location = props
            .minimized_location
            .unwrap_or_else(|| self.next_location());
        Rectangle::new(location, size)
    }
    /// A configure may precede the first buffer: only buffer commits map/focus
    /// a new window. Mode requests can reposition windows already in the scene.
    pub(super) fn place_mapped_window(&mut self, window: &Window, location: Point<i32, Logical>) {
        if !self.window_props(window).minimized
            && self.windows.space.element_location(window).is_some()
        {
            self.windows
                .space
                .map_element(window.clone(), location, false);
        }
    }

    pub fn reachable_window_location(
        &self,
        window: &Window,
        location: Point<i32, Logical>,
    ) -> Point<i32, Logical> {
        reachable_location(
            self.available_area(),
            location,
            self.window_server_decor(window),
        )
    }
    pub(super) fn reconcile_window_placement(&mut self, window: &Window) {
        let props = self.window_props(window);
        if props.minimized {
            return;
        }
        let Some(location) = self.windows.space.element_location(window) else {
            return;
        };
        let area = if props.fullscreen {
            Some(self.output_rect())
        } else if let Some(area) = self.split_area_for(window) {
            Some(area)
        } else if props.maximized {
            Some(self.window_available_area(window))
        } else {
            None
        };
        let bounds = area
            .unwrap_or_else(|| self.window_available_area(window))
            .size;
        if let Some(top) = window.toplevel() {
            let changed = top.with_pending_state(|pending| {
                let mut changed = pending.bounds != Some(bounds);
                pending.bounds = Some(bounds);
                if let Some(area) = area {
                    changed |= pending.size != Some(area.size);
                    pending.size = Some(area.size);
                }
                changed
            });
            if changed {
                top.send_configure();
            }
        }
        let target = area
            .map(|area| area.loc)
            .unwrap_or_else(|| self.reachable_window_location(window, location));
        if target != location {
            self.windows
                .space
                .map_element(window.clone(), target, false);
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn titlebar_stays_below_topbar_for_csd_and_ssd_after_resize_or_restore() {
        let area = Rectangle::new((0, 72).into(), (1280, 624).into());
        for decorated in [false, true] {
            let maximized = content_area(area, decorated);
            let loc = reachable_location(area, (20, 40).into(), decorated);
            assert_eq!(maximized.loc.y - title_inset(decorated), 72);
            assert_eq!(loc.y - title_inset(decorated), 72);
            assert!(maximized.loc.y + maximized.size.h <= area.loc.y + area.size.h);
        }
    }
    #[test]
    fn late_server_decoration_changes_move_maximized_client_down_and_reduce_height() {
        let area = Rectangle::new((0, 72).into(), (1280, 624).into());
        let csd = content_area(area, false);
        let ssd = content_area(area, true);
        assert_eq!(ssd.loc.y - csd.loc.y, 44);
        assert_eq!(csd.size.h - ssd.size.h, 45);
        assert_eq!(ssd.loc.y + ssd.size.h + 1, csd.loc.y + csd.size.h);
    }
}
