//! One cached floating medallion follows the pointer without repainting its pixels.
use super::*;
impl Shell {
    pub fn render_shortcut_drag(
        &mut self,
        theme: &Theme,
        fonts: &mut Fonts,
        app: &AppEntry,
        pos: Point<f64, Logical>,
    ) -> (MemoryRenderBuffer, Point<f64, Logical>) {
        let size = config::design().launcher.tile_size;
        let key = KeyHash::new()
            .u64(theme.cache_key())
            .str(&app.identity())
            .str(&app.name)
            .finish();
        if self.drag_icon.as_ref().is_none_or(|piece| piece.key != key) {
            let mut s = Surface::new(size.ceil() as u32, size.ceil() as u32);
            self.draw_launcher_tile(&mut s, fonts, theme, app, 0.0, 0.0, 1.0);
            let buffer = s.to_memory_buffer();
            self.drag_icon = Some(Piece {
                key,
                surface: s,
                buffer,
            });
        }
        (
            self.drag_icon.as_ref().unwrap().buffer.clone(),
            (
                pos.x - f64::from(size / 2.0),
                pos.y - f64::from(config::design().apps.icon_center_y),
            )
                .into(),
        )
    }
}
