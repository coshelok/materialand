//! Smithay FractionalScaleHandler adapter.
use super::super::*;
impl FractionalScaleHandler for State {
    fn new_fractional_scale(&mut self, surface: WlSurface) {
        // The output runs at an integer scale of 1, so the preferred fractional
        // scale is always 1.0. Announcing it keeps fractionally-scaled clients
        // from picking their own guess.
        let scale = self.display.handle.current_scale().fractional_scale();
        compositor::with_states(&surface, |states| {
            fractional_scale::with_fractional_scale(states, |fs| {
                fs.set_preferred_scale(scale);
            });
        });
    }
}
