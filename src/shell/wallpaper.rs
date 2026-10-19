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
        let design = crate::config::design();
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
            if theme.is_dark {
                design.wallpaper.primary_mix_dark
            } else {
                design.wallpaper.primary_mix_light
            },
        );
        s.vertical_gradient(
            0.0,
            0.0,
            w as f32,
            h as f32,
            base,
            Theme::mix(
                theme.background,
                theme.tertiary_container,
                design.wallpaper.tertiary_mix,
            ),
        );
        // Broad, soft organic forms echo Android's tonal wallpapers. The same
        // dynamic color roles keep custom wallpaper seeds and variants useful.
        let (wf, hf) = (w as f32, h as f32);
        let mut petal = PathBuilder::new();
        let p = design.wallpaper.petal;
        petal.move_to(p[0] * wf, p[1] * hf);
        petal.cubic_to(
            p[2] * wf,
            p[3] * hf,
            p[4] * wf,
            p[5] * hf,
            p[6] * wf,
            p[7] * hf,
        );
        petal.cubic_to(
            p[8] * wf,
            p[9] * hf,
            p[10] * wf,
            p[11] * hf,
            p[12] * wf,
            p[13] * hf,
        );
        petal.close();
        s.fill_path(
            &petal.finish().expect("wallpaper petal"),
            Theme::mix(
                base,
                theme.secondary_container,
                if theme.is_dark {
                    design.wallpaper.petal_mix_dark
                } else {
                    design.wallpaper.petal_mix_light
                },
            ),
        );
        for (g, color) in [
            (design.wallpaper.glow_one, theme.tertiary_container),
            (design.wallpaper.glow_two, theme.primary_container),
        ] {
            s.radial_glow(
                wf * g[0],
                hf * g[1],
                hf * g[2],
                Theme::with_alpha(color, g[3]),
                Theme::with_alpha(color, 0.0),
            );
        }
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
