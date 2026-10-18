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
        s.vertical_gradient(
            0.0,
            0.0,
            w as f32,
            h as f32,
            Theme::with_alpha(theme.primary_container, 0.5),
            theme.background,
        );
        s.radial_glow(
            w as f32 * 0.18,
            h as f32 * 0.12,
            h as f32 * 0.95,
            Theme::with_alpha(theme.tertiary_container, 0.4),
            Theme::with_alpha(theme.tertiary_container, 0.0),
        );
        s.radial_glow(
            w as f32 * 0.85,
            h as f32 * 0.92,
            h as f32 * 0.85,
            Theme::with_alpha(theme.primary, 0.25),
            Theme::with_alpha(theme.primary, 0.0),
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
