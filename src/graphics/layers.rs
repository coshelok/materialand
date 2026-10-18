//! Scene groups are submitted back-to-front; each group is already front-to-back.
//! Reordering whole groups preserves popup/subsurface ordering within a window.
#[derive(Default)]
pub struct Layers<T> {
    groups: Vec<Vec<T>>,
}
impl<T> Layers<T> {
    pub fn new() -> Self {
        Self { groups: Vec::new() }
    }
    pub fn group(&mut self, front_to_back: Vec<T>) {
        self.groups.push(front_to_back);
    }
    pub fn single(&mut self, element: T) {
        self.group(vec![element]);
    }
    pub fn finish(self) -> Vec<T> {
        self.groups.into_iter().rev().flatten().collect()
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn overlapping_windows_preserve_popups_and_subsurfaces() {
        let mut scene = Layers::new();
        scene.single("wallpaper");
        scene.group(vec!["lower-popup", "lower-subsurface", "lower-toplevel"]);
        scene.group(vec!["upper-popup", "upper-toplevel"]);
        scene.single("cursor");
        assert_eq!(
            scene.finish(),
            [
                "cursor",
                "upper-popup",
                "upper-toplevel",
                "lower-popup",
                "lower-subsurface",
                "lower-toplevel",
                "wallpaper"
            ]
        );
    }
}
