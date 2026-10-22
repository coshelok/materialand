//! Text measurement and role/icon placement.
use super::*;
impl Fonts {
    /// Center visible ink, including glyph bearings; line-height is not an ink box.
    pub fn draw_text_centered(
        &mut self,
        surface: &mut crate::graphics::canvas::Surface,
        center: (f32, f32),
        style: TextStyle,
        text: &str,
        color: tiny_skia::Color,
    ) {
        let mut bounds: Option<(f32, f32, f32, f32)> = None;
        let mut cursor = 0.0;
        for ch in text.chars() {
            let g = self.rasterize(style.kind, ch, style.size.max(1.0) as u16);
            if g.width > 0 && g.height > 0 {
                let left = cursor + g.xmin as f32;
                let top = -(g.ymin as f32) - g.height as f32;
                let right = left + g.width as f32;
                let bottom = -(g.ymin as f32);
                bounds = Some(bounds.map_or((left, top, right, bottom), |b| {
                    (b.0.min(left), b.1.min(top), b.2.max(right), b.3.max(bottom))
                }));
            }
            cursor += g.advance + style.tracking;
        }
        if let Some((left, top, right, bottom)) = bounds {
            let ascent = self.ascent(style.kind, style.size);
            self.draw_text_role(
                surface,
                center.0 - (left + right) / 2.0,
                center.1 - (top + bottom) / 2.0 - ascent,
                style,
                text,
                color,
            );
        }
    }
    pub fn text_width_styled(&mut self, style: TextStyle, text: &str) -> f32 {
        self.text_width_tracked(style.kind, text, style.size, style.tracking)
    }

    pub(super) fn text_width_tracked(
        &mut self,
        kind: FontKind,
        text: &str,
        px: f32,
        tracking: f32,
    ) -> f32 {
        let pxi = px.max(1.0) as u16;
        let mut width = 0.0f32;
        let mut glyphs = 0u32;
        for ch in text.chars() {
            width += self.rasterize(kind, ch, pxi).advance;
            glyphs += 1;
        }
        if glyphs > 1 {
            width += tracking * (glyphs - 1) as f32;
        }
        width
    }

    pub fn draw_text_role(
        &mut self,
        surface: &mut crate::graphics::canvas::Surface,
        x: f32,
        y: f32,
        style: TextStyle,
        text: &str,
        color: tiny_skia::Color,
    ) {
        self.draw_text_styled_at(
            surface,
            x,
            y,
            style.kind,
            text,
            style.size,
            style.tracking,
            color,
        );
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn draw_text_styled_at(
        &mut self,
        surface: &mut crate::graphics::canvas::Surface,
        x: f32,
        y: f32,
        kind: FontKind,
        text: &str,
        px: f32,
        tracking: f32,
        color: tiny_skia::Color,
    ) {
        let color = [color.red(), color.green(), color.blue(), color.alpha()];
        let target_w = surface.width();
        let data = surface.data_mut();
        let target_h = (data.len() / 4) as u32 / target_w.max(1);
        let pxi = px.max(1.0) as u16;
        let baseline = (y + self.ascent(kind, px)).round() as i32;
        let mut cursor = x;
        for ch in text.chars() {
            let g = self.rasterize(kind, ch, pxi);
            // fontdue's `ymin` is the *bottom* edge relative to the baseline,
            // +y upwards, so the bitmap's top row is `baseline - ymin - height`.
            let gx = cursor.round() as i32 + g.xmin;
            let gy = baseline - g.ymin - g.height as i32;
            let (adv, gw, gh) = (g.advance, g.width as i32, g.height as i32);
            blend_coverage(data, target_w, target_h, gx, gy, gw, gh, &g.coverage, color);
            cursor += adv + tracking;
        }
    }

    pub fn draw_icon_centered(
        &mut self,
        surface: &mut crate::graphics::canvas::Surface,
        cx: f32,
        cy: f32,
        name: &str,
        px: f32,
        color: tiny_skia::Color,
    ) {
        let Some(ch) = self.icon(name).or_else(|| self.icon("apps")) else {
            return;
        };
        let pxi = px.max(1.0) as u16;
        let (xmin, ymin, gw, gh) = {
            let g = self.rasterize(FontKind::Icon, ch, pxi);
            (
                g.xmin as f32,
                g.ymin as f32,
                g.width as f32,
                g.height as f32,
            )
        };
        if gw <= 0.0 || gh <= 0.0 {
            return;
        }
        // The bitmap is painted at `x + xmin` (left) and `y + ascent - ymin -
        // height` (top), so solving for a centred ink box gives the offsets below.
        let ascent = self.ascent(FontKind::Icon, px);
        let x = cx - xmin - gw / 2.0;
        let y = cy + ymin + gh / 2.0 - ascent;
        let text = ch.to_string();
        self.draw_text_styled_at(surface, x, y, FontKind::Icon, &text, px, 0.0, color);
    }
}
