//! Shared transforms keep animated panel pixels and input in the same place.
use super::*;

#[derive(Clone, Copy)]
pub struct CardPlacement {
    pub source: RectF,
    pub target: RectF,
}

impl CardPlacement {
    pub fn map_rect(self, rect: RectF) -> RectF {
        let sx = self.target.w / self.source.w.max(1.0);
        let sy = self.target.h / self.source.h.max(1.0);
        RectF {
            x: self.target.x + (rect.x - self.source.x) * sx,
            y: self.target.y + (rect.y - self.source.y) * sy,
            w: rect.w * sx,
            h: rect.h * sy,
        }
    }
    pub fn unmap(self, point: Point<f64, Logical>) -> Point<f64, Logical> {
        Point::from((
            f64::from(self.source.x)
                + (point.x - f64::from(self.target.x))
                    * f64::from(self.source.w / self.target.w.max(0.01)),
            f64::from(self.source.y)
                + (point.y - f64::from(self.target.y))
                    * f64::from(self.source.h / self.target.h.max(0.01)),
        ))
    }

    /// Scale and local translation for a card buffer with transparent padding.
    pub(super) fn buffer_transform(self, pad: f32) -> (f32, f32, f32) {
        let scale = self.target.w / self.source.w;
        (
            scale,
            self.target.x - self.source.x + pad * (1.0 - scale),
            self.target.y - self.source.y + pad * (1.0 - scale),
        )
    }
}

impl Shell {
    pub fn launcher_placement(&self, w: f32, h: f32, progress: f32) -> CardPlacement {
        let source = launcher_card(w, h);
        if tablet_mode() && config::design().launcher.fullscreen {
            return CardPlacement {
                source,
                target: RectF {
                    y: source.y + sheet_travel(progress, h - source.y),
                    ..source
                },
            };
        }
        let origin = self
            .launcher_origin
            .unwrap_or_else(|| topbar_button_rect(w, TopAction::Launcher));
        let t = progress.clamp(0.0, crate::config::design().motion.overshoot_limit);
        let lerp = |a: f32, b: f32| a + (b - a) * t;
        CardPlacement {
            source,
            target: RectF {
                x: lerp(origin.x, source.x),
                y: lerp(origin.y, source.y),
                w: lerp(origin.w, source.w).max(1.0),
                h: lerp(origin.h, source.h).max(1.0),
            },
        }
    }

    pub fn quick_settings_placement(&self, w: f32, h: f32, progress: f32) -> CardPlacement {
        let design = crate::config::design();
        let source = quick_settings_rect(w, h);
        if tablet_mode() && design.shade.fullscreen {
            // Content stays at native size; the contour expands downward over it.
            return CardPlacement {
                source,
                target: source,
            };
        }
        if tablet_mode() {
            return CardPlacement {
                source,
                target: RectF {
                    y: source.y - sheet_travel(progress, source.bottom() + design.shade.margin),
                    ..source
                },
            };
        }
        let origin = self
            .qs_origin
            .unwrap_or_else(|| topbar_button_rect(w, TopAction::QuickSettings));
        let t = progress.clamp(0.0, design.motion.overshoot_limit);
        let scale =
            design.motion.settings_initial_scale + (1.0 - design.motion.settings_initial_scale) * t;
        let dy = (origin.y + origin.h / 2.0 - (source.y + source.h / 2.0)) * (1.0 - t);
        CardPlacement {
            source,
            target: RectF {
                x: source.x + source.w * (1.0 - scale) / 2.0,
                y: source.y + source.h * (1.0 - scale) / 2.0 + dy,
                w: source.w * scale,
                h: source.h * scale,
            },
        }
    }

    pub fn brightness_placement(&self, w: f32, progress: f32) -> CardPlacement {
        panel_placement(
            brightness_popup(w),
            topbar_rect(w).bottom(),
            progress,
            false,
        )
    }

    pub fn power_placement(&self, w: f32, progress: f32) -> CardPlacement {
        panel_placement(power_menu_rect(w), topbar_rect(w).bottom(), progress, true)
    }
}

/// Shared by drawing and hit testing: reveal the top of the shade first.
pub fn shade_contour(w: f32, h: f32, progress: f32, notifications: bool) -> RectF {
    let rect = if notifications {
        notification_panel(w, h)
    } else {
        quick_settings_rect(w, h)
    };
    RectF {
        h: (rect.h - sheet_travel(progress, rect.h)).max(0.0),
        ..rect
    }
}

fn panel_placement(source: RectF, bar_bottom: f32, progress: f32, corner: bool) -> CardPlacement {
    let design = crate::config::design();
    let t = progress.clamp(0.0, design.motion.overshoot_limit);
    let scale = design.motion.panel_initial_scale + (1.0 - design.motion.panel_initial_scale) * t;
    CardPlacement {
        source,
        target: RectF {
            x: source.x + source.w * (1.0 - scale) * if corner { 1.0 } else { 0.5 },
            y: source.y
                + source.h * (1.0 - scale) * if corner { 0.0 } else { 0.5 }
                + (bar_bottom - source.y) * (1.0 - t),
            w: source.w * scale,
            h: source.h * scale,
        },
    }
}

/// Preserve spring velocity through the landing, softly limiting excess travel
/// to 16 logical pixels even on a very tall display. No hard stop at progress 1.
fn sheet_travel(progress: f32, distance: f32) -> f32 {
    let offset = (1.0 - progress.max(0.0)) * distance;
    if offset < 0.0 {
        16.0 * (offset / 16.0).tanh()
    } else {
        offset
    }
}
