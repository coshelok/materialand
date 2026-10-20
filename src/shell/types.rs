//! Shell actions, layer frames and private cache records.
use super::*;
/// A floating-point rectangle.
#[derive(Clone, Copy, Debug)]
pub struct RectF {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

impl RectF {
    pub fn right(&self) -> f32 {
        self.x + self.w
    }
    pub fn bottom(&self) -> f32 {
        self.y + self.h
    }
    pub fn contains(&self, p: Point<f64, Logical>) -> bool {
        let (px, py) = (p.x as f32, p.y as f32);
        px >= self.x && px <= self.right() && py >= self.y && py <= self.bottom()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TopAction {
    Home,
    Clock,
    Launcher,
    Brightness,
    Theme,
    QuickSettings,
    Power,
}

/// One row of the session menu that opens from the top-bar user button.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PowerAction {
    PowerOff,
    Restart,
    Logout,
}

/// Session-menu rows, top to bottom.
pub const POWER_ACTIONS: [PowerAction; 3] = [
    PowerAction::PowerOff,
    PowerAction::Restart,
    PowerAction::Logout,
];

impl PowerAction {
    pub(super) fn label(self) -> &'static str {
        match self {
            PowerAction::PowerOff => "Power off",
            PowerAction::Restart => "Restart",
            PowerAction::Logout => "Log out",
        }
    }

    pub(super) fn icon(self) -> &'static str {
        match self {
            PowerAction::PowerOff => "power_settings_new",
            PowerAction::Restart => "restart_alt",
            PowerAction::Logout => "logout",
        }
    }
}

/// Actions available in the local quick-settings panel. These are implemented
/// by the compositor itself; OS radios are intentionally not simulated.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum QuickAction {
    Theme,
    NightLight,
    ReduceMotion,
    Brightness,
    Wifi,
    Bluetooth,
}

/// Index of a quick-settings tile, also used by the motion state.
pub fn quick_action_index(action: QuickAction) -> usize {
    match action {
        QuickAction::Theme => 0,
        QuickAction::NightLight => 1,
        QuickAction::ReduceMotion => 2,
        QuickAction::Brightness => 3,
        QuickAction::Wifi => 4,
        QuickAction::Bluetooth => 5,
    }
}

#[derive(Clone, Debug)]
pub enum DockItem {
    Launcher,
    Launch(AppEntry),
    Focus {
        window: Window,
        key: String,
        app: Option<AppEntry>,
        pinned: bool,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LauncherHit {
    Scrim,
    Input,
    Tile(usize),
    Inside,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DecorHit {
    Minimize,
    Maximize,
    Close,
    Move,
    Resize(ResizeEdges),
    None,
}

/// Identity of an interactive chrome control, used for press/ripple motion.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PressKey {
    Home(usize),
    Top(TopAction),
    Dock(usize),
    Tile(usize),
    Chip(usize),
    Launcher(usize),
    Power(usize),
}

/// A circular clip applied to a piece of chrome during the radial theme
/// reveal: new-theme pixels inside the circle, old-theme pixels outside.
#[derive(Clone, Copy, Debug)]
pub struct CircleMask {
    pub cx: f32,
    pub cy: f32,
    pub radius: f32,
}

/// One draw-ready chrome layer: a cached texture plus where and how big it is.
pub struct LayerFrame {
    pub buffer: MemoryRenderBuffer,
    pub loc: Point<f64, Logical>,
    /// Size override; `Some` asks the GPU to scale the layer while drawing,
    /// `None` draws it at its natural size.
    pub size: Option<Size<i32, Logical>>,
}

/// The pieces of one overlay (launcher, quick settings, ...), back to front.
/// Each piece carries its own opacity; the caller multiplies in any extra
/// fade, such as the light/dark transition.
pub struct OverlayFrame {
    pub pieces: Vec<(LayerFrame, f32)>,
}

impl OverlayFrame {
    pub(super) fn single(
        buffer: MemoryRenderBuffer,
        loc: Point<f64, Logical>,
        size: Option<Size<i32, Logical>>,
        alpha: f32,
    ) -> Self {
        Self {
            pieces: vec![(LayerFrame { buffer, loc, size }, alpha)],
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub(super) struct Ripple {
    pub(super) cx: f32,
    pub(super) cy: f32,
    pub(super) max_r: f32,
    pub(super) progress: Tween,
}

#[derive(Clone, Copy, Debug)]
pub(super) struct PressAnim {
    pub(super) pressed: bool,
    pub(super) scale: Spring,
    pub(super) ripple: Option<Ripple>,
}

impl PressAnim {
    pub(super) fn new() -> Self {
        Self {
            pressed: false,
            scale: Spring::responsive(1.0),
            ripple: None,
        }
    }
}

/// An in-flight radial theme switch.
#[derive(Clone, Copy, Debug)]
pub(super) struct Reveal {
    pub(super) from: Theme,
    pub(super) origin: Point<f64, Logical>,
    pub(super) progress: Tween,
}

/// One cached chrome buffer (top bar, dock, ...).
pub(super) struct Piece {
    pub(super) key: u64,
    pub(super) surface: Surface,
    pub(super) buffer: MemoryRenderBuffer,
}

/// A rasterised chrome layer that is reused across frames (launcher content,
/// quick-settings sheet, ...). The key hashes everything drawn into it.
pub(super) struct CachedLayer {
    pub(super) key: u64,
    pub(super) surface: Surface,
    pub(super) buffer: MemoryRenderBuffer,
}

pub(super) struct WallpaperEntry {
    pub(super) w: u32,
    pub(super) h: u32,
    pub(super) key: u64,
    pub(super) surface: Surface,
    pub(super) buffer: MemoryRenderBuffer,
}

pub(super) struct DecorEntry {
    pub(super) key: u64,
    pub(super) surface: Surface,
    pub(super) buffer: MemoryRenderBuffer,
    pub(super) epoch: u64,
}

#[derive(Clone, Copy, Debug)]
pub struct DecorLayout {
    pub frame: RectF,
    pub content: RectF,
    pub titlebar: RectF,
    pub minimize: (f32, f32),
    pub maximize: (f32, f32),
    pub close: (f32, f32),
}

/// Focus/maximize state used when painting a window frame.
#[derive(Clone, Copy, Debug)]
pub struct DecorParams<'a> {
    pub meta: &'a WindowMeta,
    pub focused: bool,
    pub maximized: bool,
}

/// Keep both transition endpoints resident. Selecting a palette only swaps
/// handles; it does not copy pixels, upload a new texture or repaint text.
#[derive(Default)]
pub(super) struct LayerCache {
    current: Option<CachedLayer>,
    previous: Option<CachedLayer>,
}
impl LayerCache {
    pub fn select(&mut self, key: u64) -> bool {
        if self.current.as_ref().is_some_and(|layer| layer.key == key) {
            return true;
        }
        if self.previous.as_ref().is_some_and(|layer| layer.key == key) {
            std::mem::swap(&mut self.current, &mut self.previous);
            return true;
        }
        false
    }
    pub fn store(&mut self, layer: CachedLayer) {
        self.previous = self.current.replace(layer);
    }
    pub fn as_ref(&self) -> Option<&CachedLayer> {
        self.current.as_ref()
    }
    pub fn clear(&mut self) {
        *self = Self::default();
    }
}
