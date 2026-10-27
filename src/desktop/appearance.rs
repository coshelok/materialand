//! Appearance.
use super::*;

impl State {
    /// Load before the first frame, so saved choices never flash the default theme.
    pub fn load_appearance(&mut self) {
        self.restore_appearance(config::AppearanceStore::at(config::appearance_path()));
    }

    pub(super) fn restore_appearance(&mut self, store: config::AppearanceStore) {
        self.shell.appearance_error = None;
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
                tracing::warn!(%error, "could not load appearance; using startup defaults");
                self.shell.appearance_error = Some(appearance_error(&error).into());
            }
        }
        self.appearance_writer = config::AppearanceWriter::new(store);
    }

    fn current_appearance(&self) -> config::Appearance {
        config::Appearance {
            dark: self.theme.is_dark,
            variant: self.theme_spec.variant,
            reduce_motion: self.shell.reduce_motion,
        }
    }

    pub fn save_appearance(&mut self) {
        if let Err(error) = self.appearance_writer.request(self.current_appearance()) {
            tracing::warn!(%error, "could not save appearance");
            self.shell.appearance_error = Some(appearance_error(&error).into());
        }
    }

    pub fn poll_appearance(&mut self) -> bool {
        let Some(result) = self.appearance_writer.poll() else {
            return false;
        };
        let error = result
            .err()
            .map(|error| appearance_error(&error).to_owned());
        let changed = self.shell.appearance_error != error;
        self.shell.appearance_error = error;
        changed
    }

    #[cfg(test)]
    pub(crate) fn flush_appearance(&self) {
        self.appearance_writer.flush();
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
        // Scene assembly lazily prepares the visible layers once per frame.
        // Rasterising here delays input and also warms hidden desktop chrome.
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
    }
}

fn appearance_error(error: &std::io::Error) -> &'static str {
    match error.kind() {
        std::io::ErrorKind::InvalidData => "Config needs repair",
        std::io::ErrorKind::PermissionDenied => "Save permission denied",
        _ => "Appearance not saved",
    }
}
