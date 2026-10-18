use smithay::{
    backend::{allocator::Fourcc, renderer::element::memory::MemoryRenderBuffer},
    utils::Transform,
};
use tiny_skia::{
    Color, FillRule, FilterQuality, GradientStop, LinearGradient, Mask, Paint, Path, PathBuilder,
    Pixmap, PixmapPaint, RadialGradient, SpreadMode, Stroke, Transform as SkTransform,
};

/// A tiny-skia drawing surface. Pixel data is premultiplied RGBA.
pub struct Surface {
    pub pixmap: Pixmap,
}

mod compositing;
mod paths;
mod primitives;
mod surface;
pub use paths::*;
#[cfg(test)]
mod tests;
