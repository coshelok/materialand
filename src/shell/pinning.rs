//! User pin choices are independent of terminal shortcuts and running windows.
use super::*;
impl Shell {
    pub fn load_pins(&mut self) {
        let Some(path) = config::pins_path() else {
            return;
        };
        if let Err(error) = self.load_pins_at(path.clone()) {
            tracing::warn!(%error, "could not load dock pins");
            self.pins.set_path(path);
        }
    }
    fn load_pins_at(&mut self, path: std::path::PathBuf) -> std::io::Result<()> {
        let pins = config::Pins::load(path, self.pins.ids().to_vec())?;
        let choices: Vec<_> = self.apps.iter().chain(&self.pinned).cloned().collect();
        self.pinned = pins
            .ids()
            .iter()
            .filter_map(|id| choices.iter().find(|a| &a.identity() == id).cloned())
            .collect();
        self.pins = pins;
        Ok(())
    }
    pub fn is_pinned(&self, app: &AppEntry) -> bool {
        self.pins.contains(&app.identity())
    }
    pub fn toggle_pin(&mut self, app: AppEntry) -> std::io::Result<()> {
        let id = app.identity();
        self.pins.toggle(id.clone())?;
        if self.pins.contains(&id) {
            self.pinned.push(app);
        } else {
            self.pinned.retain(|a| a.identity() != id);
        }
        self.dock_cache.clear();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn pin_resolution_survives_restart_and_preserves_uninstalled_desktop_ids() {
        let dir =
            std::env::temp_dir().join(format!("materialand-pin-resolution-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("pinned-apps");
        std::fs::write(
            &path,
            "desktop:a.desktop\ndesktop:missing.desktop\ndesktop:b.desktop\n",
        )
        .unwrap();
        let entry = |name: &str| AppEntry {
            name: name.into(),
            exec: "/bin/true".into(),
            icon: "apps".into(),
            terminal: false,
            desktop_file: Some(format!("/applications/{name}.desktop").into()),
        };
        let (a, b, missing) = (entry("a"), entry("b"), entry("missing"));
        let mut shell = Shell::new(vec![a.clone(), b.clone()], ThemeSpec::default());
        shell.load_pins_at(path.clone()).unwrap();
        assert_eq!(
            shell
                .pinned
                .iter()
                .map(AppEntry::identity)
                .collect::<Vec<_>>(),
            [a.identity(), b.identity()]
        );
        shell.toggle_pin(a.clone()).unwrap();
        let mut restarted = Shell::new(vec![a, b.clone()], ThemeSpec::default());
        restarted.load_pins_at(path.clone()).unwrap();
        assert_eq!(restarted.pinned.len(), 1);
        assert_eq!(restarted.pinned[0].identity(), b.identity());
        restarted.toggle_pin(b).unwrap();
        let mut reinstalled = Shell::new(vec![missing.clone()], ThemeSpec::default());
        reinstalled.load_pins_at(path).unwrap();
        assert_eq!(reinstalled.pinned.len(), 1);
        assert_eq!(reinstalled.pinned[0].identity(), missing.identity());
        std::fs::remove_dir_all(dir).unwrap();
    }
}
