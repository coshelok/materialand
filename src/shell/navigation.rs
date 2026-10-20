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
        d.navigation.height + 2.0 * d.navigation.margin
    } else {
        d.dock.height + d.dock.margin
    }
}
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
pub fn navigation_hit(w: f32, h: f32, pos: Point<f64, Logical>) -> Option<NavAction> {
    let r = navigation_rect(w, h);
    if !tablet_mode() || !r.contains(pos) {
        return None;
    }
    [
        NavAction::Home,
        NavAction::Launcher,
        NavAction::Overview,
        NavAction::Shade,
    ]
    .get((((pos.x as f32 - r.x) / r.w) * 4.0).floor().clamp(0.0, 3.0) as usize)
    .copied()
}
impl Shell {
    pub fn render_navigation(
        &mut self,
        theme: &Theme,
        fonts: &mut Fonts,
        w: f32,
        h: f32,
    ) -> (MemoryRenderBuffer, Point<f64, Logical>) {
        let r = navigation_rect(w, h);
        let pad = floating_dock_padding();
        let loc = ((r.x - pad) as f64, (r.y - pad) as f64).into();
        let key = KeyHash::new()
            .u64(theme.cache_key())
            .u32(r.w.to_bits())
            .u32(r.h.to_bits())
            .finish();
        if let Some(piece) = self.navigation_cache.iter().find(|p| p.key == key) {
            return (piece.buffer.clone(), loc);
        }
        let mut s = floating_dock::surface(theme, r.w, r.h);
        for (i, icon) in [
            "radio_button_unchecked",
            "apps",
            "crop_square",
            "expand_more",
        ]
        .iter()
        .enumerate()
        {
            fonts.draw_icon_centered(
                &mut s,
                pad + r.w * (i as f32 + 0.5) / 4.0,
                pad + r.h / 2.0,
                icon,
                config::design().navigation.icon_size,
                floating_dock::foreground(theme),
            );
        }
        let buffer = s.to_memory_buffer();
        self.navigation_cache.push(Piece {
            key,
            surface: s,
            buffer: buffer.clone(),
        });
        if self.navigation_cache.len() > 2 {
            self.navigation_cache.remove(0);
        }
        (buffer, loc)
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
