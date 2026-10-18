use std::collections::HashMap;
use std::path::{Path, PathBuf};

use fontdue::{Font, FontSettings};

use crate::style::TextStyle;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum FontKind {
    Regular,
    Medium,
    Bold,
    Icon,
}

struct Glyph {
    width: usize,
    height: usize,
    xmin: i32,
    ymin: i32,
    advance: f32,
    coverage: Vec<u8>,
}

pub struct Fonts {
    regular: Font,
    medium: Font,
    bold: Font,
    icons: Font,
    icon_codepoints: HashMap<String, char>,
    cache: HashMap<(FontKind, char, u16), Glyph>,
}

fn font_search_roots() -> Vec<PathBuf> {
    let mut roots = Vec::new();
    if let Some(dir) = std::env::var_os("MATERIALAND_FONT_DIR") {
        roots.push(PathBuf::from(dir));
    }
    roots.push(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets/fonts"));
    roots.push(PathBuf::from("assets/fonts"));
    if let Some(home) = std::env::var_os("HOME") {
        let home = PathBuf::from(home);
        roots.push(home.join(".local/share/fonts"));
        roots.push(home.join(".fonts"));
    }
    roots.extend([
        PathBuf::from("/usr/share/fonts/truetype/roboto"),
        PathBuf::from("/usr/share/fonts/truetype/google-roboto"),
        PathBuf::from("/usr/share/fonts/opentype/roboto"),
        PathBuf::from("/usr/share/fonts/truetype/noto"),
        PathBuf::from("/usr/share/fonts/truetype/dejavu"),
        PathBuf::from("/usr/share/fonts/truetype/liberation"),
        PathBuf::from("/usr/share/fonts/fonts-go"),
        PathBuf::from("/usr/share/fonts"),
        PathBuf::from("/usr/local/share/fonts"),
    ]);
    roots.dedup();
    roots
}

fn find_file(names: &[&str]) -> Option<PathBuf> {
    let roots = font_search_roots();
    // Fast path for the customary font directory layout.
    for root in &roots {
        for name in names {
            let candidate = root.join(name);
            if candidate.is_file() {
                return Some(candidate);
            }
        }
    }

    // Distros commonly install Roboto in a versioned or nested subdirectory.
    // Search only font directories and cap depth so startup remains predictable.
    fn walk(dir: &Path, wanted: &[&str], depth: usize) -> Option<PathBuf> {
        if depth > 5 {
            return None;
        }
        let entries = std::fs::read_dir(dir).ok()?;
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() {
                let file_name = path.file_name()?.to_string_lossy();
                if wanted.iter().any(|n| file_name.eq_ignore_ascii_case(n)) {
                    return Some(path);
                }
            } else if path.is_dir() {
                if let Some(found) = walk(&path, wanted, depth + 1) {
                    return Some(found);
                }
            }
        }
        None
    }
    for root in roots {
        if root.is_dir() {
            if let Some(found) = walk(&root, names, 0) {
                return Some(found);
            }
        }
    }
    None
}

fn load_font(path: &Path) -> Option<Font> {
    let bytes = std::fs::read(path).ok()?;
    Font::from_bytes(bytes, FontSettings::default()).ok()
}

fn fallback_icons() -> HashMap<String, char> {
    // Readable Unicode glyph fallbacks for systems without Google's optional
    // Material Icons font. Installing MaterialIcons-Regular.ttf plus its
    // `codepoints` map switches to the canonical Material glyphs automatically.
    [
        ("apps", '▦'),
        ("terminal", '›'),
        ("public", '◎'),
        ("code", '〈'),
        ("music_note", '♫'),
        ("photo", '▧'),
        ("movie", '▶'),
        ("settings", '⚙'),
        ("folder", '▱'),
        ("mail", '✉'),
        ("calculate", '∑'),
        ("sports_esports", '♧'),
        ("monitor", '▣'),
        ("description", '▤'),
        ("calendar_month", '▦'),
        ("chat", '▰'),
        ("search", '⌕'),
        ("account_circle", '☺'),
        ("power_settings_new", '⏻'),
        ("restart_alt", '↻'),
        ("logout", '→'),
        ("light_mode", '☼'),
        ("dark_mode", '☾'),
        ("brightness_6", '☼'),
        ("nightlight", '☾'),
        ("reduce_motion", '≋'),
        ("remove", '−'),
        ("crop_square", '□'),
        ("filter_none", '▢'),
        ("close", '×'),
    ]
    .into_iter()
    .map(|(name, glyph)| (name.to_string(), glyph))
    .collect()
}

fn parse_codepoints(path: &Path) -> HashMap<String, char> {
    let mut map = HashMap::new();
    let Ok(text) = std::fs::read_to_string(path) else {
        return map;
    };
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let mut parts = line.split_whitespace();
        if let (Some(name), Some(cp)) = (parts.next(), parts.next()) {
            let cp = cp.trim_start_matches("U+");
            if let Ok(cp) = u32::from_str_radix(cp, 16) {
                if let Some(ch) = char::from_u32(cp) {
                    map.insert(name.to_string(), ch);
                }
            }
        }
    }
    map
}

impl Fonts {
    pub fn load() -> Result<Self, std::io::Error> {
        let regular_path = find_file(&[
            "Roboto-Regular.ttf",
            "NotoSans-Regular.ttf",
            "DejaVuSans.ttf",
            "LiberationSans-Regular.ttf",
            "Go-Regular.ttf",
        ])
        .ok_or_else(|| std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "Materialand needs a system font; install Roboto, Noto Sans or DejaVu Sans",
        ))?;
        let regular = load_font(&regular_path).ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("could not read or parse font {}", regular_path.display()),
            )
        })?;

        let medium = find_file(&[
            "Roboto-Medium.ttf",
            "NotoSans-Medium.ttf",
            "DejaVuSans.ttf",
            "LiberationSans-Regular.ttf",
            "Go-Medium.ttf",
        ])
        .and_then(|path| load_font(&path))
        .or_else(|| load_font(&regular_path))
        .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::InvalidData, "could not reload fallback font"))?;

        let bold = find_file(&[
            "Roboto-Bold.ttf",
            "NotoSans-Bold.ttf",
            "DejaVuSans-Bold.ttf",
            "LiberationSans-Bold.ttf",
            "Go-Bold.ttf",
        ])
        .and_then(|path| load_font(&path))
        .or_else(|| load_font(&regular_path))
        .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::InvalidData, "could not reload fallback font"))?;

        let mut icon_codepoints = fallback_icons();
        let mut icons = load_font(&regular_path).ok_or_else(|| {
            std::io::Error::new(std::io::ErrorKind::InvalidData, "could not reload icon fallback font")
        })?;
        if let Some(icon_path) = find_file(&["MaterialIcons-Regular.ttf"]) {
            let codepoints_path = icon_path
                .parent()
                .map(|dir| dir.join("codepoints"))
                .filter(|path| path.is_file());
            if let (Some(codepoints_path), Some(icon_font)) =
                (codepoints_path, load_font(&icon_path))
            {
                let canonical = parse_codepoints(&codepoints_path);
                if !canonical.is_empty() {
                    icon_codepoints = canonical;
                    icons = icon_font;
                }
            }
        }

        Ok(Self {
            regular,
            medium,
            bold,
            icons,
            icon_codepoints,
            cache: HashMap::new(),
        })
    }

    pub fn icon(&self, name: &str) -> Option<char> {
        self.icon_codepoints.get(name).copied()
    }

    fn font(&self, kind: FontKind) -> &Font {
        match kind {
            FontKind::Regular => &self.regular,
            FontKind::Medium => &self.medium,
            FontKind::Bold => &self.bold,
            FontKind::Icon => &self.icons,
        }
    }

    fn rasterize(&mut self, kind: FontKind, ch: char, px: u16) -> &Glyph {
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

    /// Returns `(ascent, descent)`; `descent` is negative, as in fontdue.
    fn line_metrics(&self, kind: FontKind, px: f32) -> (f32, f32) {
        match self.font(kind).horizontal_line_metrics(px) {
            Some(m) => (m.ascent, m.descent),
            None => (px * 0.8, -px * 0.2),
        }
    }

    pub fn ascent(&self, kind: FontKind, px: f32) -> f32 {
        self.line_metrics(kind, px).0
    }

    /// Width in px of a single line of text with M3 letter-spacing applied.
    pub fn text_width_styled(&mut self, style: TextStyle, text: &str) -> f32 {
        self.text_width_tracked(style.kind, text, style.size, style.tracking)
    }

    fn text_width_tracked(&mut self, kind: FontKind, text: &str, px: f32, tracking: f32) -> f32 {
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

    /// Draw a single M3 type role onto a surface; `(x, y)` is the top-left of
    /// the *line box*, so callers centre vertically by subtracting
    /// `style.line_height / 2`.
    pub fn draw_text_role(
        &mut self,
        surface: &mut crate::render::Surface,
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

    /// Draw one line of text, applying `tracking` px of letter spacing between
    /// glyphs. `(x, y)` is the top-left of the line box.
    #[allow(clippy::too_many_arguments)]
    fn draw_text_styled_at(
        &mut self,
        surface: &mut crate::render::Surface,
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

    /// Draw an icon whose glyph is visually centred on `(cx, cy)`.
    ///
    /// The *line box* top-left is placed directly, which would leave a glyph a
    /// few pixels above the optical centre; the bitmap metrics from fontdue are
    /// used here to place the ink box exactly in the middle.
    pub fn draw_icon_centered(
        &mut self,
        surface: &mut crate::render::Surface,
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
            (g.xmin as f32, g.ymin as f32, g.width as f32, g.height as f32)
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

#[allow(clippy::too_many_arguments)]
fn blend_coverage(
    target: &mut [u8],
    target_w: u32,
    target_h: u32,
    gx: i32,
    gy: i32,
    gw: i32,
    gh: i32,
    coverage: &[u8],
    color: [f32; 4],
) {
    let (sr, sg, sb, sa0) = (color[0], color[1], color[2], color[3]);
    for row in 0..gh {
        let ty = gy + row;
        if ty < 0 || ty >= target_h as i32 {
            continue;
        }
        for col in 0..gw {
            let tx = gx + col;
            if tx < 0 || tx >= target_w as i32 {
                continue;
            }
            let a = coverage[(row * gw + col) as usize] as f32 / 255.0;
            if a <= 0.0 {
                continue;
            }
            let sa = (sa0 * a).clamp(0.0, 1.0);
            let idx = ((ty * target_w as i32 + tx) * 4) as usize;
            let dr = target[idx] as f32 / 255.0;
            let dg = target[idx + 1] as f32 / 255.0;
            let db = target[idx + 2] as f32 / 255.0;
            let da = target[idx + 3] as f32 / 255.0;
            // premultiplied "source over"
            let out_r = sr * sa + dr * (1.0 - sa);
            let out_g = sg * sa + dg * (1.0 - sa);
            let out_b = sb * sa + db * (1.0 - sa);
            let out_a = sa + da * (1.0 - sa);
            target[idx] = (out_r.clamp(0.0, 1.0) * 255.0).round() as u8;
            target[idx + 1] = (out_g.clamp(0.0, 1.0) * 255.0).round() as u8;
            target[idx + 2] = (out_b.clamp(0.0, 1.0) * 255.0).round() as u8;
            target[idx + 3] = (out_a.clamp(0.0, 1.0) * 255.0).round() as u8;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render::Surface;

    fn ink_rows(s: &Surface) -> Option<(u32, u32)> {
        let (mut top, mut bottom, mut any) = (u32::MAX, 0u32, false);
        for y in 0..s.pixmap.height() {
            for x in 0..s.width() {
                if s.pixmap.pixel(x, y).unwrap().alpha() > 20 {
                    top = top.min(y);
                    bottom = bottom.max(y);
                    any = true;
                }
            }
        }
        any.then_some((top, bottom))
    }

    fn ink_bounds(s: &Surface) -> Option<(u32, u32, u32, u32)> {
        let (mut x0, mut y0, mut x1, mut y1, mut any) =
            (u32::MAX, u32::MAX, 0u32, 0u32, false);
        for y in 0..s.pixmap.height() {
            for x in 0..s.width() {
                if s.pixmap.pixel(x, y).unwrap().alpha() > 20 {
                    x0 = x0.min(x);
                    y0 = y0.min(y);
                    x1 = x1.max(x);
                    y1 = y1.max(y);
                    any = true;
                }
            }
        }
        any.then_some((x0, y0, x1, y1))
    }

    #[test]
    fn glyph_sits_above_baseline() {
        let mut f = Fonts::load().expect("test system font");
        let mut s = Surface::new(140, 96);
        let (px, y) = (48.0, 20.0);
        f.draw_text_styled_at(
            &mut s,
            10.0,
            y,
            FontKind::Bold,
            "H",
            px,
            0.0,
            tiny_skia::Color::from_rgba8(255, 255, 255, 255),
        );
        let baseline = (y + f.ascent(FontKind::Bold, px)).round() as i32;
        let (top, bottom) = ink_rows(&s).expect("glyph produced ink");
        assert!(
            (bottom as i32) < baseline + 2,
            "bottom {bottom} must sit on the baseline {baseline}"
        );
        assert!((top as i32) < baseline, "glyph must rise above the baseline");
        assert!(
            ((top + bottom) as i32) / 2 < baseline,
            "glyph center must be above the baseline"
        );
    }

    #[test]
    fn icon_names_resolve() {
        let f = Fonts::load().expect("test system font");
        for name in ["terminal", "public", "settings", "folder", "apps"] {
            assert!(f.icon(name).is_some(), "missing icon: {name}");
        }
    }

    #[test]
    fn centered_icon_ink_is_centered() {
        let mut f = Fonts::load().expect("test system font");
        let (cx, cy) = (60.0, 40.0);
        let mut s = Surface::new(120, 80);
        f.draw_icon_centered(
            &mut s,
            cx,
            cy,
            "apps",
            32.0,
            tiny_skia::Color::from_rgba8(255, 255, 255, 255),
        );
        let (x0, y0, x1, y1) = ink_bounds(&s).expect("icon produced ink");
        let mid_x = (x0 + x1) as f32 / 2.0;
        let mid_y = (y0 + y1) as f32 / 2.0;
        assert!((mid_x - cx).abs() <= 2.0, "x center {mid_x} != {cx}");
        assert!((mid_y - cy).abs() <= 2.0, "y center {mid_y} != {cy}");
    }
}