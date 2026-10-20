//! Compositing.
use super::*;

impl Surface {
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

    pub fn masked_path(&self, path: &Path) -> Surface {
        let mut pixmap = self.pixmap.clone();
        if let Some(mut mask) = Mask::new(pixmap.width(), pixmap.height()) {
            mask.fill_path(path, FillRule::Winding, true, SkTransform::identity());
            pixmap.apply_mask(&mask);
        }
        Self { pixmap }
    }

    pub fn blit_intersection(&mut self, src: &Surface, clips: &[&Path]) {
        let Some(mut mask) = Mask::new(self.pixmap.width(), self.pixmap.height()) else {
            return;
        };
        for (i, path) in clips.iter().enumerate() {
            if i == 0 {
                mask.fill_path(path, FillRule::Winding, true, SkTransform::identity());
            } else {
                mask.intersect_path(path, FillRule::Winding, true, SkTransform::identity());
            }
        }
        self.pixmap.draw_pixmap(
            0,
            0,
            src.pixmap.as_ref(),
            &PixmapPaint::default(),
            SkTransform::identity(),
            Some(&mask),
        );
    }

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
        self.pixmap
            .draw_pixmap(0, 0, src.pixmap.as_ref(), &paint, transform, mask.as_ref());
    }
}
