//! One coordinate contract for client buffers, window geometry and popup origins.
use smithay::utils::{Logical, Point};
#[derive(Clone, Copy, Debug)]
pub struct SurfaceSpace {
    origin: Point<i32, Logical>,
}
impl SurfaceSpace {
    pub fn new(window_location: Point<i32, Logical>, geometry_offset: Point<i32, Logical>) -> Self {
        Self {
            origin: window_location - geometry_offset,
        }
    }
    pub fn origin(self) -> Point<i32, Logical> {
        self.origin
    }
    pub fn local(self, global: Point<f64, Logical>) -> Point<f64, Logical> {
        global - self.origin.to_f64()
    }
    pub fn surface_origin(self, offset: Point<i32, Logical>) -> Point<f64, Logical> {
        (self.origin + offset).to_f64()
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn inset_client_decorations_use_buffer_origin_for_input() {
        let space = SurfaceSpace::new((300, 200).into(), (10, 10).into());
        assert_eq!(space.origin(), (290, 190).into());
        assert_eq!(space.local((320.0, 230.0).into()), (30.0, 40.0).into());
    }
    #[test]
    fn popups_may_start_outside_parent_geometry() {
        let space = SurfaceSpace::new((100, 80).into(), (12, 8).into());
        let origin = space.surface_origin((-40, -30).into());
        assert_eq!(origin, (48.0, 42.0).into());
        assert_eq!(
            Point::from((63.0, 62.0)) - origin,
            Point::from((15.0, 20.0))
        );
    }
}
