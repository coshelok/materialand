//! Hit testing and rendering share the same surface placement contract.
use super::*;
use smithay::desktop::WindowSurfaceType;
pub enum WindowHit {
    Client {
        window: Window,
        surface: WlSurface,
        origin: Point<f64, Logical>,
    },
    Decoration {
        window: Window,
        geometry: Rectangle<i32, Logical>,
    },
}
impl State {
    pub fn window_under(&self, point: Point<f64, Logical>) -> Option<WindowHit> {
        for window in self.windows.space.elements().rev() {
            if !self.window_has_content(window) || self.shell.is_closing(window) {
                continue;
            }
            let coordinates = self.surface_space(window);
            if let Some((surface, offset)) =
                window.surface_under(coordinates.local(point), WindowSurfaceType::ALL)
            {
                return Some(WindowHit::Client {
                    window: window.clone(),
                    surface,
                    origin: coordinates.surface_origin(offset),
                });
            }
            if self.window_props(window).fullscreen {
                return None;
            }
            if self.window_server_decor(window) {
                if let Some(geometry) = self.windows.space.element_geometry(window) {
                    let frame = crate::shell::decor_layout(geometry);
                    if frame.frame.contains(point) && !frame.content.contains(point) {
                        return Some(WindowHit::Decoration {
                            window: window.clone(),
                            geometry,
                        });
                    }
                }
            }
        }
        None
    }
}
