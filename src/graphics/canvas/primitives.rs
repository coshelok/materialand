//! Primitives.
use super::*;

impl Surface {
    pub fn fill_round_rect(&mut self, x: f32, y: f32, w: f32, h: f32, radius: f32, color: Color) {
        if w <= 0.0 || h <= 0.0 {
            return;
        }
        let mut paint = Paint::default();
        paint.set_color(color);
        paint.anti_alias = true;
        let path = round_rect_path(x, y, w, h, radius);
        self.pixmap.fill_path(
            &path,
            &paint,
            FillRule::Winding,
            SkTransform::identity(),
            None,
        );
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
            self.pixmap.fill_path(
                &path,
                &paint,
                FillRule::Winding,
                SkTransform::identity(),
                None,
            );
        }
    }

    pub fn fill_path(&mut self, path: &Path, color: Color) {
        let mut paint = Paint::default();
        paint.set_color(color);
        paint.anti_alias = true;
        self.pixmap.fill_path(
            path,
            &paint,
            FillRule::Winding,
            SkTransform::identity(),
            None,
        );
    }

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

    #[allow(clippy::too_many_arguments)]
    pub fn shadow_round_rect(
        &mut self,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        radius: f32,
        spread: f32,
        tint: Color,
    ) {
        let design = crate::config::design();
        if w <= 0.0 || h <= 0.0 || spread <= 0.0 {
            return;
        }
        let layers = design.shadow.layers as u32;
        for i in (1..=layers).rev() {
            let t = i as f32 / layers as f32; // 1.0 = outermost
            let grow = spread * t;
            let alpha =
                design.shadow.inner_alpha * (1.0 - t) * (1.0 - t) + design.shadow.outer_alpha;
            let color =
                Color::from_rgba(tint.red(), tint.green(), tint.blue(), alpha * tint.alpha())
                    .expect("valid shadow color");
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
