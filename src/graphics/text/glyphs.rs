//! Raster.
use super::*;

impl Fonts {
    pub fn icon(&self, name: &str) -> Option<char> {
        self.icon_codepoints.get(name).copied()
    }

    pub(super) fn font(&self, kind: FontKind) -> &Font {
        match kind {
            FontKind::Regular => &self.regular,
            FontKind::Medium => &self.medium,
            FontKind::Bold => &self.bold,
            FontKind::Icon => &self.icons,
        }
    }

    pub(super) fn rasterize(&mut self, kind: FontKind, ch: char, px: u16) -> &Glyph {
        if !self.cache.contains_key(&(kind, ch, px)) {
            let (metrics, coverage) = self.font(kind).rasterize(ch, px as f32);
            self.cache.insert(
                (kind, ch, px),
                Glyph {
                    width: metrics.width,
                    height: metrics.height,
                    xmin: metrics.xmin,
                    ymin: metrics.ymin,
                    advance: metrics.advance_width,
                    coverage,
                },
            );
        }
        self.cache.get(&(kind, ch, px)).unwrap()
    }

    pub(super) fn line_metrics(&self, kind: FontKind, px: f32) -> (f32, f32) {
        match self.font(kind).horizontal_line_metrics(px) {
            Some(m) => (m.ascent, m.descent),
            None => (px * 0.8, -px * 0.2),
        }
    }

    pub fn ascent(&self, kind: FontKind, px: f32) -> f32 {
        self.line_metrics(kind, px).0
    }
}
