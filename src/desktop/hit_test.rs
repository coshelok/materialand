//! Hit testing and rendering share the same surface placement contract.
use super::*;
use smithay::desktop::WindowSurfaceType;
pub enum WindowHit {
    Opening(Window),
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
            let split_pose = self
                .window_frame_rect(window)
                .and_then(|frame| self.split_pose(window, frame));
            let split_transforming = split_pose.is_some_and(|pose| {
                (pose.source.x - pose.target.x).abs() > 1.0
                    || (pose.source.y - pose.target.y).abs() > 1.0
                    || (pose.source.w - pose.target.w).abs() > 1.0
                    || (pose.source.h - pose.target.h).abs() > 1.0
            });
            if self.shell.is_opening(window) || split_transforming {
                if self.window_frame_rect(window).is_some_and(|frame| {
                    split_pose
                        .unwrap_or_else(|| self.shell.window_presentation(window, frame))
                        .target
                        .contains(point)
                }) {
                    return Some(WindowHit::Opening(window.clone()));
                }
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
