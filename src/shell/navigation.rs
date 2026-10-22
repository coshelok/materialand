//! Mouse/keyboard access remains available alongside laptop touchpad gestures.
use super::*;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NavAction {
    Home,
    Launcher,
    Overview,
    Shade,
}
pub fn tablet_mode() -> bool {
    config::design().navigation.mode == "tablet"
}
pub fn top_inset() -> f32 {
    if tablet_mode() {
        config::design().statusbar.height
    } else {
        let d = config::design();
        d.topbar.height + 2.0 * d.topbar.margin
    }
}
pub fn bottom_inset() -> f32 {
    let d = config::design();
    if tablet_mode() {
        0.0
    } else {
        d.dock.height + d.dock.margin
    }
}
// Invisible launch-animation origin retained for desktop-mode compatibility.
pub fn navigation_rect(w: f32, h: f32) -> RectF {
    let d = &config::design().navigation;
    let width = d.width.min((w - 2.0 * d.margin).max(1.0));
    RectF {
        x: (w - width) / 2.0,
        y: (h - d.height - d.margin).max(0.0),
        w: width,
        h: d.height,
    }
}
impl Shell {
    pub fn render_split_handle(&mut self, theme: &Theme) -> MemoryRenderBuffer {
        let d = &config::design().split;
        let key = theme.cache_key();
        if let Some(piece) = self.split_handle.iter().find(|p| p.key == key) {
            return piece.buffer.clone();
        }
        let mut surface = Surface::new(d.handle_width.ceil() as u32, d.handle_height.ceil() as u32);
        surface.fill_round_rect(
            0.0,
            0.0,
            d.handle_width,
            d.handle_height,
            d.handle_width / 2.0,
            theme.primary,
        );
        let buffer = surface.to_memory_buffer();
        self.split_handle.push(Piece {
            key,
            surface,
            buffer: buffer.clone(),
        });
        if self.split_handle.len() > 2 {
            self.split_handle.remove(0);
        }
        buffer
    }
}
