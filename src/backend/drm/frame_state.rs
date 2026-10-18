//! Presentation state belongs to one output and is reset when that output dies.
#[derive(Default, Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum FrameState {
    #[default]
    Ready,
    Submitted,
}
impl FrameState {
    pub fn is_pending(self) -> bool {
        self == Self::Submitted
    }
    pub fn queued(&mut self) {
        assert_eq!(*self, Self::Ready, "only one DRM frame may be queued");
        *self = Self::Submitted;
    }
    pub fn complete(&mut self) -> bool {
        let pending = self.is_pending();
        *self = Self::Ready;
        pending
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn unsolicited_vblank_does_not_complete_a_frame() {
        let mut state = FrameState::default();
        assert!(!state.complete());
        state.queued();
        assert!(state.is_pending());
        assert!(state.complete());
        assert!(!state.complete());
    }
    #[test]
    #[should_panic(expected = "only one DRM frame")]
    fn a_second_submission_requires_completion() {
        let mut state = FrameState::default();
        state.queued();
        state.queued();
    }
}
