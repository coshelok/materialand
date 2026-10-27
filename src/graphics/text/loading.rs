//! Loading.
use super::*;

impl Fonts {
    pub fn load() -> Result<Self, std::io::Error> {
        Self::load_from_roots(&font_search_roots())
    }

    pub(super) fn load_from_roots(roots: &[PathBuf]) -> Result<Self, std::io::Error> {
        let find_file = |names: &[&str]| discovery::find_file(roots, names);
        let regular_path = find_file(&[
            "Roboto-Regular.ttf",
            "NotoSans-Regular.ttf",
            "DejaVuSans.ttf",
            "LiberationSans-Regular.ttf",
            "Go-Regular.ttf",
        ])
        .ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "Materialand needs a system font; install Roboto, Noto Sans or DejaVu Sans",
            )
        })?;
        let regular = load_font(&regular_path).ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("could not read or parse font {}", regular_path.display()),
            )
        })?;

        let medium = find_file(&[
            "Roboto-Medium.ttf",
            "NotoSans-Medium.ttf",
            "DejaVuSans.ttf",
            "LiberationSans-Regular.ttf",
            "Go-Medium.ttf",
        ])
        .and_then(|path| load_font(&path))
        .or_else(|| load_font(&regular_path))
        .ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "could not reload fallback font",
            )
        })?;

        let bold = find_file(&[
            "Roboto-Bold.ttf",
            "NotoSans-Bold.ttf",
            "DejaVuSans-Bold.ttf",
            "LiberationSans-Bold.ttf",
            "Go-Bold.ttf",
        ])
        .and_then(|path| load_font(&path))
        .or_else(|| load_font(&regular_path))
        .ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "could not reload fallback font",
            )
        })?;

        let mut icon_codepoints = fallback_icons();
        let mut icons = load_font(&regular_path).ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "could not reload icon fallback font",
            )
        })?;
        if let Some(icon_path) = find_file(&["MaterialIcons-Regular.ttf"]) {
            let codepoints_path = icon_path
                .parent()
                .map(|dir| dir.join("codepoints"))
                .filter(|path| path.is_file());
            if let (Some(codepoints_path), Some(icon_font)) =
                (codepoints_path, load_font(&icon_path))
            {
                let canonical = parse_codepoints(&codepoints_path);
                if !canonical.is_empty() {
                    icon_codepoints = canonical;
                    icons = icon_font;
                }
            }
        }

        Ok(Self {
            regular,
            medium,
            bold,
            icons,
            icon_codepoints,
            cache: HashMap::new(),
        })
    }
}
