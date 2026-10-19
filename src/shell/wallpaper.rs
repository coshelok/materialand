//! Wallpaper.
use super::*;

impl Shell {
    pub fn render_wallpaper(
        &mut self,
        theme: &Theme,
        w: u32,
        h: u32,
        mask: Option<CircleMask>,
    ) -> (MemoryRenderBuffer, Point<f64, Logical>) {
        // Round the requested size up to a coarse grid. The compositor may
        // report tiny size jitter (e.g. 940x1010 <-> 931x1006) while otherwise
        // idle; regenerating the (expensive) wallpaper for a few pixels of
        // difference would burn the CPU. The buffer is allowed to overhang the
        // output, so rounding up is safe.
        const GRID: u32 = 64;
        let w = w.div_ceil(GRID) * GRID;
        let h = h.div_ceil(GRID) * GRID;
        let key = theme.cache_key();
        let loc = Point::from((0.0, 0.0));
        if let Some(entry) = self
            .wallpapers
            .iter()
            .find(|e| e.w == w && e.h == h && e.key == key)
        {
            return masked_from(&entry.surface, mask, &entry.buffer, loc);
        }
        let mut s = Surface::new(w, h);
        s.clear(theme.background);
        let base = Theme::mix(
            theme.background,
            theme.primary_container,
            if theme.is_dark { 0.18 } else { 0.32 },
        );
        s.vertical_gradient(
            0.0,
            0.0,
            w as f32,
            h as f32,
            base,
            Theme::mix(theme.background, theme.tertiary_container, 0.22),
        );
        // Broad, soft organic forms echo Android's tonal wallpapers. The same
        // dynamic color roles keep custom wallpaper seeds and variants useful.
        let (wf, hf) = (w as f32, h as f32);
        let mut petal = PathBuilder::new();
        petal.move_to(-0.1 * wf, 0.85 * hf);
        petal.cubic_to(
            0.18 * wf,
            0.98 * hf,
            0.22 * wf,
            0.05 * hf,
            0.60 * wf,
            0.28 * hf,
        );
        petal.cubic_to(
            0.92 * wf,
            0.47 * hf,
            0.67 * wf,
            1.12 * hf,
            0.35 * wf,
            1.18 * hf,
        );
        petal.close();
        s.fill_path(
            &petal.finish().expect("wallpaper petal"),
            Theme::mix(
                base,
                theme.secondary_container,
                if theme.is_dark { 0.36 } else { 0.65 },
            ),
        );
        s.radial_glow(
            wf * 0.14,
            hf * 0.20,
            hf * 0.85,
            Theme::with_alpha(theme.tertiary_container, 0.35),
            Theme::with_alpha(theme.tertiary_container, 0.0),
        );
        s.radial_glow(
            wf * 0.85,
            hf * 0.78,
            hf * 0.75,
            Theme::with_alpha(theme.primary_container, 0.45),
            Theme::with_alpha(theme.primary_container, 0.0),
        );
        let buffer = s.to_memory_buffer();
        self.wallpapers.insert(
            0,
            WallpaperEntry {
                w,
                h,
                key,
                surface: s,
                buffer: buffer.clone(),
            },
        );
        self.wallpapers.truncate(3);
        (buffer, loc)
    }
}
