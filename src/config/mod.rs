use std::collections::HashSet;
use std::path::{Path, PathBuf};

/// Nominal physical size of the output in millimeters.
pub const OUTPUT_PHYSICAL_MM: (i32, i32) = (370, 240);

/// Default logical size of a new toplevel.
pub const DEFAULT_WINDOW_SIZE: (i32, i32) = (900, 600);

/// Cascade offset when placing successive windows.
pub const CASCADE_STEP: i32 = 36;

/// Keyboard key repeat as announced to clients through
/// `wl_keyboard.repeat_info`: how long a key must be held before the first
/// repeat, in milliseconds.
pub const KEY_REPEAT_DELAY_MS: i32 = 500;

/// Repeats per second after `KEY_REPEAT_DELAY_MS`. Clients implement the
/// repeat themselves; the compositor only advertises the setting.
pub const KEY_REPEAT_RATE_HZ: i32 = 25;

// Layout metrics, in logical (dp) units.

pub const TOPBAR_HEIGHT: f32 = 32.0;
pub const TOPBAR_MARGIN: f32 = 0.0;

pub const DOCK_HEIGHT: f32 = 80.0;
pub const DOCK_MARGIN: f32 = 0.0;
pub const DOCK_ICON: f32 = 56.0;
pub const DOCK_GAP: f32 = 12.0;

pub const FRAME_TITLEBAR: f32 = 32.0;
pub const FRAME_BORDER: f32 = 1.0;
pub const FRAME_RADIUS: f32 = crate::style::shape::EXTRA_LARGE;

pub const LAUNCHER_WIDTH: f32 = 800.0;
pub const LAUNCHER_MAX_HEIGHT: f32 = 640.0;
pub const LAUNCHER_RADIUS: f32 = crate::style::shape::EXTRA_EXTRA_LARGE;
pub const LAUNCHER_TILE: f32 = 112.0;

mod apps;
mod bindings;
mod keyboard;
pub use apps::*;
pub use bindings::*;
pub use keyboard::*;
#[cfg(test)]
mod tests;

mod exec;
use exec::split_exec;
mod terminal;
pub use terminal::*;
