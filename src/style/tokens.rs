/// Material 3 shape scale in logical dp.
pub mod shape {
    pub const MEDIUM: f32 = 12.0;
    /// Window frame and floating popups.
    pub const LARGE: f32 = 16.0;
    /// Launcher dialog.
    pub const EXTRA_LARGE: f32 = 28.0;
    pub const EXTRA_EXTRA_LARGE: f32 = 48.0;
}

/// Material 3 state-layer opacities, applied over the appropriate `on_*` role.
pub mod state {
    pub const HOVER: f32 = 0.08;
    pub const PRESSED: f32 = 0.10;
}
