//! Shell model and independently owned visual components.
use crate::{
    config::{self, AppEntry},
    desktop::{ResizeEdges, WindowMeta},
    graphics::canvas::{round_rect_path, Surface},
    graphics::text::Fonts,
    motion::{CubicBezier, Spring, Tween},
    style::TextStyle,
    theme::{ColorVariant, KeyHash, Theme, ThemeSpec, VARIANTS},
};
use smithay::{
    backend::renderer::element::memory::MemoryRenderBuffer,
    desktop::Window,
    utils::{Logical, Point, Rectangle, Size},
};
use std::collections::HashMap;
use tiny_skia::{Color, Path, PathBuilder, Transform as SkTransform};

/// Hardware backlight range. Keep a small usable floor.
pub const BRIGHTNESS_MIN: f32 = 0.02;
pub const BRIGHTNESS_MAX: f32 = 1.0;

pub struct Shell {
    pub(crate) decoration_controls: decoration_controls::DecorationControls<Window>,
    app_icons: crate::graphics::app_icons::AppIcons,
    pub apps: Vec<AppEntry>,
    overview_cards: HashMap<Window, Vec<Piece>>,
    overview_header: LayerCache,
    pub home: home::Home,
    pub pinned: Vec<AppEntry>,
    pins: config::Pins,
    dock_motion: dock_motion::DockMotion,
    /// Command that opens a terminal (the pinned one, or a fallback).
    pub terminal: Vec<String>,
    pub terminal_app: AppEntry,
    pub app_menu: Option<AppMenu>,
    pub press_pin: bool,
    pub consume_menu_release: bool,
    pub launcher_open: bool,
    pub launcher_query: String,
    pub brightness_open: bool,
    pub system_status: crate::system::Status,
    pub notifications: crate::notifications::Snapshot,
    pub notifications_open: bool,
    statusbar_cache: LayerCache,
    pub shade_scroll: f32,
    shade_scroll_motion: Spring,
    shade_scroll_target: f32,
    pub dragged_identity: Option<String>,
    pub shortcut_error: Option<String>,
    pub appearance_error: Option<String>,
    drag_icon: Option<Piece>,
    split_handle: Vec<Piece>,
    pub connection_page: Option<QuickAction>,
    pub appearance_page: bool,
    pub brightness_available: bool,
    pub brightness_error: bool,
    pub night_light_available: bool,
    pub quick_settings_open: bool,
    /// Session menu (top-bar user button) visibility.
    pub power_open: bool,
    pub night_light_on: bool,
    pub reduce_motion: bool,
    pub clock: String,
    pub calendar: calendar::Calendar,
    calendar_layer: LayerCache,
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
    launcher_layer: LayerCache,
    launcher_scrollbar: LayerCache,
    app_tiles: HashMap<String, Vec<Piece>>,
    /// Cached quick-settings sheet content (settled state only).
    qs_layer: LayerCache,
    shade_backdrop: LayerCache,
    notice_layer: LayerCache,
    notice_scroll: Spring,
    notice_scroll_target: f32,
    /// Cached brightness-popup content.
    brightness_layer: LayerCache,
    /// Cached session-menu content.
    power_layer: LayerCache,
    /// 1x1 scrim buffer per scheme, stretched over the output by the GPU.
    scrim_cache: Option<(u64, MemoryRenderBuffer)>,
    frame_epoch: u64,
    cursor_cache: Option<(
        bool,
        smithay::input::pointer::CursorIcon,
        MemoryRenderBuffer,
    )>,

    // ---- Motion ---------------------------------------------------------
    /// Launcher spatial progress, including a small overshoot.
    launcher: Spring,
    launcher_scroll: Spring,
    launcher_scroll_target: f32,
    /// Brightness popup spatial progress.
    brightness: Spring,
    /// Quick-settings sheet spatial progress.
    quick_settings: Spring,
    /// Session-menu spatial progress.
    power: Spring,
    /// Output white-balance progress in `[0, 1]`.
    night_light: Spring,
    /// Light/dark crossfade progress (used for palette switches).
    theme_mix: Tween,
    prev_theme: Option<Theme>,
    /// Radial palette reveal, used for light/dark switches.
    reveal: Option<Reveal>,
    /// M3 switch positions, indexed by quick-settings tile.
    switches: [Spring; 6],
    /// Brightness slider handle growth, `0` idle .. `1` grabbed.
    slider: Spring,
    /// Window entrance springs, `0` (hidden) .. `1` (rest).
    window_in: HashMap<Window, window_motion::Entrance>,
    launch_origins: window_motion::LaunchOrigins,
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

mod app_identity;
mod app_menu;
mod brightness;
mod cursor;
mod decoration;
mod decoration_controls;
pub use decoration_controls::DecorButton;
mod decoration_frame;
mod dock;
mod floating_dock;
pub use floating_dock::floating_dock_padding;
mod dock_motion;
mod feedback;
mod launcher;
mod launcher_scroll;
mod layout;
mod model;
mod pinning;
pub use app_menu::{app_menu_button, app_menu_home_button, app_menu_rect, AppMenu};
mod connections;
mod motion;
mod paint;
mod power;
mod quick_settings;
mod shade_frame;
mod topbar;
mod wallpaper;
mod window_motion;
pub use layout::*;
use paint::*;
pub use window_motion::WindowPose;
#[cfg(test)]
mod tests;

mod types;
pub use types::*;

mod app_grid;
mod buttons;
mod launcher_tile;
mod placement;

mod calendar;
pub use calendar::{calendar_hit, calendar_rect, CalendarAction};

mod home;
pub use home::{home_layout, HomeHit};

pub(crate) mod android_shade;
pub use android_shade::{shade_brightness_hit, shade_brightness_value, shade_page_button};
mod navigation;
pub use navigation::*;

mod overview;

mod shortcut_drag;

#[cfg(test)]
mod previews;

mod statusbar;
pub use statusbar::*;
mod notifications;
pub use notifications::NotificationHit;
