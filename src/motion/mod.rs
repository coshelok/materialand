//! Material 3 motion: cubic-bezier easing curves and damped springs.
//!
//! The shell animates with two tools from the M3 spec:
//! * **Easing curves** for time-bound transitions (fades, scrims, page changes),
//!   expressed as CSS-style cubic beziers.
//! * **Springs** for interactive/adaptive motion (launcher, brightness slider,
//!   window open/close) where velocity continuity matters.
//!
//! Both are advanced by a single `dt` per frame; callers redraw while any
//! animation reports itself as active.

/// A cubic-bezier timing function with fixed endpoints `(0,0)` and `(1,1)`.
mod curve;
mod spring;
mod tween;
pub use curve::*;
pub use spring::*;
pub use tween::*;
#[cfg(test)]
mod tests;
