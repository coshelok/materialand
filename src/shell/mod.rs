//! Shell model and independently owned visual components.
use crate::{
    config::{self, AppEntry},
    desktop::{ResizeEdges, WindowMeta},
    graphics::canvas::{round_rect_path, Surface},
    graphics::text::{FontKind, Fonts},
    motion::{duration, CubicBezier, Spring, Tween},
    style::{shape, state, TextStyle},
    theme::{ColorVariant, KeyHash, Theme, ThemeSpec, VARIANTS},
};
use smithay::{
    backend::renderer::element::memory::MemoryRenderBuffer,
    desktop::Window,
    utils::{Logical, Point, Rectangle, Size},
};
use std::collections::HashMap;
use tiny_skia::{Color, Path, PathBuilder, Transform as SkTransform};

const DOCK_PAD: f32 = 16.0;
const SEARCH_HEIGHT: f32 = 64.0;
const GRID_GAP: f32 = 8.0;
const GRID_PAD: f32 = 20.0;

/// Horizontal gap between the right-aligned status icons in the top bar.
const TOPBAR_ICON_GAP: f32 = 24.0;

/// Margin baked into every shadowed buffer so the soft elevation shadow fits.
pub const SHADOW_PAD: f32 = 16.0;
/// How far the fake soft shadow extends past the container edge.
const SHADOW_SPREAD: f32 = 12.0;

/// Total height of the quick-settings sheet, including the palette grid.
const QS_SHEET_HEIGHT: f32 = 374.0;
const QS_CHIP_H: f32 = 34.0;
const QS_CHIP_GAP: f32 = 8.0;
const QS_CHIPS_Y: f32 = 240.0;
const QS_CHIP_MARGIN: f32 = 16.0;

/// Dimming range of the brightness control: `1.0` is unmodified output.
pub const BRIGHTNESS_MIN: f32 = 0.2;
pub const BRIGHTNESS_MAX: f32 = 1.0;

pub struct Shell {
    pub apps: Vec<AppEntry>,
    pub pinned: Vec<AppEntry>,
    /// Command that opens a terminal (the pinned one, or a fallback).
    pub terminal: Vec<String>,
    pub launcher_open: bool,
    pub launcher_query: String,
    pub brightness_open: bool,
    pub quick_settings_open: bool,
    /// Session menu (top-bar user button) visibility.
    pub power_open: bool,
    pub night_light_on: bool,
    pub reduce_motion: bool,
    pub clock: String,
    spec: ThemeSpec,
    /// Caches for the static chrome. Rebuilding these with tiny-skia on every
    /// frame dominates the frame cost, so each buffer is keyed by its inputs and
    /// only regenerated when they change. The lists keep a couple of entries so
    /// the radial reveal can show two palettes at once.
    wallpapers: Vec<WallpaperEntry>,
    topbar_cache: Vec<Piece>,
    dock_cache: Vec<Piece>,
    decor_cache: HashMap<Window, Vec<DecorEntry>>,
    /// Cached full-screen content layer of the launcher overlay, keyed by what
    /// it shows; the open/close transform is applied by the GPU.
    launcher_layer: Option<CachedLayer>,
    /// Cached quick-settings sheet content (settled state only).
    qs_layer: Option<CachedLayer>,
    /// Cached brightness-popup content.
    brightness_layer: Option<CachedLayer>,
    /// Cached session-menu content.
    power_layer: Option<CachedLayer>,
    /// 1x1 scrim buffer per scheme, stretched over the output by the GPU.
    scrim_cache: Option<(u64, MemoryRenderBuffer)>,
    frame_epoch: u64,
    cursor_cache: Option<(
        bool,
        smithay::input::pointer::CursorIcon,
        MemoryRenderBuffer,
    )>,

    // ---- Motion ---------------------------------------------------------
    /// Launcher open progress in `[0, 1]`.
    launcher: Spring,
    /// Brightness popup progress in `[0, 1]`.
    brightness: Spring,
    /// Quick-settings sheet progress in `[0, 1]`.
    quick_settings: Spring,
    /// Session-menu progress in `[0, 1]`.
    power: Spring,
    /// Warm display tint progress in `[0, 1]`.
    night_light: Spring,
    /// Light/dark crossfade progress (used for palette switches).
    theme_mix: Tween,
    prev_theme: Option<Theme>,
    /// Radial palette reveal, used for light/dark switches.
    reveal: Option<Reveal>,
    /// M3 switch positions, indexed by quick-settings tile.
    switches: [Spring; 4],
    /// Brightness slider handle growth, `0` idle .. `1` grabbed.
    slider: Spring,
    /// Window entrance springs, `0` (hidden) .. `1` (rest).
    window_in: HashMap<Window, Spring>,
    /// Window exit springs, `0` (rest) .. `1` (gone).
    window_out: HashMap<Window, Spring>,
    /// Per-control press/ripple animation state.
    press: HashMap<PressKey, PressAnim>,
    /// Hovered chrome element, for M3 state layers.
    hover_top: Option<TopAction>,
    hover_dock: Option<usize>,
    hover_power: Option<usize>,
    pub press_top: Option<TopAction>,
    pub press_dock: Option<usize>,
    pub press_tile: Option<QuickAction>,
    pub press_chip: Option<usize>,
    pub press_launch: Option<usize>,
    /// Session-menu row under a swallowed press.
    pub press_power: Option<usize>,
    /// Screen rect the launcher grows out of (dock launcher or top-bar button).
    pub launcher_origin: Option<RectF>,
    /// Screen rect the quick-settings sheet grows out of.
    qs_origin: Option<RectF>,
    /// Palette swatches (primary, secondary, tertiary) per variant, built
    /// lazily for each scheme brightness.
    swatches_dark: Option<[(Color, Color, Color); 9]>,
    swatches_light: Option<[(Color, Color, Color); 9]>,
}

mod brightness;
mod cursor;
mod decoration;
mod dock;
mod feedback;
mod launcher;
mod layout;
mod model;
mod motion;
mod paint;
mod power;
mod quick_settings;
mod topbar;
mod wallpaper;
mod window_motion;
pub use layout::*;
use paint::*;
#[cfg(test)]
mod tests;

mod types;
pub use types::*;

mod buttons;
mod launcher_tile;
