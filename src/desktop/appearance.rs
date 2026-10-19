//! Appearance.
use super::*;

impl State {
    /// Load before the first frame, so saved choices never flash the default theme.
    pub fn load_appearance(&mut self) {
        self.restore_appearance(config::AppearanceStore::at(config::appearance_path()));
    }

    pub(super) fn restore_appearance(&mut self, store: config::AppearanceStore) {
        match store.load(self.current_appearance()) {
            Ok(appearance) => {
                self.theme_spec.variant = appearance.variant;
                self.theme = if appearance.dark {
                    self.theme_spec.dark()
                } else {
                    self.theme_spec.light()
                };
                self.shell.set_reduce_motion(appearance.reduce_motion);
                self.shell.set_theme_switch(appearance.dark);
            }
            Err(error) => {
                tracing::warn!(%error, "could not load appearance; using startup defaults")
            }
        }
        self.appearance_store = store;
    }

    fn current_appearance(&self) -> config::Appearance {
        config::Appearance {
            dark: self.theme.is_dark,
            variant: self.theme_spec.variant,
            reduce_motion: self.shell.reduce_motion,
        }
    }

    fn save_appearance(&self) {
        if let Err(error) = self.appearance_store.save(self.current_appearance()) {
            tracing::warn!(%error, "could not save appearance");
        }
    }

    pub fn toggle_reduce_motion(&mut self) {
        self.shell.set_reduce_motion(!self.shell.reduce_motion);
        self.save_appearance();
    }

    pub fn toggle_theme(&mut self, origin: Point<f64, Logical>) {
        // Continue from the palette currently on screen when toggled mid-animation.
        let old = self.shell.effective_theme(&self.theme);
        let dark = !self.theme.is_dark;
        self.theme = if dark {
            self.theme_spec.dark()
        } else {
            self.theme_spec.light()
        };
        self.shell.set_theme_switch(dark);
        self.shell.begin_reveal(old, origin);
        self.save_appearance();
        self.prewarm_theme();
    }

    pub fn set_variant(&mut self, variant: ColorVariant) {
        if self.theme_spec.variant == variant {
            return;
        }
        let old = self.shell.effective_theme(&self.theme);
        self.theme_spec.variant = variant;
        self.theme = if self.theme.is_dark {
            self.theme_spec.dark()
        } else {
            self.theme_spec.light()
        };
        self.shell.begin_theme_change(old);
        self.save_appearance();
        self.prewarm_theme();
    }

    pub(super) fn prewarm_theme(&mut self) {
        let w = self.display.size.w as f32;
        let h = self.display.size.h as f32;
        let theme = self.theme;
        let _ = self.shell.render_wallpaper(
            &theme,
            self.display.size.w as u32,
            self.display.size.h as u32,
            None,
        );
        let items = crate::input::dock_items(self);
        let _ = self
            .shell
            .render_topbar(&theme, &mut self.fonts, w, "", None);
        let _ = self.shell.render_dock(
            &theme,
            &mut self.fonts,
            w,
            h,
            &items,
            self.windows.focused.as_ref(),
            None,
        );
        if self.shell.quick_settings_open {
            let variant = self.theme_spec.variant;
            let _ = self.shell.render_quick_settings(
                &theme,
                &mut self.fonts,
                w,
                h,
                1.0,
                self.brightness,
                variant,
                None,
            );
        }
        let windows: Vec<_> = self.windows.space.elements().cloned().collect();
        for window in windows {
            let props = self.window_props(&window);
            if props.fullscreen || !self.window_server_decor(&window) {
                continue;
            }
            let geom = self
                .windows
                .space
                .element_geometry(&window)
                .unwrap_or_else(|| window.geometry());
            let meta = self.window_meta(&window);
            let focused = self.windows.focused.as_ref() == Some(&window);
            let _ = self.shell.render_decoration(
                &theme,
                &mut self.fonts,
                &window,
                geom,
                crate::shell::DecorParams {
                    meta: &meta,
                    focused,
                    maximized: props.maximized,
                },
                None,
            );
        }
    }
}
