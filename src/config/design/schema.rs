//! Typed view of config/default.toml; defaults are declared only in that file.
use crate::style::TextStyle;
use serde::Deserialize;
use std::collections::BTreeMap;
#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Design {
    pub schema_version: u32,
    pub appearance: Appearance,
    pub shape: Shape,
    pub state: State,
    pub motion: Motion,
    pub easing: Easing,
    pub window: Window,
    pub topbar: Topbar,
    pub dock: Dock,
    pub launcher: Launcher,
    pub apps: Apps,
    pub home: Home,
    pub panels: Panels,
    pub shadow: Shadow,
    pub brightness: Brightness,
    pub power: Power,
    pub quick_settings: QuickSettings,
    pub switch: Switch,
    pub calendar: Calendar,
    pub wallpaper: Wallpaper,
    pub cursor: Cursor,
    pub typography: Typography,
    pub colors: Colors,
    pub app_menu: AppMenu,
    pub fonts: Fonts,
    pub input: Input,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Input {
    pub touchpad: Touchpad,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Touchpad {
    pub invert_scroll: bool,
}
#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Appearance {
    pub theme: String,
    pub variant: String,
    pub seed: String,
    pub wallpaper: String,
    pub reduce_motion: bool,
}
#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Shape {
    pub medium: f32,
    pub large: f32,
    pub extra_large: f32,
    pub extra_extra_large: f32,
}
#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct State {
    pub hover: f32,
    pub pressed: f32,
    pub ripple: f32,
}
#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Motion {
    pub spatial_stiffness: f32,
    pub spatial_damping: f32,
    pub responsive_stiffness: f32,
    pub responsive_damping: f32,
    pub effects_stiffness: f32,
    pub effects_damping: f32,
    pub window_stiffness: f32,
    pub window_damping: f32,
    pub ripple_duration: f32,
    pub reveal_duration: f32,
    pub theme_duration: f32,
    pub overshoot_limit: f32,
    pub panel_initial_scale: f32,
    pub settings_initial_scale: f32,
    pub window_initial_scale: f32,
    pub window_initial_alpha: f32,
    pub window_alpha_gain: f32,
    pub window_close_shrink: f32,
    pub home_offset: f32,
    pub calendar_offset: f32,
    pub press_scale: f32,
    pub ripple_radius_ratio: f32,
}
#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Easing {
    pub standard_decelerate: [f32; 4],
    pub emphasized_decelerate: [f32; 4],
}
#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Window {
    pub width: f32,
    pub height: f32,
    pub cascade_step: f32,
    pub titlebar_height: f32,
    pub border: f32,
    pub button_radius: f32,
    pub button_hit_radius: f32,
    pub button_gap: f32,
    pub button_margin: f32,
    pub resize_band: f32,
    pub focused_tint: f32,
    pub inactive_tint: f32,
    pub workspace_bottom_gap: f32,
    pub control_radius: f32,
    pub control_icon: f32,
    pub maximize_icon: f32,
    pub app_icon_x: f32,
    pub app_icon_size: f32,
    pub title_x: f32,
    pub title_right_padding: f32,
    pub inactive_title_alpha: f32,
    pub initial_spare_width: f32,
    pub initial_spare_height: f32,
    pub spawn_x: f32,
    pub spawn_gap: f32,
    pub edge_margin: f32,
    pub cascade_count: usize,
}
#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Topbar {
    pub height: f32,
    pub margin: f32,
    pub icon_gap: f32,
    pub button_inset: f32,
    pub home_icon_size: f32,
    pub status_icon_size: f32,
    pub button_fill_scale: f32,
    pub clock_gap: f32,
    pub clock_width: f32,
    pub clock_height: f32,
    pub clock_offset_y: f32,
}
#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Dock {
    pub height: f32,
    pub margin: f32,
    pub icon_size: f32,
    pub gap: f32,
    pub padding: f32,
    pub glyph_size: f32,
    pub indicator_active_width: f32,
    pub indicator_width: f32,
    pub indicator_height: f32,
    pub indicator_bottom: f32,
    pub indicator_radius: f32,
    pub feedback_padding: f32,
}
#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Launcher {
    pub width: f32,
    pub max_height: f32,
    pub min_width: f32,
    pub min_height: f32,
    pub margin_x: f32,
    pub margin_y: f32,
    pub offset_y: f32,
    pub tile_size: f32,
    pub search_height: f32,
    pub search_inset_x: f32,
    pub search_inset_y: f32,
    pub grid_gap: f32,
    pub grid_padding: f32,
    pub grid_header_height: f32,
    pub bottom_padding: f32,
    pub shadow_spread: f32,
    pub shadow_extra_padding: f32,
    pub scrim_alpha: f32,
    pub outline_width: f32,
    pub outline_alpha: f32,
    pub handle_width: f32,
    pub handle_height: f32,
    pub handle_y: f32,
    pub search_icon_x: f32,
    pub search_icon_size: f32,
    pub search_label_x: f32,
    pub section_label_x: f32,
    pub section_label_y: f32,
    pub scrollbar_min_height: f32,
    pub scrollbar_right: f32,
    pub scrollbar_width: f32,
    pub scrollbar_alpha: f32,
    pub empty_label_y: f32,
}
#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Apps {
    pub medallion_radius: f32,
    pub icon_center_y: f32,
    pub glyph_size: f32,
    pub label_y: f32,
    pub label_max_chars: usize,
    pub secondary_icons: Vec<String>,
    pub tertiary_icons: Vec<String>,
}
#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Home {
    pub top_gap: f32,
    pub bottom_gap: f32,
    pub margin_x: f32,
    pub max_width: f32,
    pub compact_threshold: f32,
    pub hero_height: f32,
    pub compact_hero_height: f32,
    pub footer_height: f32,
    pub cell_width: f32,
    pub cell_height: f32,
    pub tile_height: f32,
    pub tile_offset_y: f32,
    pub page_gap: f32,
    pub nav_bottom: f32,
    pub nav_offset: f32,
    pub nav_width: f32,
    pub nav_height: f32,
    pub nav_radius: f32,
    pub nav_icon_size: f32,
    pub label_offset_y: f32,
    pub clock_x: f32,
    pub clock_size: f32,
    pub compact_clock_size: f32,
    pub clock_line_height: f32,
    pub date_y: f32,
}
#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Panels {
    pub edge_inset: f32,
    pub viewport_margin: f32,
    pub bottom_gap: f32,
}
#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Shadow {
    pub padding: f32,
    pub spread: f32,
    pub layers: usize,
    pub inner_alpha: f32,
    pub outer_alpha: f32,
}
#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Brightness {
    pub width: f32,
    pub height: f32,
    pub track_left: f32,
    pub track_right: f32,
    pub icon_x: f32,
    pub icon_size: f32,
    pub center_y_offset: f32,
    pub label_y: f32,
    pub track_height: f32,
    pub minimum_fill: f32,
    pub handle_height: f32,
    pub handle_growth: f32,
    pub handle_width: f32,
}
#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Power {
    pub width: f32,
    pub row_height: f32,
    pub row_gap: f32,
    pub padding: f32,
    pub icon_x: f32,
    pub icon_size: f32,
    pub label_x: f32,
}
#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QuickSettings {
    pub width: f32,
    pub height: f32,
    pub min_height: f32,
    pub padding: f32,
    pub title_y: f32,
    pub subtitle_y: f32,
    pub tile_top: f32,
    pub tile_height: f32,
    pub tile_gap: f32,
    pub chip_height: f32,
    pub chip_gap: f32,
    pub chips_y: f32,
    pub chip_margin: f32,
    pub chip_bottom_gap: f32,
    pub tile_tint: f32,
    pub tile_radius: f32,
    pub tile_active_radius_growth: f32,
    pub tile_press_radius_growth: f32,
    pub icon_x: f32,
    pub icon_y: f32,
    pub icon_radius: f32,
    pub icon_alpha: f32,
    pub icon_size: f32,
    pub label_x: f32,
    pub label_y: f32,
    pub status_y: f32,
    pub switch_y: f32,
    pub palette_title_gap: f32,
    pub swatch_first_x: f32,
    pub swatch_gap: f32,
    pub swatch_radius: f32,
    pub chip_label_x: f32,
    pub chip_label_padding: f32,
    pub chip_outline_width: f32,
    pub chip_outline_alpha: f32,
}
#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Switch {
    pub width: f32,
    pub height: f32,
    pub stroke: f32,
    pub handle_off_radius: f32,
    pub handle_on_radius: f32,
}
#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Calendar {
    pub width: f32,
    pub height: f32,
    pub padding: f32,
    pub heading_y: f32,
    pub time_y: f32,
    pub time_size: f32,
    pub time_height: f32,
    pub date_y: f32,
    pub zone_y: f32,
    pub month_y: f32,
    pub nav_y: f32,
    pub nav_width: f32,
    pub nav_height: f32,
    pub nav_right: f32,
    pub nav_gap: f32,
    pub nav_icon_size: f32,
    pub grid_padding: f32,
    pub weekdays_y: f32,
    pub first_day_y: f32,
    pub reserved_height: f32,
    pub min_row_height: f32,
    pub max_row_height: f32,
    pub today_radius_ratio: f32,
    pub today_radius: f32,
    pub day_label_offset: f32,
}
#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Wallpaper {
    pub primary_mix_dark: f32,
    pub primary_mix_light: f32,
    pub tertiary_mix: f32,
    pub petal_mix_dark: f32,
    pub petal_mix_light: f32,
    pub glow_one: [f32; 4],
    pub glow_two: [f32; 4],
    pub petal: [f32; 14],
}
#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Cursor {
    pub size: f32,
    pub outline_width: f32,
    pub stroke_width: f32,
    pub fill_light: String,
    pub fill_dark: String,
    pub outline_light: String,
    pub outline_dark: String,
}
#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Typography {
    pub title_medium: TextStyle,
    pub headline_small: TextStyle,
    pub title_small: TextStyle,
    pub label_large: TextStyle,
    pub label_medium: TextStyle,
    pub label_small: TextStyle,
    pub body_large: TextStyle,
    pub body_medium: TextStyle,
    pub body_small: TextStyle,
    pub focused_title: TextStyle,
}
#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Colors {
    pub light: BTreeMap<String, String>,
    pub dark: BTreeMap<String, String>,
}
#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AppMenu {
    pub width: f32,
    pub height: f32,
    pub margin: f32,
    pub row_inset: f32,
    pub row_y: f32,
    pub home_row_y: f32,
    pub row_height: f32,
    pub title_x: f32,
    pub title_y: f32,
    pub icon_x: f32,
    pub icon_size: f32,
    pub label_x: f32,
    pub label_y: f32,
}
#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Fonts {
    pub directory: String,
}
