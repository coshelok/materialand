//! Resize transactions keep the anchored edge stable until the client commits.
use super::*;
#[derive(Clone, Copy, Debug)]
pub struct ResizePlacement {
    pub requested: Size<i32, Logical>,
    pub original: Rectangle<i32, Logical>,
    pub edges: ResizeEdges,
}
impl ResizePlacement {
    pub fn location(self, actual: Size<i32, Logical>) -> Point<i32, Logical> {
        let mut location = self.original.loc;
        if self.edges.left {
            location.x += self.original.size.w - actual.w;
        }
        if self.edges.top {
            location.y += self.original.size.h - actual.h;
        }
        location
    }
}
impl State {
    pub fn commit_resize(&mut self, window: &Window) {
        let Some(placement) = self.windows.pending_resize.get(window).copied() else {
            return;
        };
        let acknowledged = window
            .toplevel()
            .is_some_and(|t| t.current_state().size == Some(placement.requested));
        if !acknowledged {
            return;
        }
        let location = placement.location(window.geometry().size);
        self.windows
            .space
            .map_element(window.clone(), location, false);
        self.windows.pending_resize.remove(window);
    }
}
#[derive(Clone, Copy)]
pub(super) struct SizeLimits {
    pub min: Size<i32, Logical>,
    pub max: Size<i32, Logical>,
}
impl SizeLimits {
    pub fn constrain(self, requested: Size<i32, Logical>) -> Size<i32, Logical> {
        fn axis(value: i32, min: i32, max: i32) -> i32 {
            let lower = min.max(1);
            let upper = if max > 0 { max.max(lower) } else { i32::MAX };
            value.clamp(lower, upper)
        }
        (
            axis(requested.w, self.min.w, self.max.w),
            axis(requested.h, self.min.h, self.max.h),
        )
            .into()
    }
}
impl State {
    pub fn constrained_size(
        &self,
        window: &Window,
        requested: Size<i32, Logical>,
    ) -> Size<i32, Logical> {
        let Some(top) = window.toplevel() else {
            return requested;
        };
        compositor::with_states(top.wl_surface(), |states| {
            let mut cached = states
                .cached_state
                .get::<smithay::wayland::shell::xdg::SurfaceCachedState>();
            let current = cached.current();
            SizeLimits {
                min: current.min_size,
                max: current.max_size,
            }
            .constrain(requested)
        })
    }
    pub fn begin_resize(
        &mut self,
        window: Window,
        edges: ResizeEdges,
        geometry: Rectangle<i32, Logical>,
        button: u32,
    ) {
        let props = self.window_props(&window);
        if props.fullscreen || props.maximized {
            return;
        }
        if let Some(top) = window.toplevel() {
            top.with_pending_state(|s|s.states.set(smithay::reexports::wayland_protocols::xdg::shell::server::xdg_toplevel::State::Resizing));
            top.send_configure();
        }
        self.interaction = Interaction::Resize {
            window,
            edges,
            start_pointer: self.cursor,
            start_geometry: geometry,
            button,
        };
    }
    pub fn finish_interaction(&mut self, button: u32) {
        if self.interaction.button() != Some(button) {
            return;
        }
        if let Interaction::Resize { window, .. } = &self.interaction {
            if let Some(top) = window.toplevel() {
                top.with_pending_state(|s|s.states.unset(smithay::reexports::wayland_protocols::xdg::shell::server::xdg_toplevel::State::Resizing));
                top.send_configure();
            }
        }
        self.interaction = Interaction::None;
        self.shell.set_slider_active(false);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn client_clamping_keeps_the_opposite_corner_stationary() {
        let p = ResizePlacement {
            requested: (500, 400).into(),
            original: Rectangle::new((100, 120).into(), (400, 300).into()),
            edges: ResizeEdges {
                left: true,
                top: true,
                ..Default::default()
            },
        };
        let loc = p.location((450, 320).into());
        assert_eq!(loc, (50, 100).into());
        assert_eq!(loc.x + 450, 500);
        assert_eq!(loc.y + 320, 420);
    }

    #[test]
    fn client_maximum_and_unconstrained_axes_are_respected() {
        let limits = SizeLimits {
            min: (80, 50).into(),
            max: (200, 0).into(),
        };
        assert_eq!(limits.constrain((320, 400).into()), (200, 400).into());
        assert_eq!(limits.constrain((1, 1).into()), (80, 50).into());
    }
    #[test]
    fn inconsistent_limits_cannot_panic_or_create_empty_buffers() {
        let limits = SizeLimits {
            min: (100, 0).into(),
            max: (50, 0).into(),
        };
        assert_eq!(limits.constrain((0, 0).into()), (100, 1).into());
    }
}
