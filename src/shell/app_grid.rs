//! Shared app textures and GPU crops for home pages and the app drawer.
use super::*;
use crate::shell::placement::CardPlacement;

impl Shell {
    pub(super) fn app_tile(
        &mut self,
        fonts: &mut Fonts,
        theme: &Theme,
        app: &AppEntry,
        scale: f32,
        pressed: bool,
    ) -> MemoryRenderBuffer {
        let key = KeyHash::new()
            .u64(theme.cache_key())
            .str(&app.name)
            .str(&app.icon)
            .f32(scale)
            .bool(pressed)
            .finish();
        let id = app.identity();
        if let Some(piece) = self
            .app_tiles
            .get(&id)
            .and_then(|v| v.iter().find(|p| p.key == key))
        {
            return piece.buffer.clone();
        }
        let d = config::design();
        let size = d.launcher.tile_size.ceil() as u32;
        let mut surface = Surface::new(size, size);
        if pressed {
            surface.fill_round_rect(
                0.0,
                0.0,
                size as f32,
                size as f32,
                d.shape.large,
                Theme::with_alpha(theme.primary, d.state.pressed),
            );
        }
        self.draw_launcher_tile(&mut surface, fonts, theme, app, 0.0, 0.0, scale);
        let buffer = surface.to_memory_buffer();
        let cache = self.app_tiles.entry(id).or_default();
        cache.push(Piece {
            key,
            surface,
            buffer: buffer.clone(),
        });
        if cache.len() > 2 {
            cache.remove(0);
        }
        buffer
    }
}

impl LayerFrame {
    /// Clip before mapping: source pixels, drawn bounds and input share one rect.
    pub(super) fn clipped(
        buffer: MemoryRenderBuffer,
        tile: RectF,
        viewport: RectF,
        placement: CardPlacement,
    ) -> Option<Self> {
        let x = tile.x.max(viewport.x);
        let y = tile.y.max(viewport.y);
        let right = tile.right().min(viewport.right());
        let bottom = tile.bottom().min(viewport.bottom());
        if right <= x || bottom <= y {
            return None;
        }
        let crop = RectF {
            x,
            y,
            w: right - x,
            h: bottom - y,
        };
        let target = placement.map_rect(crop);
        Some(Self {
            rounded: None,
            buffer,
            source: Some(RectF {
                x: x - tile.x,
                y: y - tile.y,
                w: crop.w,
                h: crop.h,
            }),
            loc: (f64::from(target.x), f64::from(target.y)).into(),
            size: Some(
                (
                    (target.w.round() as i32).max(1),
                    (target.h.round() as i32).max(1),
                )
                    .into(),
            ),
        })
    }
}
