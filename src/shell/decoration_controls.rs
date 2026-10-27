//! A titlebar click belongs to one button on one window until release.
use super::DecorHit;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DecorButton {
    Minimize,
    Maximize,
    Close,
}
impl DecorButton {
    pub fn from_hit(hit: DecorHit) -> Option<Self> {
        match hit {
            DecorHit::Minimize => Some(Self::Minimize),
            DecorHit::Maximize => Some(Self::Maximize),
            DecorHit::Close => Some(Self::Close),
            _ => None,
        }
    }
}

pub(crate) struct DecorationControls<T> {
    hovered: Option<(T, DecorButton)>,
    pressed: Option<(T, DecorButton)>,
}
impl<T> Default for DecorationControls<T> {
    fn default() -> Self {
        Self {
            hovered: None,
            pressed: None,
        }
    }
}
impl<T: Clone + PartialEq> DecorationControls<T> {
    pub fn hover(&mut self, hit: Option<(T, DecorButton)>) -> bool {
        if self.hovered == hit {
            return false;
        }
        self.hovered = hit;
        true
    }
    pub fn begin(&mut self, window: T, button: DecorButton) {
        self.hovered = Some((window.clone(), button));
        self.pressed = Some((window, button));
    }
    pub fn has_press(&self) -> bool {
        self.pressed.is_some()
    }
    pub fn release(&mut self, hit: Option<(T, DecorButton)>) -> Option<(T, DecorButton)> {
        self.pressed
            .take()
            .filter(|original| Some(original) == hit.as_ref())
    }
    pub fn visual(&self, window: &T, button: DecorButton) -> (bool, bool) {
        let matches = |hit: &Option<(T, DecorButton)>| {
            hit.as_ref()
                .is_some_and(|(w, b)| w == window && *b == button)
        };
        let hovered = matches(&self.hovered);
        (hovered, hovered && matches(&self.pressed))
    }
    pub fn clear(&mut self) {
        self.hovered = None;
        self.pressed = None;
    }
    pub fn remove(&mut self, window: &T) {
        if self.hovered.as_ref().is_some_and(|(w, _)| w == window) {
            self.hovered = None;
        }
        if self.pressed.as_ref().is_some_and(|(w, _)| w == window) {
            self.pressed = None;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn release_must_match_the_original_window_and_button() {
        let mut controls = DecorationControls::default();
        controls.begin(1, DecorButton::Close);
        assert_eq!(controls.visual(&1, DecorButton::Close), (true, true));
        controls.hover(None);
        assert_eq!(controls.visual(&1, DecorButton::Close), (false, false));
        assert!(controls.release(Some((2, DecorButton::Close))).is_none());
        controls.begin(1, DecorButton::Close);
        assert!(controls.release(Some((1, DecorButton::Maximize))).is_none());
        controls.begin(1, DecorButton::Close);
        assert_eq!(
            controls.release(Some((1, DecorButton::Close))),
            Some((1, DecorButton::Close))
        );
        assert!(!controls.has_press());
        assert!(controls.release(Some((1, DecorButton::Close))).is_none());
    }
    #[test]
    fn returning_inside_works_but_cancellation_and_destroy_do_not_click() {
        let mut controls = DecorationControls::default();
        controls.begin(1, DecorButton::Minimize);
        controls.hover(None);
        controls.hover(Some((1, DecorButton::Minimize)));
        assert_eq!(controls.visual(&1, DecorButton::Minimize), (true, true));
        controls.clear();
        assert!(controls.release(Some((1, DecorButton::Minimize))).is_none());
        controls.begin(1, DecorButton::Close);
        controls.remove(&1);
        assert_eq!(controls.visual(&1, DecorButton::Close), (false, false));
        assert!(controls.release(Some((1, DecorButton::Close))).is_none());
    }
}
