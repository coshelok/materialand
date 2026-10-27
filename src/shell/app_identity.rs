//! One desktop-entry identity and original icon across window chrome and launchers.
use super::*;

pub(super) struct AppIcon<'a> {
    pub key: &'a str,
    image: Option<&'a Surface>,
}

impl AppIcon<'_> {
    /// Fit an original image into a square without changing its aspect or alpha.
    /// Return false when the caller should paint its usual themed fallback.
    pub fn paint(&self, surface: &mut Surface, center: (f32, f32), side: f32) -> bool {
        let Some(image) = self.image else {
            return false;
        };
        let ratio = side / image.width().max(image.pixmap.height()) as f32;
        surface.blit(
            image,
            SkTransform::from_scale(ratio, ratio).post_translate(
                center.0 - image.width() as f32 * ratio / 2.0,
                center.1 - image.pixmap.height() as f32 * ratio / 2.0,
            ),
            1.0,
            None,
        );
        true
    }
}

impl Shell {
    pub fn window_app(&self, app_id: &str) -> Option<&AppEntry> {
        let apps = self
            .pinned
            .iter()
            .chain(&self.apps)
            .chain(std::iter::once(&self.terminal_app));
        // An exact desktop ID wins over another application's executable/name alias.
        apps.clone()
            .find(|app| app.matches_desktop_id(app_id))
            .or_else(|| apps.into_iter().find(|app| app.matches_app_id(app_id)))
    }

    pub(super) fn app_icon<'a>(&'a self, key: &'a str) -> AppIcon<'a> {
        AppIcon {
            key,
            image: self.app_icons.get(key),
        }
    }

    pub(super) fn window_icon<'a>(&'a self, meta: &'a WindowMeta) -> AppIcon<'a> {
        let key = self
            .window_app(&meta.app_id)
            .map(|app| app.icon.as_str())
            .filter(|key| !key.is_empty())
            .unwrap_or_else(|| {
                if meta.app_id.is_empty() {
                    &meta.title
                } else {
                    &meta.app_id
                }
            });
        self.app_icon(key)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn app(id: &str, command: &str, icon: &str) -> AppEntry {
        AppEntry {
            name: id.into(),
            exec: command.into(),
            icon: icon.into(),
            terminal: false,
            desktop_file: Some(format!("/applications/{id}.desktop").into()),
        }
    }

    #[test]
    fn exact_identity_wins_over_pinned_alias_and_unknown_apps_keep_fallback() {
        let exact = app("org.example.Editor", "editor", "original-editor-icon");
        let mut shell = Shell::new(vec![exact.clone()], ThemeSpec::default());
        shell.pinned = vec![app("other-editor", "Editor", "wrong-icon")];
        assert_eq!(
            shell
                .window_app("ORG.EXAMPLE.EDITOR.desktop")
                .unwrap()
                .identity(),
            exact.identity()
        );
        let mut meta = WindowMeta {
            app_id: "org.example.Editor".into(),
            title: "Document".into(),
        };
        assert_eq!(shell.window_icon(&meta).key, exact.icon);
        meta.app_id = "unregistered-app".into();
        assert!(shell.window_app(&meta.app_id).is_none());
        assert_eq!(shell.window_icon(&meta).key, "unregistered-app");
        meta.app_id.clear();
        assert_eq!(shell.window_icon(&meta).key, "Document");
    }

    #[test]
    fn original_icon_reaches_recents_and_preserves_aspect_and_alpha() {
        let path =
            std::env::temp_dir().join(format!("materialand-identity-{}.png", std::process::id()));
        let mut original = Surface::new(16, 8);
        original.clear(Color::from_rgba8(255, 0, 0, 128));
        original.pixmap.save_png(&path).unwrap();
        let entry = app("org.example.Image", "image", path.to_str().unwrap());
        let shell = Shell::new(vec![entry.clone()], ThemeSpec::default());
        let meta = WindowMeta {
            app_id: "org.example.Image".into(),
            title: "Image".into(),
        };
        let mut surface = Surface::new(32, 32);
        assert!(shell
            .window_icon(&meta)
            .paint(&mut surface, (16.0, 16.0), 16.0));
        let center = surface.pixmap.pixel(16, 16).unwrap();
        assert_eq!((center.red(), center.alpha()), (128, 128));
        assert_eq!(surface.pixmap.pixel(16, 8).unwrap().alpha(), 0);
        let mut fonts = Fonts::load().unwrap();
        let theme = ThemeSpec::default().light();
        let card = shell.paint_recent_card(&theme, &mut fonts, &meta, (300.0, 200.0), false, false);
        let header = config::design().overview.app_header_height;
        let center = card
            .pixmap
            .pixel((header * 0.35) as u32, (header / 2.0) as u32)
            .unwrap();
        assert_eq!(
            (center.red(), center.green(), center.alpha()),
            (128, 0, 128)
        );
        std::fs::remove_file(path).unwrap();
    }
}
