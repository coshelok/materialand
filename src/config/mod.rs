use std::collections::HashSet;
use std::path::{Path, PathBuf};

/// Keyboard key repeat as announced to clients through
/// `wl_keyboard.repeat_info`: how long a key must be held before the first
/// repeat, in milliseconds.
pub const KEY_REPEAT_DELAY_MS: i32 = 500;

/// Repeats per second after `KEY_REPEAT_DELAY_MS`. Clients implement the
/// repeat themselves; the compositor only advertises the setting.
pub const KEY_REPEAT_RATE_HZ: i32 = 25;

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

mod app_identity;
mod pins;
pub use pins::{pins_path, Pins};
mod workspace;
pub use workspace::{Anchor, Shortcut, Workspace};
mod appearance;
mod storage;
pub use appearance::{appearance_path, Appearance, AppearanceStore, AppearanceWriter};
mod design;
pub use design::check_file as check_design_file;
pub use design::compact_file as compact_config_file;
pub use design::initialize_file as initialize_design_file;
pub use design::{built_in_design, design, initialize_design, DEFAULT_CONFIG};
