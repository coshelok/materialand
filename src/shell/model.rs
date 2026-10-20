//! Model.
use super::*;

impl Shell {
    pub fn new(apps: Vec<AppEntry>, spec: ThemeSpec) -> Self {
        let terminal_app = config::select_terminal(&apps);
        let terminal = terminal_app.command();
        let pins = config::Pins::initial(vec![terminal_app.identity()]);

        Shell {
            apps,
            overview_cards: HashMap::new(),
            overview_header: None,
            home: home::Home::new(),
            pinned: vec![terminal_app.clone()],
            terminal_app,
            pins,
            dock_motion: Default::default(),
            terminal,
            app_menu: None,
            press_pin: false,
            consume_menu_release: false,
            launcher_open: false,
            launcher_query: String::new(),
            brightness_open: false,
            system_status: Default::default(),
            notifications: Default::default(),
            notifications_open: false,
            statusbar_cache: LayerCache::default(),
            shade_scroll: 0.0,
            dragged_identity: None,
            shortcut_error: None,
            drag_icon: None,
            navigation_cache: Vec::new(),
            split_handle: Vec::new(),
            connection_page: None,
            brightness_available: false,
            brightness_error: false,
            night_light_available: false,
            quick_settings_open: false,
            power_open: false,
            night_light_on: false,
            reduce_motion: false,
            clock: "00:00".into(),
            calendar: calendar::Calendar::new(),
            calendar_layer: LayerCache::default(),
            spec,
            wallpapers: Vec::new(),
            topbar_cache: Vec::new(),
            dock_cache: Vec::new(),
            decor_cache: HashMap::new(),
            launcher_layer: LayerCache::default(),
            qs_layer: LayerCache::default(),
            brightness_layer: LayerCache::default(),
            power_layer: LayerCache::default(),
            scrim_cache: None,
            frame_epoch: 0,
            cursor_cache: None,
            launcher: Spring::spatial(0.0),
            launcher_scroll: Spring::effects(0.0),
            launcher_scroll_target: 0.0,
            brightness: Spring::spatial(0.0),
            quick_settings: Spring::spatial(0.0),
            power: Spring::spatial(0.0),
            night_light: Spring::effects(0.0),
            theme_mix: Tween::new(
                1.0,
                1.0,
                crate::config::design().motion.theme_duration,
                CubicBezier::emphasized_decelerate(),
            ),
            prev_theme: None,
            reveal: None,
            // Light surfaces make the pastel containers the default appearance.
            switches: [
                Spring::spatial(0.0),
                Spring::spatial(0.0),
                Spring::spatial(0.0),
                Spring::spatial(0.0),
                Spring::spatial(0.0),
                Spring::spatial(0.0),
            ],
            slider: Spring::spatial(0.0),
            window_in: HashMap::new(),
            launch_origins: Default::default(),
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

    pub fn mark_dirty(&mut self) {
        self.wallpapers.clear();
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
            self.reset_launcher_scroll();
        }
    }

    pub fn launcher_backspace(&mut self) {
        self.launcher_query.pop();
        self.reset_launcher_scroll();
    }
}
