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

impl Surface {
    pub fn new(width: u32, height: u32) -> Self {
        let pixmap = Pixmap::new(width.max(1), height.max(1)).expect("allocate surface");
        Self { pixmap }
    }

    pub fn width(&self) -> u32 {
        self.pixmap.width()
    }

    /// A copy of this surface. Used to stamp animated overlays (press
    /// feedback, ripples) onto an otherwise cached layer without re-rastering
    /// the whole layer.
    pub fn duplicate(&self) -> Self {
        Self {
            pixmap: self.pixmap.clone(),
        }
    }

    pub fn clear(&mut self, color: Color) {
        self.pixmap.fill(color);
    }

    pub fn fill_round_rect(&mut self, x: f32, y: f32, w: f32, h: f32, radius: f32, color: Color) {
        if w <= 0.0 || h <= 0.0 {
            return;
        }
        let mut paint = Paint::default();
        paint.set_color(color);
        paint.anti_alias = true;
        let path = round_rect_path(x, y, w, h, radius);
        self.pixmap
            .fill_path(&path, &paint, FillRule::Winding, SkTransform::identity(), None);
    }

    #[allow(clippy::too_many_arguments)]
    pub fn stroke_round_rect(
        &mut self,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        radius: f32,
        width: f32,
        color: Color,
    ) {
        if w <= 0.0 || h <= 0.0 {
            return;
        }
        let mut paint = Paint::default();
        paint.set_color(color);
        paint.anti_alias = true;
        let path = round_rect_path(x, y, w, h, radius);
        let stroke = Stroke {
            width,
            ..Stroke::default()
        };
        self.pixmap
            .stroke_path(&path, &paint, &stroke, SkTransform::identity(), None);
    }

    pub fn circle(&mut self, cx: f32, cy: f32, r: f32, color: Color) {
        if r <= 0.0 {
            return;
        }
        let mut paint = Paint::default();
        paint.set_color(color);
        paint.anti_alias = true;
        let mut pb = PathBuilder::new();
        pb.push_circle(cx, cy, r);
        if let Some(path) = pb.finish() {
            self.pixmap
                .fill_path(&path, &paint, FillRule::Winding, SkTransform::identity(), None);
        }
    }

    /// Fill an arbitrary path.
    pub fn fill_path(&mut self, path: &Path, color: Color) {
        let mut paint = Paint::default();
        paint.set_color(color);
        paint.anti_alias = true;
        self.pixmap
            .fill_path(path, &paint, FillRule::Winding, SkTransform::identity(), None);
    }

    /// Stroke an arbitrary path with the given width.
    pub fn stroke_path(&mut self, path: &Path, width: f32, color: Color) {
        let mut paint = Paint::default();
        paint.set_color(color);
        paint.anti_alias = true;
        let stroke = Stroke {
            width,
            ..Stroke::default()
        };
        self.pixmap
            .stroke_path(path, &paint, &stroke, SkTransform::identity(), None);
    }

    pub fn vertical_gradient(&mut self, x: f32, y: f32, w: f32, h: f32, from: Color, to: Color) {
        let stops = vec![GradientStop::new(0.0, from), GradientStop::new(1.0, to)];
        if let Some(shader) = LinearGradient::new(
            tiny_skia::Point::from_xy(x, y),
            tiny_skia::Point::from_xy(x, y + h),
            stops,
            SpreadMode::Pad,
            SkTransform::identity(),
        ) {
            let paint = Paint {
                shader,
                anti_alias: true,
                ..Default::default()
            };
            self.pixmap
                .fill_rect(rect(x, y, w, h), &paint, SkTransform::identity(), None);
        }
    }

    pub fn radial_glow(&mut self, cx: f32, cy: f32, radius: f32, inner: Color, outer: Color) {
        let stops = vec![GradientStop::new(0.0, inner), GradientStop::new(1.0, outer)];
        if let Some(shader) = RadialGradient::new(
            tiny_skia::Point::from_xy(cx, cy),
            tiny_skia::Point::from_xy(cx, cy),
            radius.max(1.0),
            stops,
            SpreadMode::Pad,
            SkTransform::identity(),
        ) {
            let paint = Paint {
                shader,
                anti_alias: true,
                ..Default::default()
            };
            self.pixmap.fill_rect(
                rect(cx - radius, cy - radius, radius * 2.0, radius * 2.0),
                &paint,
                SkTransform::identity(),
                None,
            );
        }
    }

    /// Upload the surface into a Smithay render buffer.
    ///
    /// Smithay blends with `(ONE, ONE_MINUS_SRC_ALPHA)` and its shader only
    /// scales by `alpha`, so the texture must hold *premultiplied* RGBA — which
    /// is exactly what a `tiny_skia::Pixmap` stores. `Fourcc::Abgr8888` maps to
    /// a plain `GL_RGBA` swizzle, matching the pixmap's `[R, G, B, A]` bytes.
    pub fn to_memory_buffer(&self) -> MemoryRenderBuffer {
        MemoryRenderBuffer::from_slice(
            self.pixmap.data(),
            Fourcc::Abgr8888,
            (self.pixmap.width() as i32, self.pixmap.height() as i32),
            1,
            Transform::Normal,
            None,
        )
    }

    /// Raw premultiplied bytes, for text blending.
    pub fn data_mut(&mut self) -> &mut [u8] {
        self.pixmap.data_mut()
    }

    // ---- Material 3 motion helpers -------------------------------------

    /// A copy of this surface with everything outside the circle erased.
    ///
    /// Used for the radial theme reveal: the new scheme is drawn masked by a
    /// growing circle over the old one. `tiny_skia` works in premultiplied
    /// space, so scaling every channel by the coverage is a correct
    /// "draw the source at alpha = coverage" operation.
    pub fn masked_circle(&self, cx: f32, cy: f32, radius: f32) -> Surface {
        let mut pb = PathBuilder::new();
        pb.push_circle(cx, cy, radius.max(0.0));
        match pb.finish() {
            Some(path) => self.masked_path(&path),
            None => Self {
                pixmap: self.pixmap.clone(),
            },
        }
    }

    /// A copy of this surface with everything outside the circle erased,
    /// cropped to the circle's bounding box.
    ///
    /// Returns the copy and its top-left offset inside the original surface.
    /// The radial theme reveal runs this for every layer on every frame, so
    /// cropping makes the cost scale with the revealed area rather than with
    /// the full screen.
    pub fn masked_circle_cropped(&self, cx: f32, cy: f32, radius: f32) -> (Surface, (i32, i32)) {
        let r = radius.max(0.0);
        let x0 = (cx - r).floor().max(0.0) as i32;
        let y0 = (cy - r).floor().max(0.0) as i32;
        let x1 = (cx + r).ceil().min(self.width() as f32) as i32;
        let y1 = (cy + r).ceil().min(self.pixmap.height() as f32) as i32;
        let w = (x1 - x0).max(0) as u32;
        let h = (y1 - y0).max(0) as u32;
        if w == 0 || h == 0 {
            // A fully grown circle already covers everything; an empty one
            // shows nothing. Either way a 1x1 transparent surface is enough.
            return (Surface::new(1, 1), (0, 0));
        }
        if w >= self.width() && h >= self.pixmap.height() {
            return (self.masked_circle(cx, cy, r), (0, 0));
        }
        let mut dest = Surface::new(w, h);
        let mut pb = PathBuilder::new();
        pb.push_circle(cx - x0 as f32, cy - y0 as f32, r);
        let path = pb.finish();
        dest.blit(
            self,
            SkTransform::from_translate(-(x0 as f32), -(y0 as f32)),
            1.0,
            path.as_ref(),
        );
        (dest, (x0, y0))
    }

    /// A copy of this surface with everything outside `path` erased.
    pub fn masked_path(&self, path: &Path) -> Surface {
        let mut pixmap = self.pixmap.clone();
        if let Some(mut mask) = Mask::new(pixmap.width(), pixmap.height()) {
            mask.fill_path(path, FillRule::Winding, true, SkTransform::identity());
            pixmap.apply_mask(&mask);
        }
        Self { pixmap }
    }

    /// Composite `src` through an arbitrary transform, optionally clipped to a
    /// destination-space path and faded by `opacity`.
    ///
    /// This is the primitive behind the Material 3 *container transform*: the
    /// final layout is drawn once and then mapped onto an animating rectangle.
    pub fn blit(
        &mut self,
        src: &Surface,
        transform: SkTransform,
        opacity: f32,
        clip: Option<&Path>,
    ) {
        let mask = clip.and_then(|path| {
            let mut mask = Mask::new(self.pixmap.width(), self.pixmap.height())?;
            mask.fill_path(path, FillRule::Winding, true, SkTransform::identity());
            Some(mask)
        });
        let paint = PixmapPaint {
            opacity: opacity.clamp(0.0, 1.0),
            quality: FilterQuality::Bilinear,
            ..PixmapPaint::default()
        };
        self.pixmap.draw_pixmap(
            0,
            0,
            src.pixmap.as_ref(),
            &paint,
            transform,
            mask.as_ref(),
        );
    }

    /// Approximate a soft Material 3 elevation shadow.
    ///
    /// tiny-skia has no blur, so the shadow is faked with a handful of
    /// translucent rounded rectangles that grow outwards while fading. The
    /// layers accumulate towards the surface, which reads as a soft penumbra
    /// once the elevated container itself is drawn on top.
    pub fn shadow_round_rect(
        &mut self,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        radius: f32,
        spread: f32,
    ) {
        if w <= 0.0 || h <= 0.0 || spread <= 0.0 {
            return;
        }
        const LAYERS: u32 = 6;
        for i in (1..=LAYERS).rev() {
            let t = i as f32 / LAYERS as f32; // 1.0 = outermost
            let grow = spread * t;
            let alpha = 0.030 * (1.0 - t) * (1.0 - t) + 0.010;
            let color = Color::from_rgba(0.0, 0.0, 0.0, alpha).expect("valid shadow color");
            self.fill_round_rect(
                x - grow,
                y - grow,
                w + 2.0 * grow,
                h + 2.0 * grow,
                radius + grow,
                color,
            );
        }
    }

    /// Fill `path` clipped to `clip` — a bounded ripple.
    pub fn fill_path_clipped(&mut self, path: &Path, clip: &Path, color: Color) {
        let Some(mut mask) = Mask::new(self.pixmap.width(), self.pixmap.height()) else {
            return;
        };
        mask.fill_path(clip, FillRule::Winding, true, SkTransform::identity());
        let mut paint = Paint::default();
        paint.set_color(color);
        paint.anti_alias = true;
        self.pixmap.fill_path(
            path,
            &paint,
            FillRule::Winding,
            SkTransform::identity(),
            Some(&mask),
        );
    }
}

fn rect(x: f32, y: f32, w: f32, h: f32) -> tiny_skia::Rect {
    tiny_skia::Rect::from_xywh(x, y, w, h)
        .unwrap_or_else(|| tiny_skia::Rect::from_xywh(0.0, 0.0, 1.0, 1.0).unwrap())
}

/// Build a closed polygon path from a list of points.
pub fn polygon_path(points: &[(f32, f32)]) -> Path {
    let mut pb = PathBuilder::new();
    if let Some((&(x0, y0), rest)) = points.split_first() {
        pb.move_to(x0, y0);
        for &(x, y) in rest {
            pb.line_to(x, y);
        }
        pb.close();
    } else {
        pb.move_to(0.0, 0.0);
    }
    pb.finish().unwrap_or_else(|| {
        let mut p = PathBuilder::new();
        p.move_to(0.0, 0.0);
        p.finish().unwrap()
    })
}

pub fn round_rect_path(x: f32, y: f32, w: f32, h: f32, radius: f32) -> Path {
    let r = radius.min(w / 2.0).min(h / 2.0).max(0.0);
    let mut pb = PathBuilder::new();
    if r <= 0.01 {
        pb.push_rect(rect(x, y, w, h));
        return pb.finish().unwrap();
    }
    pb.move_to(x + r, y);
    pb.line_to(x + w - r, y);
    pb.quad_to(x + w, y, x + w, y + r);
    pb.line_to(x + w, y + h - r);
    pb.quad_to(x + w, y + h, x + w - r, y + h);
    pb.line_to(x + r, y + h);
    pb.quad_to(x, y + h, x, y + h - r);
    pb.line_to(x, y + r);
    pb.quad_to(x, y, x + r, y);
    pb.close();
    pb.finish().unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn px(s: &Surface, x: u32, y: u32) -> (u8, u8, u8, u8) {
        let p = s.pixmap.pixel(x, y).unwrap();
        (p.red(), p.green(), p.blue(), p.alpha())
    }

    #[test]
    fn clear_fills_everything() {
        let mut s = Surface::new(4, 4);
        s.clear(Color::from_rgba8(10, 20, 30, 255));
        assert_eq!(px(&s, 0, 0), (10, 20, 30, 255));
        assert_eq!(px(&s, 3, 3), (10, 20, 30, 255));
    }

    #[test]
    fn zero_radius_round_rect_fills_rect() {
        let mut s = Surface::new(10, 10);
        s.fill_round_rect(2.0, 2.0, 4.0, 4.0, 0.0, Color::from_rgba8(255, 0, 0, 255));
        assert_eq!(px(&s, 0, 0).3, 0);
        assert_eq!(px(&s, 3, 3), (255, 0, 0, 255));
    }

    #[test]
    fn round_rect_clips_corner() {
        let mut s = Surface::new(20, 20);
        s.fill_round_rect(0.0, 0.0, 20.0, 20.0, 8.0, Color::from_rgba8(0, 0, 255, 255));
        assert!(px(&s, 0, 0).3 <= 5, "corner must be ~transparent");
        assert_eq!(px(&s, 10, 10).3, 255, "center must be opaque");
    }

    #[test]
    fn memory_buffer_is_premultiplied() {
        let mut s = Surface::new(2, 2);
        s.clear(Color::from_rgba8(255, 0, 0, 128));
        let data = s.pixmap.data();
        assert_eq!(data[3], 128, "alpha stored as-is");
        assert!(
            (data[0] as i32 - 128).abs() <= 1,
            "premultiplied red ~= alpha"
        );
    }

    #[test]
    fn masked_circle_erases_outside() {
        let mut s = Surface::new(20, 20);
        s.clear(Color::from_rgba8(255, 255, 255, 255));
        let masked = s.masked_circle(10.0, 10.0, 4.0);
        assert_eq!(px(&masked, 10, 10).3, 255, "inside the circle stays");
        assert_eq!(px(&masked, 0, 0).3, 0, "outside is erased");
        assert_eq!(px(&masked, 19, 10).3, 0, "outside is erased");
    }

    #[test]
    fn cropped_masked_circle_keeps_its_offset() {
        let mut s = Surface::new(40, 40);
        s.clear(Color::from_rgba8(0, 255, 0, 255));
        let (c, (ox, oy)) = s.masked_circle_cropped(20.0, 20.0, 6.0);
        assert_eq!((ox, oy), (14, 14), "crop origin is the bbox top-left");
        assert_eq!(c.width(), 12);
        assert_eq!(c.pixmap.height(), 12);
        assert_eq!(px(&c, 6, 6), (0, 255, 0, 255), "the centre survives");
        assert_eq!(px(&c, 0, 0).3, 0, "corners of the box are outside the circle");
        // A circle larger than the surface needs no crop at all.
        let (full, off) = s.masked_circle_cropped(20.0, 20.0, 100.0);
        assert_eq!(off, (0, 0));
        assert_eq!(full.width(), 40);
        assert_eq!(px(&full, 39, 39), (0, 255, 0, 255));
        // A zero radius reveals nothing.
        let (empty, _) = s.masked_circle_cropped(5.0, 5.0, 0.0);
        assert_eq!(empty.pixmap.pixel(0, 0).map(|p| p.alpha()), Some(0));
    }

    #[test]
    fn blit_clip_cuts_in_destination_space() {
        let mut src = Surface::new(4, 4);
        src.clear(Color::from_rgba8(255, 0, 0, 255));
        let mut dst = Surface::new(40, 40);
        // Maps the source onto dst x/y 10..18.
        let transform = SkTransform::from_scale(2.0, 2.0).post_translate(10.0, 10.0);
        // The clip stays put in destination space: only the left half survives.
        let clip = round_rect_path(10.0, 10.0, 4.0, 8.0, 1.0);
        dst.blit(&src, transform, 1.0, Some(&clip));
        assert_eq!(px(&dst, 11, 13).3, 255, "inside the clip is painted");
        assert_eq!(px(&dst, 16, 13).3, 0, "the clip cuts in device space");
    }

    #[test]
    fn shadow_fades_outward_and_stops() {
        let mut s = Surface::new(120, 120);
        s.shadow_round_rect(40.0, 40.0, 40.0, 40.0, 4.0, 16.0);
        let near = px(&s, 60, 39).3;
        let mid = px(&s, 60, 30).3;
        let far = px(&s, 60, 27).3;
        let outside = px(&s, 60, 22).3;
        assert!(near > 0, "shadow must exist next to the surface");
        assert!(near > mid, "shadow must fade with distance");
        assert!(mid > far, "shadow must fade with distance");
        assert_eq!(outside, 0, "shadow must not extend past the spread");
        // The layers also cover the container itself; the elevated container is
        // drawn on top of them by the caller, which hides that part.
        assert!(px(&s, 60, 60).3 > 0, "the covered interior is painted too");
    }

    #[test]
    fn clipped_fill_respects_clip_path() {
        let mut s = Surface::new(20, 20);
        let mut pb = PathBuilder::new();
        pb.push_circle(10.0, 10.0, 8.0);
        let ripple = pb.finish().unwrap();
        let clip = round_rect_path(0.0, 0.0, 12.0, 20.0, 2.0);
        s.fill_path_clipped(&ripple, &clip, Color::from_rgba8(0, 0, 255, 255));
        assert_eq!(px(&s, 8, 10).3, 255, "inside the clip");
        assert_eq!(px(&s, 15, 10).3, 0, "ripple is clipped on the right");
    }
}