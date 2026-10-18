use smithay::{
    backend::renderer::element::memory::MemoryRenderBuffer,
    desktop::Window,
    utils::{Logical, Point, Rectangle, Size},
};
use std::collections::HashMap;
use tiny_skia::{Color, Path, PathBuilder, Transform as SkTransform};
use crate::{
    anim::{duration, CubicBezier, Spring, Tween},
    config::{self, AppEntry},
    font::{FontKind, Fonts},
    render::{round_rect_path, Surface},
    state::{ResizeEdges, WindowMeta},
    style::{shape, state, TextStyle},
    theme::{ColorVariant, KeyHash, Theme, ThemeSpec, VARIANTS},
};

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
    fn label(self) -> &'static str {
        match self {
            PowerAction::PowerOff => "Power off",
            PowerAction::Restart => "Restart",
            PowerAction::Logout => "Log out",
        }
    }

    fn icon(self) -> &'static str {
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
}

/// Index of a quick-settings tile, also used by the motion state.
pub fn quick_action_index(action: QuickAction) -> usize {
    match action {
        QuickAction::Theme => 0,
        QuickAction::NightLight => 1,
        QuickAction::ReduceMotion => 2,
        QuickAction::Brightness => 3,
    }
}

#[derive(Clone, Debug)]
pub enum DockItem {
    Launcher,
    Launch(AppEntry),
    Focus { window: Window, key: String },
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
    fn single(
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
struct Ripple {
    cx: f32,
    cy: f32,
    max_r: f32,
    progress: Tween,
}

#[derive(Clone, Copy, Debug)]
struct PressAnim {
    pressed: bool,
    scale: Spring,
    ripple: Option<Ripple>,
}

impl PressAnim {
    fn new() -> Self {
        Self {
            pressed: false,
            scale: Spring::spatial(1.0),
            ripple: None,
        }
    }
}

/// An in-flight radial theme switch.
#[derive(Clone, Copy, Debug)]
struct Reveal {
    from: Theme,
    origin: Point<f64, Logical>,
    progress: Tween,
}

/// One cached chrome buffer (top bar, dock, ...).
struct Piece {
    key: u64,
    surface: Surface,
    buffer: MemoryRenderBuffer,
}

/// A rasterised chrome layer that is reused across frames (launcher content,
/// quick-settings sheet, ...). The key hashes everything drawn into it.
struct CachedLayer {
    key: u64,
    surface: Surface,
    buffer: MemoryRenderBuffer,
}

struct WallpaperEntry {
    w: u32,
    h: u32,
    key: u64,
    surface: Surface,
    buffer: MemoryRenderBuffer,
}

struct DecorEntry {
    key: u64,
    surface: Surface,
    buffer: MemoryRenderBuffer,
    epoch: u64,
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
    cursor_cache: Option<(bool, MemoryRenderBuffer)>,

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

impl Shell {
    pub fn new(apps: Vec<AppEntry>, spec: ThemeSpec) -> Self {
        // The first pinned slot is always the terminal. Kitty entries win over
        // every other terminal, then any non-foot terminal, then foot itself;
        // a bare `kitty` is the fallback when nothing was discovered.
        let is_kitty = |a: &AppEntry| {
            let (name, exec) = (a.name.to_lowercase(), a.exec.to_lowercase());
            name.contains("kitty") || exec.contains("kitty")
        };
        let is_terminal = |a: &AppEntry| {
            let (name, exec) = (a.name.to_lowercase(), a.exec.to_lowercase());
            const TERMINALS: [&str; 8] = [
                "terminal",
                "kitty",
                "foot",
                "alacritty",
                "wezterm",
                "konsole",
                "terminator",
                "xterm",
            ];
            TERMINALS.iter().any(|t| name.contains(t) || exec.contains(t))
        };
        let is_foot = |a: &AppEntry| {
            let (name, exec) = (a.name.to_lowercase(), a.exec.to_lowercase());
            name.contains("foot") || exec.contains("foot")
        };
        let terminal_app = apps
            .iter()
            .find(|a| is_kitty(a))
            .or_else(|| apps.iter().find(|a| is_terminal(a) && !is_foot(a)))
            .or_else(|| apps.iter().find(|a| is_terminal(a)))
            .cloned()
            .unwrap_or_else(|| AppEntry {
                name: "Terminal".into(),
                exec: "kitty".into(),
                icon: "utilities-terminal".into(),
                terminal: false,
                desktop_file: None,
            });
        let terminal = terminal_app.command();

        Shell {
            apps,
            pinned: vec![terminal_app],
            terminal,
            launcher_open: false,
            launcher_query: String::new(),
            brightness_open: false,
            quick_settings_open: false,
            power_open: false,
            night_light_on: false,
            reduce_motion: false,
            clock: "00:00".into(),
            spec,
            wallpapers: Vec::new(),
            topbar_cache: Vec::new(),
            dock_cache: Vec::new(),
            decor_cache: HashMap::new(),
            launcher_layer: None,
            qs_layer: None,
            brightness_layer: None,
            power_layer: None,
            scrim_cache: None,
            frame_epoch: 0,
            cursor_cache: None,
            launcher: Spring::spatial(0.0),
            brightness: Spring::spatial(0.0),
            quick_settings: Spring::spatial(0.0),
            power: Spring::spatial(0.0),
            night_light: Spring::spatial(0.0),
            theme_mix: Tween::new(1.0, 1.0, duration::LONG2, CubicBezier::EMPHASIZED_DECELERATE),
            prev_theme: None,
            reveal: None,
            // The dark scheme is on by default, so its switch starts engaged.
            switches: [
                Spring::spatial(1.0),
                Spring::spatial(0.0),
                Spring::spatial(0.0),
                Spring::spatial(0.0),
            ],
            slider: Spring::spatial(0.0),
            window_in: HashMap::new(),
            window_out: HashMap::new(),
            press: HashMap::new(),
            hover_top: None,
            hover_dock: None,
            hover_power: None,
            press_top: None,
            press_dock: None,
            press_tile: None,
            press_chip: None,
            press_launch: None,
            press_power: None,
            launcher_origin: None,
            qs_origin: None,
            swatches_dark: None,
            swatches_light: None,
        }
    }

    pub fn theme_fade(&self) -> Option<(Theme, f32)> {
        match &self.prev_theme {
            Some(prev) if !self.theme_mix.is_done() => Some((*prev, self.theme_mix.value())),
            _ => None,
        }
    }

    pub fn mark_dirty(&mut self) {
        self.wallpapers.clear();
    }

    /// Launcher open progress, `0` closed .. `1` open.
    pub fn launcher_progress(&self) -> f32 {
        self.launcher.value()
    }

    /// Brightness popup progress, `0` hidden .. `1` shown.
    pub fn brightness_progress(&self) -> f32 {
        self.brightness.value()
    }

    pub fn set_launcher_open(&mut self, open: bool) {
        self.launcher.set_target(if open { 1.0 } else { 0.0 });
        if self.reduce_motion {
            self.launcher.snap(if open { 1.0 } else { 0.0 });
        }
    }

    pub fn set_launcher_origin(&mut self, rect: RectF) {
        self.launcher_origin = Some(rect);
    }

    pub fn set_brightness_open(&mut self, open: bool) {
        self.brightness.set_target(if open { 1.0 } else { 0.0 });
        if self.reduce_motion {
            self.brightness.snap(if open { 1.0 } else { 0.0 });
        }
    }

    pub fn quick_settings_progress(&self) -> f32 {
        self.quick_settings.value()
    }

    pub fn set_quick_settings_open(&mut self, open: bool) {
        self.quick_settings_open = open;
        self.quick_settings.set_target(if open { 1.0 } else { 0.0 });
        if self.reduce_motion {
            self.quick_settings.snap(if open { 1.0 } else { 0.0 });
        }
    }

    pub fn set_qs_origin(&mut self, rect: RectF) {
        self.qs_origin = Some(rect);
    }

    pub fn power_progress(&self) -> f32 {
        self.power.value()
    }

    pub fn set_power_open(&mut self, open: bool) {
        self.power.set_target(if open { 1.0 } else { 0.0 });
        if self.reduce_motion {
            self.power.snap(if open { 1.0 } else { 0.0 });
        }
    }

    pub fn night_light_progress(&self) -> f32 {
        self.night_light.value()
    }

    pub fn toggle_night_light(&mut self) {
        self.night_light_on = !self.night_light_on;
        let target = if self.night_light_on { 1.0 } else { 0.0 };
        self.night_light.set_target(target);
        let switch = &mut self.switches[quick_action_index(QuickAction::NightLight)];
        switch.set_target(target);
        if self.reduce_motion {
            self.night_light.snap(target);
            switch.snap(target);
        }
    }

    /// Drive the quick-settings "Dark theme" switch from the actual scheme.
    pub fn set_theme_switch(&mut self, dark: bool) {
        let target = if dark { 1.0 } else { 0.0 };
        let switch = &mut self.switches[quick_action_index(QuickAction::Theme)];
        switch.set_target(target);
        if self.reduce_motion {
            switch.snap(target);
        }
    }

    /// Grow the brightness slider handle while the user drags it.
    pub fn set_slider_active(&mut self, active: bool) {
        let target = if active { 1.0 } else { 0.0 };
        self.slider.set_target(target);
        if self.reduce_motion {
            self.slider.snap(target);
        }
    }

    fn switch_spring(&self, action: QuickAction) -> f32 {
        self.switches[quick_action_index(action)].value()
    }

    pub fn set_reduce_motion(&mut self, enabled: bool) {
        self.reduce_motion = enabled;
        // The tile's own switch must follow the setting, not just the springs.
        let switch = &mut self.switches[quick_action_index(QuickAction::ReduceMotion)];
        switch.set_target(if enabled { 1.0 } else { 0.0 });
        if enabled {
            switch.snap(1.0);
            self.launcher.snap(if self.launcher_open { 1.0 } else { 0.0 });
            self.brightness.snap(if self.brightness_open { 1.0 } else { 0.0 });
            self.quick_settings.snap(if self.quick_settings_open { 1.0 } else { 0.0 });
            self.night_light.snap(if self.night_light_on { 1.0 } else { 0.0 });
            self.theme_mix.snap(1.0);
            self.prev_theme = None;
            self.reveal = None;
            self.slider.snap(self.slider.value());
            for s in self.switches.iter_mut() {
                s.snap(s.value());
            }
            self.window_in.clear();
            for s in self.window_out.values_mut() {
                s.snap(1.0);
            }
            self.press.clear();
        }
    }

    /// Begin an animated crossfade from `old` to the next scheme (palette
    /// switches; light/dark uses the radial reveal instead).
    pub fn begin_theme_change(&mut self, old: Theme) {
        self.reveal = None;
        if self.reduce_motion {
            self.prev_theme = None;
            self.theme_mix.snap(1.0);
            return;
        }
        self.prev_theme = Some(old);
        self.theme_mix.start(0.0, 1.0);
    }

    /// Start the radial reveal from `origin` over the previous scheme.
    pub fn begin_reveal(&mut self, from: Theme, origin: Point<f64, Logical>) {
        self.prev_theme = None;
        self.theme_mix.snap(1.0);
        if self.reduce_motion {
            self.reveal = None;
            return;
        }
        self.reveal = Some(Reveal {
            from,
            origin,
            progress: Tween::new(0.0, 1.0, duration::LONG1, CubicBezier::EMPHASIZED_DECELERATE),
        });
    }

    /// `(previous scheme, origin, progress)` while a radial reveal runs.
    pub fn reveal_frame(&self) -> Option<(Theme, Point<f64, Logical>, f32)> {
        self.reveal
            .as_ref()
            .map(|r| (r.from, r.origin, r.progress.value()))
    }

    /// The scheme to render with this frame (blended during a theme change).
    pub fn effective_theme(&self, target: &Theme) -> Theme {
        match &self.prev_theme {
            Some(prev) if !self.theme_mix.is_done() => {
                Theme::blend(prev, target, self.theme_mix.value())
            }
            _ => *target,
        }
    }

    /// Advance every running animation by `dt`; returns `true` while the shell
    /// still needs redraws (so the main loop keeps the frame dirty).
    pub fn tick(&mut self, dt: f32) -> bool {
        let mut active = false;
        active |= self.launcher.step(dt);
        active |= self.brightness.step(dt);
        active |= self.quick_settings.step(dt);
        active |= self.power.step(dt);
        active |= self.night_light.step(dt);
        active |= self.slider.step(dt);
        for s in self.switches.iter_mut() {
            active |= s.step(dt);
        }
        for s in self.window_in.values_mut() {
            active |= s.step(dt);
        }
        for s in self.window_out.values_mut() {
            active |= s.step(dt);
        }
        self.window_in.retain(|_, s| !s.settled());
        for p in self.press.values_mut() {
            active |= p.scale.step(dt);
            if let Some(r) = &mut p.ripple {
                active |= r.progress.step(dt);
            }
        }
        self.press.retain(|_, p| {
            let ripple = p.ripple.as_ref().is_some_and(|r| !r.progress.is_done());
            p.pressed || ripple || !p.scale.settled()
        });
        if !self.theme_mix.is_done() {
            active |= self.theme_mix.step(dt);
            if self.theme_mix.is_done() {
                self.prev_theme = None;
            }
        }
        let mut reveal_done = false;
        if let Some(r) = &mut self.reveal {
            if !r.progress.is_done() {
                active |= r.progress.step(dt);
                reveal_done = r.progress.is_done();
            }
        }
        if reveal_done {
            self.reveal = None;
        }
        active
    }

    // ---- Press / ripple motion -----------------------------------------

    /// Pointer pressed on a chrome control: start the scale and ripple
    /// animations. `pressed_scale` is the squeezed scale (e.g. 0.88).
    pub fn press_begin(&mut self, key: PressKey, center: (f32, f32), max_r: f32, pressed_scale: f32) {
        let entry = self.press.entry(key).or_insert_with(PressAnim::new);
        entry.pressed = true;
        entry.scale.set_target(pressed_scale);
        if self.reduce_motion {
            entry.scale.snap(pressed_scale);
        }
        entry.ripple = Some(Ripple {
            cx: center.0,
            cy: center.1,
            max_r: max_r.max(2.0),
            progress: Tween::new(0.0, 1.0, duration::MEDIUM3, CubicBezier::STANDARD_DECELERATE),
        });
    }

    /// Pointer released: the control springs back to rest.
    pub fn press_end(&mut self, key: PressKey) {
        if let Some(entry) = self.press.get_mut(&key) {
            entry.pressed = false;
            entry.scale.set_target(1.0);
            if self.reduce_motion {
                entry.scale.snap(1.0);
            }
        }
    }

    fn press_scale(&self, key: PressKey) -> f32 {
        self.press
            .get(&key)
            .map(|p| p.scale.value())
            .unwrap_or(1.0)
    }

    fn ripple_draw(&self, key: PressKey) -> Option<(f32, f32, f32, f32)> {
        let ripple = self.press.get(&key)?.ripple.as_ref()?;
        if ripple.progress.is_done() {
            return None;
        }
        let t = ripple.progress.value().clamp(0.0, 1.0);
        let alpha = 0.12 * (1.0 - t) * (1.0 - t);
        Some((ripple.cx, ripple.cy, ripple.max_r * t, alpha))
    }

    /// Draw a bounded ripple for `key`, clipped to `clip`.
    fn draw_ripple(&self, s: &mut Surface, key: PressKey, clip: &Path, color: Color) {
        let Some((cx, cy, radius, alpha)) = self.ripple_draw(key) else {
            return;
        };
        if radius <= 0.5 {
            return;
        }
        let mut pb = PathBuilder::new();
        pb.push_circle(cx, cy, radius);
        if let Some(path) = pb.finish() {
            s.fill_path_clipped(&path, clip, Theme::with_alpha(color, alpha));
        }
    }

    // ---- Window open/close motion --------------------------------------

    pub fn on_window_mapped(&mut self, window: &Window) {
        self.window_out.remove(window);
        self.window_in.entry(window.clone()).or_insert_with(|| {
            // Starts hidden at 0.0 and animates to the rest pose.
            let mut spring = Spring::spatial(0.0);
            spring.set_target(1.0);
            spring
        });
    }

    pub fn on_window_destroyed(&mut self, window: &Window) {
        self.window_in.remove(window);
        self.window_out.remove(window);
    }

    /// Begin the closing animation. Returns `true` when the close must be sent
    /// immediately (reduced motion).
    pub fn begin_window_close(&mut self, window: &Window) -> bool {
        if self.reduce_motion {
            return true;
        }
        if self.window_out.contains_key(window) {
            return false;
        }
        self.window_in.remove(window);
        let mut spring = Spring::spatial(0.0);
        spring.set_target(1.0);
        self.window_out.insert(window.clone(), spring);
        false
    }

    pub fn is_closing(&self, window: &Window) -> bool {
        self.window_out.contains_key(window)
    }

    /// Windows whose fade-out has finished; the caller sends the close.
    pub fn take_closed_windows(&mut self) -> Vec<Window> {
        let done: Vec<Window> = self
            .window_out
            .iter()
            .filter(|(_, s)| s.value() >= 0.999)
            .map(|(w, _)| w.clone())
            .collect();
        for w in &done {
            self.window_out.remove(w);
        }
        done
    }

    /// `(scale, alpha)` while the window plays its entrance or exit.
    pub fn window_anim(&self, window: &Window) -> Option<(f64, f32)> {
        if let Some(s) = self.window_in.get(window) {
            let t = s.value().clamp(0.0, 1.0);
            if t < 0.999 {
                return Some((0.90 + 0.10 * t as f64, t));
            }
        }
        if let Some(s) = self.window_out.get(window) {
            let t = s.value().clamp(0.0, 1.0);
            return Some((1.0 - 0.06 * t as f64, (1.0 - t).max(0.0)));
        }
        None
    }

    /// `(primary, secondary, tertiary)` swatches for every palette variant,
    /// built lazily once per scheme brightness.
    fn swatches(&mut self, dark: bool) -> &[(Color, Color, Color); 9] {
        let seed = self.spec.seed;
        let slot = if dark {
            &mut self.swatches_dark
        } else {
            &mut self.swatches_light
        };
        slot.get_or_insert_with(|| {
            std::array::from_fn(|i| {
                crate::theme::variant_swatch(
                    ThemeSpec {
                        seed,
                        variant: VARIANTS[i],
                    },
                    dark,
                )
            })
        })
    }

    pub fn set_hover_top(&mut self, action: Option<TopAction>) {
        self.hover_top = action;
    }

    pub fn set_hover_dock(&mut self, index: Option<usize>) {
        self.hover_dock = index;
    }

    pub fn set_hover_power(&mut self, index: Option<usize>) {
        self.hover_power = index;
    }

    /// Begin a render pass; stale decoration caches are pruned in [`end_frame`].
    pub fn begin_frame(&mut self) {
        self.frame_epoch = self.frame_epoch.wrapping_add(1);
    }

    /// Drop decoration buffers that were not requested during this frame.
    pub fn end_frame(&mut self) {
        let epoch = self.frame_epoch;
        self.decor_cache.retain(|_, entries| {
            entries.retain(|e| e.epoch == epoch);
            !entries.is_empty()
        });
    }

    pub fn filtered_apps(&self) -> Vec<usize> {
        let q = self.launcher_query.to_lowercase();
        self.apps
            .iter()
            .enumerate()
            .filter(|(_, a)| q.is_empty() || a.name.to_lowercase().contains(&q))
            .map(|(i, _)| i)
            .collect()
    }

    pub fn launcher_push(&mut self, c: char) {
        if !c.is_control() && self.launcher_query.chars().count() < 64 {
            self.launcher_query.push(c);
        }
    }

    pub fn launcher_backspace(&mut self) {
        self.launcher_query.pop();
    }

    // ---- Rendering ------------------------------------------------------

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

    #[allow(clippy::too_many_arguments)]
    pub fn render_topbar(
        &mut self,
        theme: &Theme,
        fonts: &mut Fonts,
        win_w: f32,
        _title: &str,
        mask: Option<CircleMask>,
    ) -> (MemoryRenderBuffer, Point<f64, Logical>) {
        let rect = topbar_rect(win_w);
        let loc = Point::from((rect.x as f64, rect.y as f64));
        let key = KeyHash::new()
            .i32(win_w.round() as i32)
            .u64(theme.cache_key())
            .str(&self.clock)
            .i32(self.hover_top.map_or(-1, |a| a as i32))
            .i32(self.press_top.map_or(-1, |a| a as i32))
            .bool(self.quick_settings_open)
            .finish();
        let animating = self.press.keys().any(|k| matches!(k, PressKey::Top(_)));
        if !animating {
            if let Some(piece) = self.topbar_cache.iter().find(|p| p.key == key) {
                return masked_from(&piece.surface, mask, &piece.buffer, loc);
            }
        }
        let mut s = Surface::new(rect.w as u32, rect.h as u32);
        let cy = rect.h / 2.0;

        // Left side: launcher button, then the clock (classic Android phone
        // style). The launcher occupies the leading full-height square.
        self.draw_icon_button(
            &mut s,
            fonts,
            theme,
            topbar_button_rect(win_w, TopAction::Launcher),
            TopAction::Launcher,
            "apps",
            20.0,
            theme.on_surface,
            None,
        );
        let clock_style = TextStyle::LABEL_LARGE;
        fonts.draw_text_role(
            &mut s,
            16.0 + config::TOPBAR_HEIGHT + 4.0,
            cy - clock_style.line_height / 2.0,
            clock_style,
            &self.clock,
            theme.on_surface,
        );

        // Right side: status icons, right to left.
        self.draw_icon_button(
            &mut s,
            fonts,
            theme,
            topbar_button_rect(win_w, TopAction::Power),
            TopAction::Power,
            "account_circle",
            18.0,
            theme.on_surface,
            Some(theme.surface_container_high),
        );
        let theme_icon = if theme.is_dark {
            "light_mode"
        } else {
            "dark_mode"
        };
        self.draw_icon_button(
            &mut s,
            fonts,
            theme,
            topbar_button_rect(win_w, TopAction::Theme),
            TopAction::Theme,
            theme_icon,
            18.0,
            theme.on_surface_variant,
            None,
        );
        self.draw_icon_button(
            &mut s,
            fonts,
            theme,
            topbar_button_rect(win_w, TopAction::Brightness),
            TopAction::Brightness,
            "brightness_medium",
            18.0,
            theme.on_surface_variant,
            None,
        );
        let qs_bg = self.quick_settings_open.then_some(theme.secondary_container);
        self.draw_icon_button(
            &mut s,
            fonts,
            theme,
            topbar_button_rect(win_w, TopAction::QuickSettings),
            TopAction::QuickSettings,
            "settings",
            18.0,
            if self.quick_settings_open {
                theme.on_secondary_container
            } else {
                theme.on_surface_variant
            },
            qs_bg,
        );

        let buffer = s.to_memory_buffer();
        // During a radial reveal even a fresh piece must come back masked, or
        // its first frame would flash the full new scheme.
        let result = masked_from(&s, mask, &buffer, loc);
        if !animating {
            self.topbar_cache.insert(
                0,
                Piece {
                    key,
                    surface: s,
                    buffer,
                },
            );
            self.topbar_cache.truncate(4);
        }
        result
    }

    /// Draw one circular icon button of the top bar: its optional container
    /// circle, icon, state layer and ripple.
    #[allow(clippy::too_many_arguments)]
    fn draw_icon_button(
        &self,
        s: &mut Surface,
        fonts: &mut Fonts,
        theme: &Theme,
        rect: RectF,
        action: TopAction,
        icon: &str,
        icon_px: f32,
        icon_color: Color,
        bg: Option<Color>,
    ) {
        let cx = rect.x + rect.w / 2.0;
        let cy = rect.y + rect.h / 2.0;
        let key = PressKey::Top(action);
        let scale = self.press_scale(key);
        if let Some(bg) = bg {
            s.circle(cx, cy, rect.w / 2.0 * 0.8 * scale, bg);
        }
        fonts.draw_icon_centered(s, cx, cy, icon, icon_px * scale, icon_color);
        state_layer_overlay(
            s,
            theme,
            cx,
            cy,
            rect.w / 2.0,
            self.hover_top == Some(action),
            self.press_top == Some(action),
        );
        let mut cb = PathBuilder::new();
        cb.push_circle(cx, cy, rect.w / 2.0);
        if let Some(clip) = cb.finish() {
            self.draw_ripple(s, key, &clip, theme.on_surface);
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn render_dock(
        &mut self,
        theme: &Theme,
        fonts: &mut Fonts,
        win_w: f32,
        win_h: f32,
        items: &[DockItem],
        focused: Option<&Window>,
        mask: Option<CircleMask>,
    ) -> (MemoryRenderBuffer, Point<f64, Logical>) {
        let rect = dock_rect(win_w, win_h, items.len());
        let loc = Point::from((rect.x as f64, rect.y as f64));
        let mut key = KeyHash::new()
            .i32(win_w.round() as i32)
            .i32(win_h.round() as i32)
            .u64(theme.cache_key())
            .i32(self.hover_dock.map_or(-1, |i| i as i32));
        for item in items {
            match item {
                DockItem::Launcher => key = key.u32(1),
                DockItem::Launch(app) => key = key.u32(2).str(&app.icon),
                DockItem::Focus { window, key: k } => {
                    key = key.u32(3).str(k).bool(focused == Some(window));
                }
            }
        }
        let cache_key = key.finish();
        let animating = self.press.keys().any(|k| matches!(k, PressKey::Dock(_)));
        if !animating {
            if let Some(piece) = self.dock_cache.iter().find(|p| p.key == cache_key) {
                return masked_from(&piece.surface, mask, &piece.buffer, loc);
            }
        }
        let mut s = Surface::new(rect.w as u32, rect.h as u32);
        // Taskbar background: slightly elevated surface.
        s.fill_round_rect(0.0, 0.0, rect.w, rect.h, 0.0, theme.surface_container_high);
        s.stroke_round_rect(
            0.0, 0.5, rect.w, 1.0, 0.0, 1.0,
            Theme::with_alpha(theme.outline, 0.15),
        );

        // Launchers and window icons share one centred row of dock icons.
        let mut x = dock_row_start(rect.w, items.len());

        for (i, item) in items.iter().enumerate() {
            let cy = rect.h / 2.0;
            let icon_size = config::DOCK_ICON;
            let cx = x + icon_size / 2.0;
            let key = PressKey::Dock(i);
            let scale = self.press_scale(key);

            match item {
                DockItem::Launcher => {
                    s.circle(cx, cy, icon_size / 2.0 * scale, theme.primary_container);
                    fonts.draw_icon_centered(
                        &mut s,
                        cx,
                        cy,
                        "apps",
                        28.0 * scale,
                        theme.on_primary_container,
                    );
                }
                DockItem::Launch(app) => {
                    let icon = config::icon_for_key(&app.icon);
                    fonts.draw_icon_centered(&mut s, cx, cy, icon, 30.0 * scale, theme.on_surface);
                }
                DockItem::Focus { window, key: k } => {
                    let icon = config::icon_for_key(k);
                    let active = focused == Some(window);
                    if active {
                        let w = icon_size * scale;
                        let h = icon_size * scale;
                        s.fill_round_rect(
                            cx - w / 2.0,
                            cy - h / 2.0,
                            w,
                            h,
                            shape::MEDIUM,
                            theme.secondary_container,
                        );
                    }
                    fonts.draw_icon_centered(
                        &mut s,
                        cx,
                        cy,
                        icon,
                        30.0 * scale,
                        if active {
                            theme.on_secondary_container
                        } else {
                            theme.on_surface
                        },
                    );
                    // Indicator dot for running apps.
                    s.circle(cx, rect.h - 6.0, 2.0, theme.primary);
                }
            }

            let hovered = self.hover_dock == Some(i);
            let pressed = self.press_dock == Some(i);
            if hovered || pressed {
                let opacity = if pressed { state::PRESSED } else { state::HOVER };
                s.circle(
                    cx,
                    cy,
                    icon_size / 2.0 + 4.0,
                    theme.state_layer(theme.on_surface, opacity),
                );
            }
            let mut cb = PathBuilder::new();
            cb.push_circle(cx, cy, icon_size / 2.0 + 4.0);
            if let Some(clip) = cb.finish() {
                self.draw_ripple(&mut s, key, &clip, theme.on_surface);
            }

            x += icon_size + config::DOCK_GAP;
        }

        let buffer = s.to_memory_buffer();
        let result = masked_from(&s, mask, &buffer, loc);
        if !animating {
            self.dock_cache.insert(
                0,
                Piece {
                    key: cache_key,
                    surface: s,
                    buffer,
                },
            );
            self.dock_cache.truncate(4);
        }
        result
    }

    /// Cache key of the launcher's content layer. Press visuals are separate
    /// overlay pieces, so they never invalidate this layer.
    fn launcher_layer_key(&self, win_w: f32, win_h: f32, theme: &Theme) -> u64 {
        let mut key = KeyHash::new()
            .i32(win_w.round() as i32)
            .i32(win_h.round() as i32)
            .u64(theme.cache_key())
            .str(&self.launcher_query);
        let query = self.launcher_query.to_lowercase();
        for app in &self.apps {
            if query.is_empty() || app.name.to_lowercase().contains(&query) {
                key = key.str(&app.icon).str(&app.name);
            }
        }
        key.finish()
    }

    /// One launcher grid tile: icon container, icon and caption at `(x, y)`
    /// with the given press scale.
    #[allow(clippy::too_many_arguments)]
    fn draw_launcher_tile(
        &self,
        s: &mut Surface,
        fonts: &mut Fonts,
        theme: &Theme,
        app: &AppEntry,
        x: f32,
        y: f32,
        scale: f32,
    ) {
        let icon_name = config::icon_for_key(&app.icon);
        s.circle(
            x + config::LAUNCHER_TILE / 2.0,
            y + 36.0,
            32.0 * scale,
            theme.surface_container_highest,
        );
        fonts.draw_icon_centered(
            s,
            x + config::LAUNCHER_TILE / 2.0,
            y + 36.0,
            icon_name,
            36.0 * scale,
            theme.on_surface,
        );
        let label = truncate(&app.name, 14);
        let label_style = TextStyle::BODY_MEDIUM;
        let lw = fonts.text_width_styled(label_style, &label);
        fonts.draw_text_role(
            s,
            x + config::LAUNCHER_TILE / 2.0 - lw / 2.0,
            y + 76.0,
            label_style,
            &label,
            theme.on_surface,
        );
    }

    /// Draw the launcher's resting content (card, search pill, app grid) into
    /// its cached layer. Tiles are painted at rest scale; press feedback lives
    /// in separate overlay pieces.
    fn paint_launcher_layer(
        &self,
        layer: &mut Surface,
        fonts: &mut Fonts,
        theme: &Theme,
        win_w: f32,
        win_h: f32,
    ) {
        let card = launcher_card(win_w, win_h);
        // The elevation shadow is baked in: it scales with the container, so
        // it must not be redrawn on every animation frame.
        const LAUNCHER_SHADOW: f32 = 18.0;
        let pad = LAUNCHER_SHADOW + 2.0;
        let ox = card.x - pad;
        let oy = card.y - pad;
        layer.shadow_round_rect(
            card.x - ox,
            card.y - oy,
            card.w,
            card.h,
            config::LAUNCHER_RADIUS,
            LAUNCHER_SHADOW,
        );
        layer.fill_round_rect(
            card.x - ox,
            card.y - oy,
            card.w,
            card.h,
            config::LAUNCHER_RADIUS,
            theme.surface_container_low,
        );
        layer.stroke_round_rect(
            card.x - ox + 0.5,
            card.y - oy + 0.5,
            card.w - 1.0,
            card.h - 1.0,
            config::LAUNCHER_RADIUS,
            1.0,
            Theme::with_alpha(theme.outline, 0.1),
        );

        // Drag handle (Android-style).
        layer.fill_round_rect(
            card.x - ox + card.w / 2.0 - 16.0,
            card.y - oy + 12.0,
            32.0,
            4.0,
            2.0,
            theme.surface_container_highest,
        );

        // Search pill.
        let sp = search_rect(win_w, win_h);
        let spx = sp.x - ox;
        let spy = sp.y - oy;
        layer.fill_round_rect(
            spx,
            spy,
            sp.w,
            sp.h,
            sp.h / 2.0,
            theme.surface_container_highest,
        );
        fonts.draw_icon_centered(
            layer,
            spx + 24.0,
            spy + sp.h / 2.0,
            "search",
            24.0,
            theme.on_surface_variant,
        );
        let query = if self.launcher_query.is_empty() {
            "Search your apps"
        } else {
            &self.launcher_query
        };
        let search_style = TextStyle::BODY_LARGE;
        fonts.draw_text_role(
            layer,
            spx + 52.0,
            spy + sp.h / 2.0 - search_style.line_height / 2.0,
            search_style,
            query,
            if self.launcher_query.is_empty() {
                Theme::with_alpha(theme.on_surface_variant, 0.6)
            } else {
                theme.on_surface
            },
        );

        let cols = launcher_columns(card.w);
        let origin_g = launcher_grid_origin(win_w, win_h);
        let filtered = self.filtered_apps();
        for (slot, &idx) in filtered.iter().enumerate() {
            let (tx, ty) = launcher_tile_pos(origin_g, slot, cols);
            if ty + config::LAUNCHER_TILE > card.bottom() - 16.0 {
                break;
            }
            let app = &self.apps[idx];
            self.draw_launcher_tile(layer, fonts, theme, app, tx - ox, ty - oy, 1.0);
        }

        if filtered.is_empty() {
            let msg = "No apps found";
            let msg_style = TextStyle::BODY_LARGE;
            let mw = fonts.text_width_styled(msg_style, msg);
            fonts.draw_text_role(
                layer,
                card.x - ox + card.w / 2.0 - mw / 2.0,
                card.y - oy + 180.0,
                msg_style,
                msg,
                theme.on_surface_variant,
            );
        }
    }

    /// The pressed (or still rippling) tiles rendered as small overlays, in
    /// screen coordinates. The big content layer underneath is untouched, so a
    /// press never re-rasterises the whole launcher.
    fn launcher_press_surfaces(
        &self,
        fonts: &mut Fonts,
        theme: &Theme,
        win_w: f32,
        win_h: f32,
    ) -> Vec<(Surface, (f32, f32))> {
        let mut out = Vec::new();
        let card = launcher_card(win_w, win_h);
        let cols = launcher_columns(card.w);
        let origin = launcher_grid_origin(win_w, win_h);
        let filtered = self.filtered_apps();
        for (slot, &idx) in filtered.iter().enumerate() {
            let key = PressKey::Launcher(slot);
            let Some(anim) = self.press.get(&key) else {
                continue;
            };
            let ripple_active = anim
                .ripple
                .as_ref()
                .is_some_and(|r| !r.progress.is_done());
            if !anim.pressed && !ripple_active && anim.scale.settled() {
                continue;
            }
            let (tx, ty) = launcher_tile_pos(origin, slot, cols);
            if ty + config::LAUNCHER_TILE > card.bottom() - 16.0 {
                break;
            }
            let mut s = Surface::new(config::LAUNCHER_TILE as u32, config::LAUNCHER_TILE as u32);
            // Erase the resting tile underneath before drawing the squeezed one.
            s.clear(theme.surface_container_low);
            let scale = anim.scale.value();
            self.draw_launcher_tile(&mut s, fonts, theme, &self.apps[idx], 0.0, 0.0, scale);
            let clip = round_rect_path(
                0.0,
                0.0,
                config::LAUNCHER_TILE,
                config::LAUNCHER_TILE,
                shape::LARGE,
            );
            self.draw_ripple(&mut s, key, &clip, theme.on_surface);
            out.push((s, (tx, ty)));
        }
        out
    }

    /// A 1x1 buffer of the scheme's scrim colour, stretched by the GPU.
    fn scrim_buffer(&mut self, theme: &Theme) -> MemoryRenderBuffer {
        let key = theme.cache_key();
        if let Some((k, buf)) = &self.scrim_cache {
            if *k == key {
                return buf.clone();
            }
        }
        let mut s = Surface::new(1, 1);
        s.clear(theme.scrim);
        let buf = s.to_memory_buffer();
        self.scrim_cache = Some((key, buf.clone()));
        buf
    }

    #[allow(clippy::too_many_arguments)]
    pub fn render_launcher(
        &mut self,
        theme: &Theme,
        fonts: &mut Fonts,
        win_w: f32,
        win_h: f32,
        progress: f32,
        mask: Option<CircleMask>,
    ) -> OverlayFrame {
        let p = progress.clamp(0.0, 1.0);
        let ease = CubicBezier::EMPHASIZED_DECELERATE.eval(p);
        let card = launcher_card(win_w, win_h);
        let origin = self.launcher_origin.unwrap_or(RectF {
            x: 0.0,
            y: 0.0,
            w: config::TOPBAR_HEIGHT,
            h: config::TOPBAR_HEIGHT,
        });
        const LAUNCHER_SHADOW: f32 = 18.0;
        let pad = LAUNCHER_SHADOW + 2.0;
        // Top-left of the content layer in screen space.
        let ox = card.x - pad;
        let oy = card.y - pad;

        // Content layer: laid out at its final geometry, cached across frames.
        let layer_key = self.launcher_layer_key(win_w, win_h, theme);
        if self
            .launcher_layer
            .as_ref()
            .map(|l| l.key != layer_key)
            .unwrap_or(true)
        {
            let mut layer = Surface::new(
                (card.w + 2.0 * pad).ceil() as u32,
                (card.h + 2.0 * pad).ceil() as u32,
            );
            self.paint_launcher_layer(&mut layer, fonts, theme, win_w, win_h);
            let buffer = layer.to_memory_buffer();
            self.launcher_layer = Some(CachedLayer {
                key: layer_key,
                surface: layer,
                buffer,
            });
        }

        // Container transform: the card morphs from the control that opened it
        // into its final rectangle while it fades in.
        let lerp = |a: f32, b: f32| a + (b - a) * ease;
        let anim = RectF {
            x: lerp(origin.x, card.x),
            y: lerp(origin.y, card.y),
            w: lerp(origin.w, card.w),
            h: lerp(origin.h, card.h),
        };
        let sx = (anim.w / card.w).clamp(0.01, 1.0);
        let sy = (anim.h / card.h).clamp(0.01, 1.0);

        if let Some(mask) = mask {
            // Radial reveal: the circular clip is applied on the CPU, so the
            // card and the press pieces are composited into one surface first.
            let cached = self.launcher_layer.as_ref().expect("just cached");
            let mut content = cached.surface.duplicate();
            for (press, (px, py)) in self.launcher_press_surfaces(fonts, theme, win_w, win_h)
            {
                content.blit(
                    &press,
                    SkTransform::from_translate(px - ox, py - oy),
                    1.0,
                    None,
                );
            }
            let mut s = Surface::new(win_w as u32, win_h as u32);
            s.clear(Theme::with_alpha(theme.scrim, 0.45 * ease));
            let transform = SkTransform::from_scale(sx, sy)
                .post_translate(anim.x - pad * sx, anim.y - pad * sy);
            s.blit(&content, transform, ease.clamp(0.0, 1.0), None);
            let (buffer, loc) =
                masked_from(&s, Some(mask), &s.to_memory_buffer(), Point::from((0.0, 0.0)));
            return OverlayFrame::single(buffer, loc, None, 1.0);
        }

        // GPU path: a stretched 1x1 scrim, the scaled card layer, then one
        // small overlay per pressed tile.
        let mut pieces: Vec<(LayerFrame, f32)> = Vec::with_capacity(2);
        if p > 0.001 {
            pieces.push((
                LayerFrame {
                    buffer: self.scrim_buffer(theme),
                    loc: Point::from((0.0, 0.0)),
                    size: Some(Size::from((
                        win_w.round() as i32,
                        win_h.round() as i32,
                    ))),
                },
                0.45 * ease,
            ));
        }
        {
            let layer = self.launcher_layer.as_ref().expect("just cached");
            let layer_w = layer.surface.width() as f32;
            let layer_h = layer.surface.pixmap.height() as f32;
            pieces.push((
                LayerFrame {
                    buffer: layer.buffer.clone(),
                    loc: Point::from(((anim.x - pad * sx) as f64, (anim.y - pad * sy) as f64)),
                    size: Some(Size::from((
                        (layer_w * sx).round() as i32,
                        (layer_h * sy).round() as i32,
                    ))),
                },
                ease.clamp(0.0, 1.0),
            ));
        }
        for (press, (px, py)) in self.launcher_press_surfaces(fonts, theme, win_w, win_h) {
            // Keep the feedback glued to the scaled card while it animates.
            let loc = (
                anim.x as f64 + (px - card.x) as f64 * sx as f64,
                anim.y as f64 + (py - card.y) as f64 * sy as f64,
            );
            pieces.push((
                LayerFrame {
                    buffer: press.to_memory_buffer(),
                    loc: Point::from(loc),
                    size: Some(Size::from((
                        (config::LAUNCHER_TILE * sx).round() as i32,
                        (config::LAUNCHER_TILE * sy).round() as i32,
                    ))),
                },
                ease.clamp(0.0, 1.0),
            ));
        }
        OverlayFrame { pieces }
    }

    /// Cache key of the brightness popup's content layer.
    fn brightness_layer_key(&self, layer_w: u32, layer_h: u32, theme: &Theme, brightness: f32) -> u64 {
        KeyHash::new()
            .u32(layer_w)
            .u32(layer_h)
            .u64(theme.cache_key())
            .u32((brightness.clamp(0.0, 1.0) * 1000.0).round() as u32)
            .finish()
    }

    /// Draw the brightness popup content (card, icon, slider) into `layer`.
    fn paint_brightness(
        &self,
        layer: &mut Surface,
        fonts: &mut Fonts,
        theme: &Theme,
        card: &RectF,
        win_w: f32,
        brightness: f32,
    ) {
        let pad = SHADOW_PAD;
        layer.shadow_round_rect(pad, pad, card.w, card.h, shape::LARGE, SHADOW_SPREAD);
        layer.fill_round_rect(0.0 + pad, pad, card.w, card.h, shape::LARGE, theme.surface_container_high);
        layer.stroke_round_rect(
            pad + 0.5,
            pad + 0.5,
            card.w - 1.0,
            card.h - 1.0,
            shape::LARGE,
            1.0,
            Theme::with_alpha(theme.outline, 0.22),
        );

        let cy = pad + card.h / 2.0;
        fonts.draw_icon_centered(layer, pad + 28.0, cy, "brightness_6", 22.0, theme.on_surface);

        let track = brightness_track(win_w);
        let tx = track.x - card.x + pad;
        let t = ((brightness - BRIGHTNESS_MIN) / (BRIGHTNESS_MAX - BRIGHTNESS_MIN)).clamp(0.0, 1.0);
        layer.fill_round_rect(tx, cy - 3.0, track.w, 6.0, 3.0, theme.surface_container_highest);
        layer.fill_round_rect(tx, cy - 3.0, track.w * t, 6.0, 3.0, theme.primary);
        // M3 slider: the handle grows while grabbed.
        let handle = 8.0 + 5.0 * self.slider.value().clamp(0.0, 1.0);
        layer.circle(tx + track.w * t, cy, handle, theme.primary);
    }

    pub fn render_brightness(
        &mut self,
        theme: &Theme,
        fonts: &mut Fonts,
        win_w: f32,
        brightness: f32,
        progress: f32,
        mask: Option<CircleMask>,
    ) -> OverlayFrame {
        let p = progress.clamp(0.0, 1.0);
        let card = brightness_popup(win_w);
        let ease = CubicBezier::EMPHASIZED_DECELERATE.eval(p);
        let dy = (topbar_rect(win_w).bottom() - card.y) * (1.0 - ease);
        let scale = 0.94 + 0.06 * ease;
        let pad = SHADOW_PAD;
        let layer_w = (card.w + 2.0 * pad).ceil() as u32;
        let layer_h = (card.h + 2.0 * pad).ceil() as u32;
        let loc_x = card.x as f64 - pad as f64;
        let loc_y = card.y as f64 - pad as f64;
        let cx = pad + card.w / 2.0;
        let cy_card = pad + card.h / 2.0;
        let scale_tx = cx * (1.0 - scale);
        let scale_ty = cy_card * (1.0 - scale) + dy;

        // While the slider is animating its handle the content changes; such
        // frames are rebuilt, settled ones reuse the cached layer.
        let stable = self.slider.settled();
        let key = self.brightness_layer_key(layer_w, layer_h, theme, brightness);
        let cached_ok = mask.is_none()
            && stable
            && self.brightness_layer.as_ref().is_some_and(|l| l.key == key);
        let fresh = if cached_ok {
            None
        } else {
            let mut layer = Surface::new(layer_w, layer_h);
            self.paint_brightness(&mut layer, fonts, theme, &card, win_w, brightness);
            let buffer = layer.to_memory_buffer();
            if mask.is_none() && stable {
                self.brightness_layer = Some(CachedLayer { key, surface: layer, buffer });
                None
            } else {
                Some(CachedLayer { key, surface: layer, buffer })
            }
        };
        let cached = fresh
            .as_ref()
            .or(self.brightness_layer.as_ref())
            .expect("just built");

        if let Some(mask) = mask {
            // Radial reveal: the circle clip is CPU work, so the scaled layer
            // is composited and masked here.
            let mut s = Surface::new(layer_w, layer_h);
            let transform = SkTransform::from_scale(scale, scale).post_translate(scale_tx, scale_ty);
            s.blit(&cached.surface, transform, 1.0, None);
            let (buffer, loc) =
                masked_from(&s, Some(mask), &s.to_memory_buffer(), Point::from((loc_x, loc_y)));
            return OverlayFrame::single(buffer, loc, None, p);
        }

        OverlayFrame::single(
            cached.buffer.clone(),
            Point::from((loc_x + scale_tx as f64, loc_y + scale_ty as f64)),
            Some(Size::from((
                (layer_w as f32 * scale).round() as i32,
                (layer_h as f32 * scale).round() as i32,
            ))),
            p,
        )
    }

    /// Draw the session menu rows into `layer`.
    fn paint_power_menu(
        &self,
        layer: &mut Surface,
        fonts: &mut Fonts,
        theme: &Theme,
        card: &RectF,
        win_w: f32,
    ) {
        let pad = SHADOW_PAD;
        layer.shadow_round_rect(pad, pad, card.w, card.h, shape::LARGE, SHADOW_SPREAD);
        layer.fill_round_rect(pad, pad, card.w, card.h, shape::LARGE, theme.surface_container_high);
        layer.stroke_round_rect(
            pad + 0.5,
            pad + 0.5,
            card.w - 1.0,
            card.h - 1.0,
            shape::LARGE,
            1.0,
            Theme::with_alpha(theme.outline, 0.22),
        );

        for (i, action) in POWER_ACTIONS.iter().enumerate() {
            let row = power_row_rect(win_w, i);
            let rx = row.x - card.x + pad;
            let ry = row.y - card.y + pad;
            let key = PressKey::Power(i);
            // The press spring drives the state layer, so a release fades the
            // highlight out over the same frames as the squeeze.
            let hold = ((1.0 - self.press_scale(key)) / 0.06).clamp(0.0, 1.0);
            let hovered = self.hover_power == Some(i);
            if hovered || hold > 0.001 {
                layer.fill_round_rect(
                    rx,
                    ry,
                    row.w,
                    row.h,
                    shape::MEDIUM,
                    theme.state_layer(theme.on_surface, state::HOVER.max(state::PRESSED * hold)),
                );
            }
            let clip = round_rect_path(rx, ry, row.w, row.h, shape::MEDIUM);
            self.draw_ripple(layer, key, &clip, theme.on_surface);

            let cy = ry + row.h / 2.0;
            fonts.draw_icon_centered(layer, rx + 28.0, cy, action.icon(), 22.0, theme.on_surface);
            fonts.draw_text_role(
                layer,
                rx + 56.0,
                cy - TextStyle::LABEL_LARGE.line_height / 2.0,
                TextStyle::LABEL_LARGE,
                action.label(),
                theme.on_surface,
            );
        }
    }

    /// Render the session menu: one row per system action, opening from the
    /// top-bar user button.
    pub fn render_power_menu(
        &mut self,
        theme: &Theme,
        fonts: &mut Fonts,
        win_w: f32,
        progress: f32,
        mask: Option<CircleMask>,
    ) -> OverlayFrame {
        let p = progress.clamp(0.0, 1.0);
        let card = power_menu_rect(win_w);
        let ease = CubicBezier::EMPHASIZED_DECELERATE.eval(p);
        let dy = (topbar_rect(win_w).bottom() - card.y) * (1.0 - ease);
        let scale = 0.94 + 0.06 * ease;
        let pad = SHADOW_PAD;
        let layer_w = (card.w + 2.0 * pad).ceil() as u32;
        let layer_h = (card.h + 2.0 * pad).ceil() as u32;
        let loc_x = card.x as f64 - pad as f64;
        let loc_y = card.y as f64 - pad as f64;
        // Grow out of the top-right corner, where the button sits.
        let anchor_x = pad + card.w;
        let anchor_y = pad;
        let scale_tx = anchor_x * (1.0 - scale);
        let scale_ty = anchor_y * (1.0 - scale) + dy;

        // Only hover and press visuals animate; settled frames are cached.
        let stable = !self.press.keys().any(|k| matches!(k, PressKey::Power(_)));
        let key = KeyHash::new()
            .u32(layer_w)
            .u32(layer_h)
            .u64(theme.cache_key())
            .i32(self.hover_power.map_or(-1, |i| i as i32))
            .finish();
        let cached_ok = mask.is_none()
            && stable
            && self.power_layer.as_ref().is_some_and(|l| l.key == key);
        let fresh = if cached_ok {
            None
        } else {
            let mut layer = Surface::new(layer_w, layer_h);
            self.paint_power_menu(&mut layer, fonts, theme, &card, win_w);
            let buffer = layer.to_memory_buffer();
            if mask.is_none() && stable {
                self.power_layer = Some(CachedLayer { key, surface: layer, buffer });
                None
            } else {
                Some(CachedLayer { key, surface: layer, buffer })
            }
        };
        let cached = fresh
            .as_ref()
            .or(self.power_layer.as_ref())
            .expect("just built");

        if let Some(mask) = mask {
            let mut s = Surface::new(layer_w, layer_h);
            let transform = SkTransform::from_scale(scale, scale).post_translate(scale_tx, scale_ty);
            s.blit(&cached.surface, transform, 1.0, None);
            let (buffer, loc) =
                masked_from(&s, Some(mask), &s.to_memory_buffer(), Point::from((loc_x, loc_y)));
            return OverlayFrame::single(buffer, loc, None, p);
        }

        OverlayFrame::single(
            cached.buffer.clone(),
            Point::from((loc_x + scale_tx as f64, loc_y + scale_ty as f64)),
            Some(Size::from((
                (layer_w as f32 * scale).round() as i32,
                (layer_h as f32 * scale).round() as i32,
            ))),
            p,
        )
    }

    /// Draw the quick-settings sheet content (card, toggles, palette grid).
    fn paint_quick_settings(
        &mut self,
        layer: &mut Surface,
        fonts: &mut Fonts,
        theme: &Theme,
        rect: RectF,
        brightness: f32,
        variant: ColorVariant,
    ) {
        let pad = SHADOW_PAD;
        layer.shadow_round_rect(pad, pad, rect.w, rect.h, shape::EXTRA_LARGE, SHADOW_SPREAD);
        layer.fill_round_rect(pad, pad, rect.w, rect.h, shape::EXTRA_LARGE, theme.surface_container_high);
        layer.stroke_round_rect(
            pad + 0.5,
            pad + 0.5,
            rect.w - 1.0,
            rect.h - 1.0,
            shape::EXTRA_LARGE,
            1.0,
            Theme::with_alpha(theme.outline, 0.2),
        );

        let title_style = TextStyle::TITLE_MEDIUM;
        fonts.draw_text_role(
            layer,
            pad + 16.0,
            pad + 14.0,
            title_style,
            "Quick settings",
            theme.on_surface,
        );

        let tile_w = ((rect.w - 44.0) / 2.0).max(56.0);
        let tile_h = 72.0;
        let x1 = pad + 16.0;
        let x2 = x1 + tile_w + 12.0;
        let y1 = pad + 48.0;
        let y2 = y1 + tile_h + 12.0;
        let tiles = [
            (x1, y1, "dark_mode", "Dark theme", QuickAction::Theme),
            (x2, y1, "nightlight", "Night light", QuickAction::NightLight),
            (
                x1,
                y2,
                "reduce_motion",
                "Reduce motion",
                QuickAction::ReduceMotion,
            ),
            (x2, y2, "brightness_6", "Brightness", QuickAction::Brightness),
        ];

        for (i, (x, y, icon, label, action)) in tiles.iter().enumerate() {
            let action = *action;
            let key = PressKey::Tile(i);
            let scale_p = self.press_scale(key);
            let tw = tile_w * scale_p;
            let th = tile_h * scale_p;
            let tx = x + (tile_w - tw) / 2.0;
            let ty = y + (tile_h - th) / 2.0;

            let t = self.switch_spring(action).clamp(0.0, 1.0);
            let active = t > 0.5;
            let fill = Theme::mix(theme.surface_container_low, theme.secondary_container, t);
            layer.fill_round_rect(tx, ty, tw, th, shape::LARGE, fill);
            layer.stroke_round_rect(
                tx + 0.5,
                ty + 0.5,
                tw - 1.0,
                th - 1.0,
                shape::LARGE,
                1.0,
                Theme::with_alpha(
                    if active { theme.primary } else { theme.outline_variant },
                    0.35,
                ),
            );
            let on_color = Theme::mix(theme.on_surface_variant, theme.on_secondary_container, t);
            fonts.draw_icon_centered(layer, x + 18.0, y + 22.0, icon, 18.0, on_color);
            fonts.draw_text_role(
                layer,
                x + 34.0,
                y + 13.0,
                TextStyle::BODY_SMALL,
                label,
                on_color,
            );

            if action == QuickAction::Brightness {
                let value = format!("{}%", (brightness.clamp(0.0, 1.0) * 100.0).round() as u32);
                fonts.draw_text_role(
                    layer,
                    x + 34.0,
                    y + 34.0,
                    TextStyle::LABEL_MEDIUM,
                    &value,
                    theme.on_surface_variant,
                );
            } else {
                draw_m3_switch(
                    layer,
                    x + tile_w - 60.0,
                    y + 38.0,
                    self.switch_spring(action),
                    theme,
                );
            }

            if self.press_tile == Some(action) {
                layer.fill_round_rect(
                    tx,
                    ty,
                    tw,
                    th,
                    shape::LARGE,
                    theme.state_layer(theme.on_surface, state::PRESSED),
                );
            }
            let clip = round_rect_path(tx, ty, tw, th, shape::LARGE);
            self.draw_ripple(layer, key, &clip, theme.on_surface);
        }

        // ---- Palette picker --------------------------------------------
        fonts.draw_text_role(
            layer,
            pad + 16.0,
            pad + QS_CHIPS_Y - 24.0,
            TextStyle::TITLE_SMALL,
            "Palette",
            theme.on_surface,
        );
        let current = variant.name();
        let current_w = fonts.text_width_styled(TextStyle::LABEL_MEDIUM, current);
        fonts.draw_text_role(
            layer,
            pad + rect.w - 16.0 - current_w,
            pad + QS_CHIPS_Y - 22.0,
            TextStyle::LABEL_MEDIUM,
            current,
            theme.on_surface_variant,
        );

        let swatches = *self.swatches(theme.is_dark);
        for (i, &variant_i) in VARIANTS.iter().enumerate() {
            let chip = palette_chip_rect_in(rect, i);
            if chip.bottom() > rect.bottom() - 6.0 {
                break;
            }
            let key = PressKey::Chip(i);
            let scale_p = self.press_scale(key);
            let cw = chip.w * scale_p;
            let ch = chip.h * scale_p;
            let cx = chip.x - rect.x + pad;
            let cy = chip.y - rect.y + pad;
            let x = cx + (chip.w - cw) / 2.0;
            let y = cy + (chip.h - ch) / 2.0;

            let selected = variant_i == variant;
            let fill = if selected {
                theme.secondary_container
            } else {
                theme.surface_container_low
            };
            let outline = if selected {
                Theme::with_alpha(theme.primary, 0.5)
            } else {
                Theme::with_alpha(theme.outline_variant, 0.5)
            };
            layer.fill_round_rect(x, y, cw, ch, ch / 2.0, fill);
            layer.stroke_round_rect(x + 0.5, y + 0.5, cw - 1.0, ch - 1.0, ch / 2.0, 1.0, outline);

            let (p1, p2, p3) = swatches[i];
            let dot_y = y + ch / 2.0;
            layer.circle(x + 14.0, dot_y, 4.0, p1);
            layer.circle(x + 25.0, dot_y, 4.0, p2);
            layer.circle(x + 36.0, dot_y, 4.0, p3);

            let name = truncate(variant_i.name(), 10);
            fonts.draw_text_role(
                layer,
                x + 48.0,
                dot_y - TextStyle::LABEL_SMALL.line_height / 2.0,
                TextStyle::LABEL_SMALL,
                &name,
                if selected {
                    theme.on_secondary_container
                } else {
                    theme.on_surface_variant
                },
            );

            if self.press_chip == Some(i) {
                layer.fill_round_rect(
                    x,
                    y,
                    cw,
                    ch,
                    ch / 2.0,
                    theme.state_layer(theme.on_surface, state::PRESSED),
                );
            }
            let clip = round_rect_path(x, y, cw, ch, ch / 2.0);
            self.draw_ripple(layer, key, &clip, theme.on_surface);

            // Keep the borrow checker happy: `swatches` is a copy.
        }
    }

    /// Render a compact, keyboard-friendly Material 3 quick-settings sheet:
    /// four local toggles plus the palette picker grid. Settled content is
    /// cached and scaled by the GPU; only the tile/chip animations rasterise.
    #[allow(clippy::too_many_arguments)]
    pub fn render_quick_settings(
        &mut self,
        theme: &Theme,
        fonts: &mut Fonts,
        win_w: f32,
        win_h: f32,
        progress: f32,
        brightness: f32,
        variant: ColorVariant,
        mask: Option<CircleMask>,
    ) -> OverlayFrame {
        let p = progress.clamp(0.0, 1.0);
        let rect = quick_settings_rect(win_w, win_h);
        let ease = CubicBezier::EMPHASIZED_DECELERATE.eval(p);
        let origin = self
            .qs_origin
            .unwrap_or_else(|| topbar_button_rect(win_w, TopAction::QuickSettings));
        let dy = (origin.y + origin.h / 2.0 - (rect.y + rect.h / 2.0)) * (1.0 - ease);
        let scale = 0.92 + 0.08 * ease;
        let pad = SHADOW_PAD;
        let layer_w = (rect.w + 2.0 * pad).ceil() as u32;
        let layer_h = (rect.h + 2.0 * pad).ceil() as u32;
        let loc_x = rect.x as f64 - pad as f64;
        let loc_y = rect.y as f64 - pad as f64;
        let cx = pad + rect.w / 2.0;
        let cy = pad + rect.h / 2.0;
        let scale_tx = cx * (1.0 - scale);
        let scale_ty = cy * (1.0 - scale) + dy;

        // The sheet only changes while a switch, tile or chip is in motion;
        // such frames are rebuilt directly, settled ones reuse the cache.
        let stable = self.switches.iter().all(|s| s.settled())
            && !self
                .press
                .keys()
                .any(|k| matches!(k, PressKey::Tile(_) | PressKey::Chip(_)));
        let mut key = KeyHash::new()
            .u32(layer_w)
            .u32(layer_h)
            .u64(theme.cache_key())
            .str(variant.name())
            .u32((brightness.clamp(0.0, 1.0) * 100.0).round() as u32);
        for s in &self.switches {
            key = key.f32(s.value());
        }
        let key = key.finish();

        let cached_ok = mask.is_none()
            && stable
            && self.qs_layer.as_ref().is_some_and(|l| l.key == key);
        let fresh = if cached_ok {
            None
        } else {
            let mut layer = Surface::new(layer_w, layer_h);
            self.paint_quick_settings(&mut layer, fonts, theme, rect, brightness, variant);
            let buffer = layer.to_memory_buffer();
            if mask.is_none() && stable {
                self.qs_layer = Some(CachedLayer { key, surface: layer, buffer });
                None
            } else {
                Some(CachedLayer { key, surface: layer, buffer })
            }
        };
        let cached = fresh.as_ref().or(self.qs_layer.as_ref()).expect("just built");

        if let Some(mask) = mask {
            // Radial reveal: CPU composite + circle clip.
            let mut s = Surface::new(layer_w, layer_h);
            let transform = SkTransform::from_scale(scale, scale).post_translate(scale_tx, scale_ty);
            s.blit(&cached.surface, transform, 1.0, None);
            let (buffer, loc) =
                masked_from(&s, Some(mask), &s.to_memory_buffer(), Point::from((loc_x, loc_y)));
            return OverlayFrame::single(buffer, loc, None, p);
        }

        OverlayFrame::single(
            cached.buffer.clone(),
            Point::from((loc_x + scale_tx as f64, loc_y + scale_ty as f64)),
            Some(Size::from((
                (layer_w as f32 * scale).round() as i32,
                (layer_h as f32 * scale).round() as i32,
            ))),
            p,
        )
    }

    /// The compositor's own pointer: an arrow with a contrasting outline so it
    /// stays visible on any surface. Cached per scheme.
    pub fn render_cursor(&mut self, theme: &Theme) -> MemoryRenderBuffer {
        const W: u32 = 30;
        const H: u32 = 34;
        let dark = theme.is_dark;
        if let Some((d, buf)) = &self.cursor_cache {
            if *d == dark {
                return buf.clone();
            }
        }
        // Arrow outline, hotspot at the top-left tip.
        const ARROW: &[(f32, f32)] = &[
            (1.5, 1.5),
            (1.5, 24.0),
            (7.0, 18.0),
            (11.0, 27.5),
            (15.5, 25.5),
            (11.5, 16.0),
            (19.5, 16.0),
        ];
        let (fill, outline) = if dark {
            (
                tiny_skia::Color::from_rgba8(0xff, 0xff, 0xff, 0xff),
                tiny_skia::Color::from_rgba8(0x11, 0x0f, 0x14, 0xff),
            )
        } else {
            (
                tiny_skia::Color::from_rgba8(0x1d, 0x1b, 0x20, 0xff),
                tiny_skia::Color::from_rgba8(0xff, 0xff, 0xff, 0xff),
            )
        };

        let mut s = Surface::new(W, H);
        let shadow: Vec<(f32, f32)> = ARROW.iter().map(|&(x, y)| (x + 1.2, y + 1.4)).collect();
        s.fill_path(
            &crate::render::polygon_path(&shadow),
            tiny_skia::Color::from_rgba8(0, 0, 0, 90),
        );
        let path = crate::render::polygon_path(ARROW);
        s.stroke_path(&path, 2.6, outline);
        s.fill_path(&path, fill);
        let buf = s.to_memory_buffer();
        self.cursor_cache = Some((dark, buf.clone()));
        buf
    }

    pub fn render_decoration(
        &mut self,
        theme: &Theme,
        fonts: &mut Fonts,
        window: &Window,
        window_geom: Rectangle<i32, Logical>,
        params: DecorParams,
        mask: Option<CircleMask>,
    ) -> (MemoryRenderBuffer, Point<f64, Logical>) {
        let DecorParams {
            meta,
            focused,
            maximized,
        } = params;
        let l = decor_layout(window_geom);
        let loc = Point::from((
            l.frame.x as f64 - SHADOW_PAD as f64,
            l.frame.y as f64 - SHADOW_PAD as f64,
        ));
        // Only the *size* affects the painted pixels; the position is applied
        // by the caller through the returned `loc`. Including it here would
        // invalidate (and re-rasterise) the frame on every drag motion.
        let sig = KeyHash::new()
            .i32(window_geom.size.w)
            .i32(window_geom.size.h)
            .str(&meta.title)
            .str(&meta.app_id)
            .bool(focused)
            .bool(maximized)
            .u64(theme.cache_key())
            .finish();
        if let Some(entries) = self.decor_cache.get_mut(window) {
            if let Some(entry) = entries.iter_mut().find(|e| e.key == sig) {
                entry.epoch = self.frame_epoch;
                return masked_from(&entry.surface, mask, &entry.buffer, loc);
            }
        }
        let pw = l.frame.w.ceil() as u32 + 2 * SHADOW_PAD as u32;
        let ph = l.frame.h.ceil() as u32 + 2 * SHADOW_PAD as u32;
        let mut s = Surface::new(pw, ph);
        // Soft elevation shadow under the floating frame.
        s.shadow_round_rect(
            SHADOW_PAD,
            SHADOW_PAD,
            l.frame.w,
            l.frame.h,
            config::FRAME_RADIUS,
            SHADOW_SPREAD,
        );

        // Focus raises the frame to a higher Material elevation.
        let surface = theme.surface_for_elevation(if focused { 3 } else { 2 });
        let on = if focused {
            theme.on_surface
        } else {
            Theme::with_alpha(theme.on_surface, 0.7)
        };

        s.fill_round_rect(
            SHADOW_PAD,
            SHADOW_PAD,
            l.frame.w,
            l.frame.h,
            config::FRAME_RADIUS,
            surface,
        );
        s.stroke_round_rect(
            SHADOW_PAD + 0.5,
            SHADOW_PAD + 0.5,
            l.frame.w - 1.0,
            l.frame.h - 1.0,
            config::FRAME_RADIUS,
            1.0,
            Theme::with_alpha(theme.outline, if focused { 0.35 } else { 0.2 }),
        );

        // Window controls: minimize, maximize/restore, close.
        let cc = (
            l.close.0 - l.frame.x + SHADOW_PAD,
            l.close.1 - l.frame.y + SHADOW_PAD,
        );
        let mc = (
            l.maximize.0 - l.frame.x + SHADOW_PAD,
            l.maximize.1 - l.frame.y + SHADOW_PAD,
        );
        let nc = (
            l.minimize.0 - l.frame.x + SHADOW_PAD,
            l.minimize.1 - l.frame.y + SHADOW_PAD,
        );

        let neutral = theme.surface_container_highest;
        let neutral_on = theme.on_surface_variant;
        s.circle(nc.0, nc.1, 13.0, neutral);
        fonts.draw_icon_centered(&mut s, nc.0, nc.1, "remove", 16.0, neutral_on);
        s.circle(mc.0, mc.1, 13.0, neutral);
        fonts.draw_icon_centered(
            &mut s,
            mc.0,
            mc.1,
            if maximized { "filter_none" } else { "crop_square" },
            14.0,
            neutral_on,
        );
        s.circle(cc.0, cc.1, 13.0, Theme::with_alpha(theme.error, 0.9));
        fonts.draw_icon_centered(&mut s, cc.0, cc.1, "close", 16.0, theme.on_error);

        let key = if meta.app_id.is_empty() {
            meta.title.as_str()
        } else {
            meta.app_id.as_str()
        };
        let mid_y = SHADOW_PAD + l.titlebar.h / 2.0;
        fonts.draw_icon_centered(
            &mut s,
            SHADOW_PAD + 26.0,
            mid_y,
            config::icon_for_key(key),
            20.0,
            on,
        );
        // The focused window's title carries extra weight.
        let title_style = if focused {
            TextStyle {
                kind: FontKind::Bold,
                ..TextStyle::TITLE_SMALL
            }
        } else {
            TextStyle::TITLE_SMALL
        };
        fonts.draw_text_role(
            &mut s,
            SHADOW_PAD + 46.0,
            mid_y - title_style.line_height / 2.0,
            title_style,
            &truncate(&meta.title, 42),
            on,
        );

        let buffer = s.to_memory_buffer();
        let result = masked_from(&s, mask, &buffer, loc);
        let epoch = self.frame_epoch;
        let entries = self.decor_cache.entry(window.clone()).or_default();
        entries.push(DecorEntry {
            key: sig,
            surface: s,
            buffer,
            epoch,
        });
        if entries.len() > 4 {
            entries.remove(0);
        }
        result
    }
}

/// Either the cached buffer or, during a radial reveal, a cropped copy of the
/// surface with everything outside `mask` erased. `loc` is the element's
/// location for the uncropped case; the returned location follows the crop.
fn masked_from(
    surface: &Surface,
    mask: Option<CircleMask>,
    cached: &MemoryRenderBuffer,
    loc: Point<f64, Logical>,
) -> (MemoryRenderBuffer, Point<f64, Logical>) {
    match mask {
        None => (cached.clone(), loc),
        Some(m) => {
            // Once the reveal circle covers the whole layer the mask is a
            // no-op; skipping it avoids cloning the full pixmap and building a
            // full-size coverage mask on the final frames of the animation.
            let w = surface.width() as f32;
            let h = surface.pixmap.height() as f32;
            let cx = m.cx - loc.x as f32;
            let cy = m.cy - loc.y as f32;
            let dx = cx.max(w - cx).max(0.0);
            let dy = cy.max(h - cy).max(0.0);
            if dx * dx + dy * dy <= m.radius * m.radius {
                return (cached.clone(), loc);
            }
            let (cropped, (ox, oy)) = surface.masked_circle_cropped(m.cx, m.cy, m.radius);
            (
                cropped.to_memory_buffer(),
                Point::from((loc.x + ox as f64, loc.y + oy as f64)),
            )
        }
    }
}

// ---- Layout -------------------------------------------------------------

pub fn topbar_rect(win_w: f32) -> RectF {
    RectF {
        x: 0.0,
        y: 0.0,
        w: win_w,
        h: config::TOPBAR_HEIGHT,
    }
}

/// Screen rectangle of one top-bar icon button; render and hit-testing share
/// this layout so a press always lands on the drawn control.
pub fn topbar_button_rect(win_w: f32, action: TopAction) -> RectF {
    let rect = topbar_rect(win_w);
    let btn = config::TOPBAR_HEIGHT;
    let x = match action {
        TopAction::Launcher => rect.x,
        TopAction::Power => rect.right() - 8.0 - btn,
        TopAction::Theme => rect.right() - 8.0 - 2.0 * btn - TOPBAR_ICON_GAP,
        TopAction::Brightness => rect.right() - 8.0 - 3.0 * btn - 2.0 * TOPBAR_ICON_GAP,
        TopAction::QuickSettings => rect.right() - 8.0 - 4.0 * btn - 3.0 * TOPBAR_ICON_GAP,
    };
    RectF {
        x,
        y: rect.y,
        w: btn,
        h: btn,
    }
}

pub fn dock_rect(win_w: f32, win_h: f32, _items: usize) -> RectF {
    RectF {
        x: 0.0,
        y: win_h - config::DOCK_HEIGHT,
        w: win_w,
        h: config::DOCK_HEIGHT,
    }
}

/// Screen rectangle of one dock icon; the launcher grows out of this.
pub fn dock_item_rect(win_w: f32, win_h: f32, items: usize, index: usize) -> RectF {
    let rect = dock_rect(win_w, win_h, items);
    let x = dock_row_start(rect.w, items) + index as f32 * (config::DOCK_ICON + config::DOCK_GAP);
    RectF {
        x,
        y: rect.y + (rect.h - config::DOCK_ICON) / 2.0,
        w: config::DOCK_ICON,
        h: config::DOCK_ICON,
    }
}

/// Left edge of the centred dock icon row.
fn dock_row_start(win_w: f32, items: usize) -> f32 {
    let total = items as f32 * config::DOCK_ICON
        + items.saturating_sub(1) as f32 * config::DOCK_GAP;
    ((win_w - total) / 2.0).max(DOCK_PAD)
}

pub fn brightness_popup(win_w: f32) -> RectF {
    let rect = topbar_rect(win_w);
    let w = 260.0_f32.min((rect.w - 16.0).max(0.0)).max(160.0);
    let h = 64.0;
    RectF {
        x: rect.right() - 8.0 - w,
        y: rect.bottom() + 8.0,
        w,
        h,
    }
}

/// Height of one session-menu row.
pub const POWER_ROW_H: f32 = 56.0;
/// Gap between session-menu rows.
pub const POWER_ROW_GAP: f32 = 4.0;
/// Padding between the session-menu card edge and its rows.
pub const POWER_MENU_PAD: f32 = 8.0;

/// Card of the session menu, anchored under the top-bar user button.
pub fn power_menu_rect(win_w: f32) -> RectF {
    let bar = topbar_rect(win_w);
    let w = 232.0_f32.min((bar.w - 16.0).max(0.0)).max(180.0);
    let rows = POWER_ACTIONS.len() as f32;
    let h = 2.0 * POWER_MENU_PAD + rows * POWER_ROW_H + (rows - 1.0) * POWER_ROW_GAP;
    RectF {
        x: bar.right() - 8.0 - w,
        y: bar.bottom() + 8.0,
        w,
        h,
    }
}

/// Screen rectangle of one session-menu row.
pub fn power_row_rect(win_w: f32, index: usize) -> RectF {
    let card = power_menu_rect(win_w);
    RectF {
        x: card.x + POWER_MENU_PAD,
        y: card.y + POWER_MENU_PAD + index as f32 * (POWER_ROW_H + POWER_ROW_GAP),
        w: card.w - 2.0 * POWER_MENU_PAD,
        h: POWER_ROW_H,
    }
}

/// Index of the session-menu row under `pos`, if any.
pub fn power_row_at(win_w: f32, pos: Point<f64, Logical>) -> Option<usize> {
    (0..POWER_ACTIONS.len()).find(|i| power_row_rect(win_w, *i).contains(pos))
}

pub fn quick_settings_rect(win_w: f32, win_h: f32) -> RectF {
    let bar = topbar_rect(win_w);
    let w = (win_w - 24.0).clamp(160.0, 376.0);
    let h = QS_SHEET_HEIGHT.min((win_h - bar.bottom() - 20.0).max(120.0));
    RectF {
        x: (bar.right() - w).max(12.0),
        y: (bar.bottom() + 8.0).min((win_h - h - 12.0).max(12.0)),
        w,
        h,
    }
}

fn quick_setting_tile(rect: RectF, index: usize, progress: f32) -> RectF {
    let progress = progress.clamp(0.0, 1.0);
    let dy = -(1.0 - progress) * 14.0;
    let tile_w = ((rect.w - 44.0) / 2.0).max(56.0);
    let tile_h = 72.0;
    let x = rect.x + if index.is_multiple_of(2) { 16.0 } else { 16.0 + tile_w + 12.0 };
    let y = rect.y + 48.0 + dy + if index < 2 { 0.0 } else { tile_h + 12.0 };
    RectF { x, y, w: tile_w, h: tile_h }
}

/// Screen rectangle of one quick-settings tile at rest.
pub fn quick_setting_tile_rect(win_w: f32, win_h: f32, index: usize) -> RectF {
    quick_setting_tile(quick_settings_rect(win_w, win_h), index, 1.0)
}

pub fn quick_settings_hit(
    win_w: f32,
    win_h: f32,
    pos: Point<f64, Logical>,
    progress: f32,
) -> Option<QuickAction> {
    let rect = quick_settings_rect(win_w, win_h);
    let actions = [
        QuickAction::Theme,
        QuickAction::NightLight,
        QuickAction::ReduceMotion,
        QuickAction::Brightness,
    ];
    actions.iter().enumerate().find_map(|(i, action)| {
        quick_setting_tile(rect, i, progress).contains(pos).then_some(*action)
    })
}

/// Palette chip rectangle inside the sheet.
fn palette_chip_rect_in(rect: RectF, index: usize) -> RectF {
    let cw = (rect.w - 2.0 * QS_CHIP_MARGIN - 2.0 * QS_CHIP_GAP) / 3.0;
    let col = index % 3;
    let row = index / 3;
    RectF {
        x: rect.x + QS_CHIP_MARGIN + col as f32 * (cw + QS_CHIP_GAP),
        y: rect.y + QS_CHIPS_Y + row as f32 * (QS_CHIP_H + QS_CHIP_GAP),
        w: cw,
        h: QS_CHIP_H,
    }
}

/// Palette chip rectangle in screen space, or `None` when the sheet is too
/// short for this row (clamped height on small outputs).
pub fn palette_chip_rect(win_w: f32, win_h: f32, index: usize) -> Option<RectF> {
    let rect = quick_settings_rect(win_w, win_h);
    let chip = palette_chip_rect_in(rect, index);
    (chip.bottom() <= rect.bottom() - 6.0).then_some(chip)
}

pub fn palette_chip_hit(win_w: f32, win_h: f32, pos: Point<f64, Logical>) -> Option<usize> {
    (0..VARIANTS.len()).find(|&i| {
        palette_chip_rect(win_w, win_h, i).is_some_and(|rect| rect.contains(pos))
    })
}

/// Draw a Material 3 switch whose handle is driven by a spring (`t` may
/// overshoot slightly, which is the M3 Expressive bounce).
fn draw_m3_switch(s: &mut Surface, x: f32, y: f32, t: f32, theme: &Theme) {
    let t = t.max(0.0);
    let curve = t.clamp(0.0, 1.0);
    let track = Theme::mix(theme.surface_container_highest, theme.primary, curve);
    let handle_color = Theme::mix(theme.outline, theme.on_primary, curve);
    s.fill_round_rect(x, y, 52.0, 32.0, 16.0, track);
    if curve < 0.999 {
        s.stroke_round_rect(
            x + 0.5,
            y + 0.5,
            51.0,
            31.0,
            16.0,
            1.0,
            Theme::with_alpha(theme.outline, 1.0 - curve),
        );
    }
    let cx = x + 16.0 + 20.0 * t.clamp(0.0, 1.06);
    let radius = 8.0 + 4.0 * curve;
    s.circle(cx, y + 16.0, radius, handle_color);
}

pub fn brightness_track(win_w: f32) -> RectF {
    let card = brightness_popup(win_w);
    let left = card.x + 56.0;
    let right = card.right() - 20.0;
    RectF {
        x: left,
        y: card.y,
        w: (right - left).max(1.0),
        h: card.h,
    }
}

pub fn launcher_card(win_w: f32, win_h: f32) -> RectF {
    let w = config::LAUNCHER_WIDTH.min(win_w - 96.0).max(320.0);
    let h = config::LAUNCHER_MAX_HEIGHT.min(win_h - 120.0).max(240.0);
    RectF {
        x: (win_w - w) / 2.0,
        y: (win_h - h) / 2.0 - 24.0,
        w,
        h,
    }
}

pub fn search_rect(win_w: f32, win_h: f32) -> RectF {
    let card = launcher_card(win_w, win_h);
    RectF {
        x: card.x + 20.0,
        y: card.y + 20.0,
        w: card.w - 40.0,
        h: SEARCH_HEIGHT,
    }
}

pub fn launcher_columns(card_w: f32) -> usize {
    let usable = card_w - 2.0 * GRID_PAD + GRID_GAP;
    ((usable / (config::LAUNCHER_TILE + GRID_GAP)).floor() as usize).max(1)
}

pub fn launcher_grid_origin(win_w: f32, win_h: f32) -> (f32, f32) {
    let card = launcher_card(win_w, win_h);
    let search = search_rect(win_w, win_h);
    (card.x + GRID_PAD, search.bottom() + 16.0)
}

pub fn launcher_tile_pos(origin: (f32, f32), slot: usize, cols: usize) -> (f32, f32) {
    let col = slot % cols;
    let row = slot / cols;
    let x = origin.0 + col as f32 * (config::LAUNCHER_TILE + GRID_GAP);
    let y = origin.1 + row as f32 * (config::LAUNCHER_TILE + GRID_GAP);
    (x, y)
}

pub fn decor_layout(window: Rectangle<i32, Logical>) -> DecorLayout {
    let content = RectF {
        x: window.loc.x as f32,
        y: window.loc.y as f32,
        w: window.size.w as f32,
        h: window.size.h as f32,
    };
    let frame = RectF {
        x: content.x - config::FRAME_BORDER,
        y: content.y - config::FRAME_TITLEBAR,
        w: content.w + 2.0 * config::FRAME_BORDER,
        h: content.h + config::FRAME_TITLEBAR + config::FRAME_BORDER,
    };
    let titlebar = RectF {
        x: frame.x,
        y: frame.y,
        w: frame.w,
        h: config::FRAME_TITLEBAR,
    };
    let btn_r = 15.0;
    let btn_gap = 36.0;
    let cy = frame.y + config::FRAME_TITLEBAR / 2.0;
    let close_x = frame.right() - btn_r - 8.0;
    let maximize = (close_x - btn_gap, cy);
    let minimize = (close_x - 2.0 * btn_gap, cy);
    let close = (close_x, cy);
    DecorLayout {
        frame,
        content,
        titlebar,
        minimize,
        maximize,
        close,
    }
}

// ---- Hit testing --------------------------------------------------------

pub fn topbar_hit(win_w: f32, pos: Point<f64, Logical>) -> Option<TopAction> {
    let rect = topbar_rect(win_w);
    if !rect.contains(pos) {
        return None;
    }
    [
        TopAction::Launcher,
        TopAction::QuickSettings,
        TopAction::Brightness,
        TopAction::Theme,
        TopAction::Power,
    ]
    .into_iter()
    .find(|action| topbar_button_rect(win_w, *action).contains(pos))
}

/// Map a pointer position inside the brightness popup to a brightness value.
pub fn brightness_hit(win_w: f32, pos: Point<f64, Logical>) -> Option<f32> {
    let card = brightness_popup(win_w);
    if !card.contains(pos) {
        return None;
    }
    let track = brightness_track(win_w);
    let t = ((pos.x as f32 - track.x) / track.w).clamp(0.0, 1.0);
    Some(BRIGHTNESS_MIN + t * (BRIGHTNESS_MAX - BRIGHTNESS_MIN))
}

pub fn dock_hit(
    win_w: f32,
    win_h: f32,
    items: usize,
    pos: Point<f64, Logical>,
) -> Option<usize> {
    let rect = dock_rect(win_w, win_h, items);
    if !rect.contains(pos) {
        return None;
    }
    let (px, _) = (pos.x as f32, pos.y as f32);
    let mut x = dock_row_start(rect.w, items);
    for i in 0..items {
        if px >= x && px <= x + config::DOCK_ICON {
            return Some(i);
        }
        x += config::DOCK_ICON + config::DOCK_GAP;
    }
    None
}

pub fn launcher_hit(
    win_w: f32,
    win_h: f32,
    filtered_count: usize,
    pos: Point<f64, Logical>,
) -> LauncherHit {
    let card = launcher_card(win_w, win_h);
    if !card.contains(pos) {
        return LauncherHit::Scrim;
    }
    if search_rect(win_w, win_h).contains(pos) {
        return LauncherHit::Input;
    }
    let cols = launcher_columns(card.w);
    let origin = launcher_grid_origin(win_w, win_h);
    for slot in 0..filtered_count {
        let (tx, ty) = launcher_tile_pos(origin, slot, cols);
        // Same cutoff as `render_launcher`: only tiles that fit the card exist.
        if ty + config::LAUNCHER_TILE > card.bottom() - 16.0 {
            break;
        }
        let tile = RectF {
            x: tx,
            y: ty,
            w: config::LAUNCHER_TILE,
            h: config::LAUNCHER_TILE,
        };
        if tile.contains(pos) {
            return LauncherHit::Tile(slot);
        }
    }
    LauncherHit::Inside
}

pub fn decor_hit(l: &DecorLayout, pos: Point<f64, Logical>) -> DecorHit {
    let (px, py) = (pos.x as f32, pos.y as f32);
    let near = |c: (f32, f32)| {
        let (dx, dy) = (px - c.0, py - c.1);
        dx * dx + dy * dy <= 16.0 * 16.0
    };
    if near(l.close) {
        return DecorHit::Close;
    }
    if near(l.maximize) {
        return DecorHit::Maximize;
    }
    if near(l.minimize) {
        return DecorHit::Minimize;
    }
    // Outer resize bands take precedence over the title bar so every edge is
    // grabbable, including the top one (which otherwise reads as a move).
    let band = 6.0;
    if l.frame.contains(pos) {
        let mut e = ResizeEdges::default();
        if px <= l.frame.x + band {
            e.left = true;
        }
        if px >= l.frame.right() - band {
            e.right = true;
        }
        if py <= l.frame.y + band {
            e.top = true;
        }
        if py >= l.frame.bottom() - band {
            e.bottom = true;
        }
        if e != ResizeEdges::default() {
            return DecorHit::Resize(e);
        }
    }
    if l.titlebar.contains(pos) && !l.content.contains(pos) {
        return DecorHit::Move;
    }
    DecorHit::None
}

/// Draw a Material 3 state layer over a circular control.
fn state_layer_overlay(
    s: &mut Surface,
    theme: &Theme,
    cx: f32,
    cy: f32,
    r: f32,
    hovered: bool,
    pressed: bool,
) {
    let opacity = if pressed {
        state::PRESSED
    } else if hovered {
        state::HOVER
    } else {
        return;
    };
    s.circle(cx, cy, r, theme.state_layer(theme.on_surface, opacity));
}

/// Shorten `s` to at most `max` characters, adding an ellipsis when clipped.
fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        return s.to_string();
    }
    if max == 0 {
        return String::new();
    }
    let mut out: String = s.chars().take(max - 1).collect();
    out.push('…');
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use smithay::utils::Size;

    fn pt(x: f64, y: f64) -> Point<f64, Logical> {
        Point::from((x, y))
    }

    fn app(name: &str) -> AppEntry {
        AppEntry {
            name: name.into(),
            exec: "x".into(),
            icon: String::new(),
            terminal: false,
            desktop_file: None,
        }
    }

    #[test]
    fn pinned_has_exactly_one_terminal() {
        let s = Shell::new(vec![], ThemeSpec::DEFAULT);
        assert_eq!(s.pinned.len(), 1);
        let s = Shell::new(vec![app("Foot Terminal"), app("Firefox")], ThemeSpec::DEFAULT);
        assert_eq!(s.pinned.len(), 1);
        assert_eq!(s.pinned[0].name, "Foot Terminal");
    }

    #[test]
    fn terminal_command_follows_discovered_app() {
        let s = Shell::new(vec![app("Kitty Terminal"), app("Firefox")], ThemeSpec::DEFAULT);
        assert_eq!(s.terminal, vec!["x".to_string()]);
        let s = Shell::new(vec![], ThemeSpec::DEFAULT);
        assert_eq!(s.terminal, vec!["kitty".to_string()]);
    }

    #[test]
    fn kitty_wins_over_other_terminals() {
        let s = Shell::new(
            vec![app("Foot Terminal"), app("kitty"), app("Firefox")],
            ThemeSpec::DEFAULT,
        );
        assert_eq!(s.pinned[0].name, "kitty");
        let s = Shell::new(
            vec![app("Foot Terminal"), app("Alacritty"), app("Firefox")],
            ThemeSpec::DEFAULT,
        );
        assert_eq!(s.pinned[0].name, "Alacritty");
    }

    #[test]
    fn session_menu_rows_hit_their_actions() {
        let w = 1000.0;
        for i in 0..POWER_ACTIONS.len() {
            let row = power_row_rect(w, i);
            let center = pt(
                (row.x + row.w / 2.0) as f64,
                (row.y + row.h / 2.0) as f64,
            );
            assert_eq!(power_row_at(w, center), Some(i));
        }
        let card = power_menu_rect(w);
        assert_eq!(power_row_at(w, pt((card.x + 2.0) as f64, (card.y + 2.0) as f64)), None);
        assert_eq!(power_row_at(w, pt(500.0, 400.0)), None);
    }

    #[test]
    fn session_menu_sits_below_the_user_button() {
        let w = 1000.0;
        let bar = topbar_rect(w);
        let button = topbar_button_rect(w, TopAction::Power);
        let card = power_menu_rect(w);
        assert!(card.y >= bar.bottom());
        assert!((card.right() - button.right()).abs() < 0.5, "right edge follows the button");
        assert!(card.h >= POWER_ACTIONS.len() as f32 * POWER_ROW_H);
    }

    #[test]
    fn topbar_hit_buttons() {
        let w = 1000.0;
        assert_eq!(topbar_hit(w, pt(30.0, 30.0)), Some(TopAction::Launcher));
        assert_eq!(topbar_hit(w, pt(960.0, 30.0)), Some(TopAction::Power));
        assert_eq!(topbar_hit(w, pt(910.0, 30.0)), Some(TopAction::Theme));
        assert_eq!(topbar_hit(w, pt(860.0, 30.0)), Some(TopAction::Brightness));
        assert_eq!(topbar_hit(w, pt(500.0, 30.0)), None);
        assert_eq!(topbar_hit(w, pt(300.0, 5.0)), None);
        assert_eq!(topbar_hit(w, pt(30.0, 40.0)), None, "below the bar");
    }

    #[test]
    fn quick_settings_tiles_have_actionable_hit_regions() {
        let (w, h) = (1000.0, 800.0);
        let card = quick_settings_rect(w, h);
        let tile_w = (card.w - 44.0) / 2.0;
        let theme_center = pt((card.x + 16.0 + tile_w / 2.0) as f64, (card.y + 48.0 + 36.0) as f64);
        let night_center = pt((card.x + 16.0 + tile_w + 12.0 + tile_w / 2.0) as f64, (card.y + 48.0 + 36.0) as f64);
        assert_eq!(quick_settings_hit(w, h, theme_center, 1.0), Some(QuickAction::Theme));
        assert_eq!(quick_settings_hit(w, h, night_center, 1.0), Some(QuickAction::NightLight));
    }

    #[test]
    fn palette_chips_map_to_variants() {
        let (w, h) = (1000.0, 800.0);
        for i in 0..VARIANTS.len() {
            let chip = palette_chip_rect(w, h, i)
                .unwrap_or_else(|| panic!("chip {i} must be reachable"));
            let center = pt((chip.x + chip.w / 2.0) as f64, (chip.y + chip.h / 2.0) as f64);
            assert_eq!(palette_chip_hit(w, h, center), Some(i));
        }
    }

    #[test]
    fn quick_settings_sheet_fits_palette_grid() {
        let (w, h) = (1000.0, 800.0);
        let rect = quick_settings_rect(w, h);
        assert!(rect.h >= QS_SHEET_HEIGHT - 1.0);
        let last = palette_chip_rect(w, h, VARIANTS.len() - 1).expect("last chip fits");
        assert!(last.bottom() <= rect.bottom());
    }

    #[test]
    fn brightness_hit_maps_and_clamps() {
        let w = 1000.0;
        let card = brightness_popup(w);
        let track = brightness_track(w);
        let cy = (card.y + card.h / 2.0) as f64;
        let lo = brightness_hit(w, pt(track.x as f64, cy)).unwrap();
        let hi = brightness_hit(w, pt(track.right() as f64, cy)).unwrap();
        assert!((lo - BRIGHTNESS_MIN).abs() < 1e-4, "left edge is dimmest");
        assert!((hi - BRIGHTNESS_MAX).abs() < 1e-4, "right edge is brightest");
        // A point outside the card is not a hit.
        assert!(brightness_hit(w, pt(10.0, 500.0)).is_none());
    }

    #[test]
    fn dock_hit_indexes() {
        let (w, h) = (1000.0, 600.0);
        assert_eq!(dock_hit(w, h, 2, pt(460.0, 540.0)), Some(0));
        assert_eq!(dock_hit(w, h, 2, pt(520.0, 540.0)), Some(1));
        assert_eq!(dock_hit(w, h, 2, pt(500.0, 540.0)), None);
        assert_eq!(dock_hit(w, h, 2, pt(460.0, 100.0)), None);
    }

    #[test]
    fn dock_item_rect_matches_hit() {
        let (w, h) = (1000.0, 600.0);
        for i in 0..3 {
            let rect = dock_item_rect(w, h, 3, i);
            let center = pt((rect.x + rect.w / 2.0) as f64, (rect.y + rect.h / 2.0) as f64);
            assert_eq!(dock_hit(w, h, 3, center), Some(i));
        }
    }

    #[test]
    fn launcher_hit_regions() {
        let (w, h) = (1000.0, 700.0);
        assert_eq!(launcher_hit(w, h, 30, pt(10.0, 10.0)), LauncherHit::Scrim);
        assert_eq!(launcher_hit(w, h, 30, pt(500.0, 90.0)), LauncherHit::Input);
        assert_eq!(launcher_hit(w, h, 1, pt(200.0, 190.0)), LauncherHit::Tile(0));
        assert_eq!(launcher_hit(w, h, 30, pt(200.0, 590.0)), LauncherHit::Inside);
    }

    #[test]
    fn decor_hit_regions() {
        let geom = Rectangle::new(Point::from((100, 200)), Size::from((400, 300)));
        let l = decor_layout(geom);
        assert_eq!(decor_hit(&l, pt(479.0, 178.0)), DecorHit::Close);
        assert_eq!(decor_hit(&l, pt(300.0, 175.0)), DecorHit::Move);
        assert!(matches!(decor_hit(&l, pt(102.0, 350.0)), DecorHit::Resize(e) if e.left));
        assert!(matches!(decor_hit(&l, pt(497.0, 350.0)), DecorHit::Resize(e) if e.right));
        assert!(matches!(decor_hit(&l, pt(300.0, 496.0)), DecorHit::Resize(e) if e.bottom));
        assert!(
            matches!(decor_hit(&l, pt(300.0, 170.0)), DecorHit::Resize(e) if e.top),
            "the top edge must be resizable, not only movable"
        );
        assert_eq!(decor_hit(&l, pt(300.0, 350.0)), DecorHit::None);
    }

    #[test]
    fn press_visuals_animate_and_settle() {
        let mut shell = Shell::new(vec![], ThemeSpec::DEFAULT);
        let key = PressKey::Top(TopAction::Theme);
        shell.press_begin(key, (10.0, 10.0), 12.0, 0.86);
        // The scale spring is already aiming at the pressed value, so one tick
        // is enough to move it off the rest pose.
        shell.tick(1.0 / 60.0);
        assert!(shell.press_scale(key) < 1.0, "press squeezes the control");
        assert!(shell.ripple_draw(key).is_some(), "press starts a ripple");
        shell.press_end(key);
        let mut steps = 0;
        while !shell.press.is_empty() {
            shell.tick(1.0 / 60.0);
            steps += 1;
            assert!(steps < 600, "press animation must settle");
        }
        assert_eq!(shell.press_scale(key), 1.0);
        assert!(shell.ripple_draw(key).is_none());
    }

    #[test]
    fn toggle_night_light_animates_the_switch() {
        let mut shell = Shell::new(vec![], ThemeSpec::DEFAULT);
        assert!(shell.switch_spring(QuickAction::NightLight) < 0.5);
        shell.toggle_night_light();
        for _ in 0..180 {
            shell.tick(1.0 / 60.0);
        }
        assert!(shell.switch_spring(QuickAction::NightLight) > 0.99);
    }

    #[test]
    fn reduce_motion_snaps_the_switch() {
        let mut shell = Shell::new(vec![], ThemeSpec::DEFAULT);
        shell.set_reduce_motion(true);
        shell.toggle_night_light();
        assert_eq!(shell.switch_spring(QuickAction::NightLight), 1.0);
        assert!(!shell.tick(1.0 / 60.0), "nothing may be left animating");
    }

    #[test]
    fn reduce_motion_engages_its_own_switch() {
        let mut shell = Shell::new(vec![], ThemeSpec::DEFAULT);
        assert!(shell.switch_spring(QuickAction::ReduceMotion) < 0.5);
        shell.set_reduce_motion(true);
        assert_eq!(
            shell.switch_spring(QuickAction::ReduceMotion),
            1.0,
            "the tile's switch must show the setting as on"
        );
        shell.set_reduce_motion(false);
        for _ in 0..240 {
            shell.tick(1.0 / 60.0);
        }
        assert!(
            shell.switch_spring(QuickAction::ReduceMotion) < 0.01,
            "switching back off must animate the switch down"
        );
    }

    #[test]
    fn theme_fade_returns_both_endpoints_only_while_running() {
        let mut shell = Shell::new(vec![], ThemeSpec::DEFAULT);
        let old = crate::theme::ThemeSpec::DEFAULT.dark();
        assert!(shell.theme_fade().is_none(), "no fade when idle");
        shell.begin_theme_change(old);
        let (base, alpha) = shell.theme_fade().expect("fade while running");
        assert_eq!(base.cache_key(), old.cache_key(), "base is the old scheme");
        assert!(alpha < 1.0, "the new scheme starts fully transparent");
        for _ in 0..240 {
            shell.tick(1.0 / 60.0);
        }
        assert!(shell.theme_fade().is_none(), "the fade must end");
    }

    #[test]
    fn truncate_adds_ellipsis() {
        assert_eq!(truncate("hello", 10), "hello");
        assert_eq!(truncate("hello world", 8), "hello w…");
        assert_eq!(truncate("abc", 0), "");
    }

    #[test]
    fn launcher_columns_positive() {
        assert!(launcher_columns(720.0) >= 1);
        assert!(launcher_columns(10.0) >= 1);
    }

    #[test]
    fn launcher_layer_caches_across_the_animation() {
        let mut shell = Shell::new(vec![], ThemeSpec::DEFAULT);
        let mut fonts = crate::font::Fonts::load().expect("test system font");
        let theme = crate::theme::ThemeSpec::DEFAULT.dark();
        let (w, h) = (1280.0, 800.0);
        let _ = shell.render_launcher(&theme, &mut fonts, w, h, 1.0, None);
        let key = shell
            .launcher_layer
            .as_ref()
            .expect("the layer is cached on the first render")
            .key;
        // Halfway through the open animation no press is active, so the very
        // same content layer must be reused instead of being re-rasterised
        // (which used to cost ~24 ms per frame).
        let _ = shell.render_launcher(&theme, &mut fonts, w, h, 0.5, None);
        assert_eq!(
            shell.launcher_layer.as_ref().unwrap().key,
            key,
            "the launcher content layer must be cached across the animation"
        );
    }

    #[test]
    fn launcher_frame_scales_and_scrims_on_the_gpu_path() {
        let mut shell = Shell::new(vec![], ThemeSpec::DEFAULT);
        let mut fonts = crate::font::Fonts::load().expect("test system font");
        let theme = crate::theme::ThemeSpec::DEFAULT.dark();
        let (w, h) = (1280.0, 800.0);
        let frame = shell.render_launcher(&theme, &mut fonts, w, h, 1.0, None);
        // Scrim (stretched over the output) plus the scaled card layer.
        assert!(frame.pieces.len() >= 2);
        assert!(frame.pieces[0].0.size.is_some(), "the scrim is stretched");
        assert!(
            frame.pieces[1].0.size.is_some(),
            "the card layer carries a size override"
        );
        assert!((frame.pieces[1].1 - 1.0).abs() < 1e-3, "fully open");
    }
}
