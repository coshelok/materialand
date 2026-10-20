//! Round live window groups in framebuffer coordinates, independent of texture transforms.
use crate::shell::RectF;
use smithay::{
    backend::renderer::{
        element::{Element, Id, Kind, RenderElement},
        gles::{
            GlesError, GlesFrame, GlesRenderer, GlesTexProgram, Uniform, UniformName, UniformType,
        },
        utils::CommitCounter,
    },
    utils::{Buffer, Physical, Rectangle, Scale, Size, Transform},
};
struct Program(Option<GlesTexProgram>);
pub(super) fn program(renderer: &mut GlesRenderer) -> Option<GlesTexProgram> {
    if let Some(p) = renderer.egl_context().user_data().get::<Program>() {
        return p.0.clone();
    }
    let result = renderer
        .compile_custom_texture_shader(
            include_str!("rounded.frag"),
            &[
                UniformName::new("screen_x", UniformType::_3f),
                UniformName::new("screen_y", UniformType::_3f),
                UniformName::new("clip_rect", UniformType::_4f),
                UniformName::new("radius", UniformType::_1f),
            ],
        )
        .map_err(|error| tracing::warn!(%error, "Rounded previews unavailable"))
        .ok();
    renderer
        .egl_context()
        .user_data()
        .insert_if_missing(|| Program(result.clone()));
    result
}
#[derive(Debug)]
pub struct RoundedElement<E> {
    inner: E,
    shader: Option<GlesTexProgram>,
    rect: RectF,
    radius: f32,
    viewport: Size<i32, Physical>,
    id: Id,
}
impl<E> RoundedElement<E> {
    pub fn new(
        inner: E,
        shader: Option<GlesTexProgram>,
        rect: RectF,
        radius: f32,
        viewport: Size<i32, Physical>,
    ) -> Self {
        Self {
            inner,
            shader,
            rect,
            radius,
            viewport,
            id: Id::new(),
        }
    }
}
impl<E: Element> Element for RoundedElement<E> {
    fn id(&self) -> &Id {
        &self.id
    }
    fn current_commit(&self) -> CommitCounter {
        self.inner.current_commit()
    }
    fn src(&self) -> Rectangle<f64, Buffer> {
        self.inner.src()
    }
    fn transform(&self) -> Transform {
        self.inner.transform()
    }
    fn geometry(&self, scale: Scale<f64>) -> Rectangle<i32, Physical> {
        self.inner.geometry(scale)
    }
    fn alpha(&self) -> f32 {
        self.inner.alpha()
    }
    fn kind(&self) -> Kind {
        self.inner.kind()
    }
    // A moving mask damages the frame; clipped textures cannot be directly scanned out.
}
impl<E: RenderElement<GlesRenderer>> RenderElement<GlesRenderer> for RoundedElement<E> {
    fn draw(
        &self,
        frame: &mut GlesFrame<'_, '_>,
        src: Rectangle<f64, Buffer>,
        dst: Rectangle<i32, Physical>,
        damage: &[Rectangle<i32, Physical>],
        _opaque: &[Rectangle<i32, Physical>],
    ) -> Result<(), GlesError> {
        let Some(shader) = &self.shader else {
            return self.inner.draw(frame, src, dst, damage, &[]);
        };
        let [x, y] = screen_from_framebuffer(frame.projection(), self.viewport);
        frame.override_default_tex_program(
            shader.clone(),
            vec![
                Uniform::new("screen_x", (x[0], x[1], x[2])),
                Uniform::new("screen_y", (y[0], y[1], y[2])),
                Uniform::new(
                    "clip_rect",
                    (self.rect.x, self.rect.y, self.rect.w, self.rect.h),
                ),
                Uniform::new(
                    "radius",
                    self.radius.min(self.rect.w.min(self.rect.h) / 2.0).max(0.0),
                ),
            ],
        );
        let result = self.inner.draw(frame, src, dst, damage, &[]);
        frame.clear_tex_program_override();
        result
    }
}
fn screen_from_framebuffer(m: &[f32; 9], size: Size<i32, Physical>) -> [[f32; 3]; 2] {
    let det = m[0] * m[4] - m[3] * m[1];
    let sx = 2.0 / size.w.max(1) as f32;
    let sy = 2.0 / size.h.max(1) as f32;
    [
        [
            m[4] * sx / det,
            -m[3] * sy / det,
            (m[3] * m[7] - m[4] * m[6] - m[4] + m[3]) / det,
        ],
        [
            -m[1] * sx / det,
            m[0] * sy / det,
            (m[1] * m[6] - m[0] * m[7] + m[1] - m[0]) / det,
        ],
    ]
}
#[cfg(test)]
#[path = "rounded_tests.rs"]
mod tests;
