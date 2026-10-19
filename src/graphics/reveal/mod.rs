//! Reveal cached textures on the GPU; no per-frame CPU masks or texture uploads.
use crate::shell::CircleMask;
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
    if let Some(cached) = renderer.egl_context().user_data().get::<Program>() {
        return cached.0.clone();
    }
    let program = renderer
        .compile_custom_texture_shader(
            include_str!("clip.frag"),
            &[
                UniformName::new("screen_mapping", UniformType::_4f),
                UniformName::new("circle", UniformType::_3f),
            ],
        )
        .map_err(|error| tracing::warn!(%error, "GPU theme reveal unavailable; using a crossfade"))
        .ok();
    renderer
        .egl_context()
        .user_data()
        .insert_if_missing(|| Program(program.clone()));
    program
}

#[derive(Debug)]
pub struct RevealElement<E> {
    inner: E,
    program: GlesTexProgram,
    mask: CircleMask,
    texture_size: Size<i32, Buffer>,
    id: Id,
}
impl<E> RevealElement<E> {
    pub fn new(
        inner: E,
        program: GlesTexProgram,
        mask: CircleMask,
        texture_size: Size<i32, Buffer>,
    ) -> Self {
        Self {
            inner,
            program,
            mask,
            texture_size,
            id: Id::new(),
        }
    }
}
impl<E: Element> Element for RevealElement<E> {
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
    // The circle changes even when the texture's commit does not. A fresh
    // element id damages this layer, and no opaque region can hide the old theme.
    // No underlying_storage: KMS must never scan out the unclipped texture.
}
impl<E: RenderElement<GlesRenderer>> RenderElement<GlesRenderer> for RevealElement<E> {
    fn draw(
        &self,
        frame: &mut GlesFrame<'_, '_>,
        src: Rectangle<f64, Buffer>,
        dst: Rectangle<i32, Physical>,
        damage: &[Rectangle<i32, Physical>],
        _opaque_regions: &[Rectangle<i32, Physical>],
    ) -> Result<(), GlesError> {
        let [sx, sy, ox, oy] = screen_mapping(self.texture_size, src, dst);
        frame.override_default_tex_program(
            self.program.clone(),
            vec![
                Uniform::new("screen_mapping", (sx, sy, ox, oy)),
                Uniform::new("circle", (self.mask.cx, self.mask.cy, self.mask.radius)),
            ],
        );
        let result = self.inner.draw(frame, src, dst, damage, &[]);
        frame.clear_tex_program_override();
        result
    }
}

/// Map texture coordinates back to the displayed pixels. Derive this from the
/// actual draw rectangles so output clipping, panel scaling and window growth
/// all keep the reveal circle anchored to the same screen position.
fn screen_mapping(
    texture: Size<i32, Buffer>,
    src: Rectangle<f64, Buffer>,
    dst: Rectangle<i32, Physical>,
) -> [f32; 4] {
    let sx = f64::from(dst.size.w) / src.size.w;
    let sy = f64::from(dst.size.h) / src.size.h;
    [
        (f64::from(texture.w) * sx) as f32,
        (f64::from(texture.h) * sy) as f32,
        (f64::from(dst.loc.x) - src.loc.x * sx) as f32,
        (f64::from(dst.loc.y) - src.loc.y * sy) as f32,
    ]
}

#[cfg(test)]
mod tests;
