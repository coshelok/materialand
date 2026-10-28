//! Expansion-driven content phases: reversible while dragging, with no timers.
use super::notifications::notification_stride;
use super::*;

#[derive(Clone, Copy)]
struct Band {
    y: f32,
    end: f32,
    delay: f32,
}
fn phase(progress: f32, delay: f32) -> f32 {
    let t = ((progress - delay) / (0.94 - delay)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}
impl Shell {
    fn reveal_bands(&self, w: f32, h: f32, column: bool) -> Vec<Band> {
        let d = config::design();
        let rect = quick_settings_rect(w, h);
        let notifications = column || (self.notifications_open && !shade_split(w));
        let main = !notifications && self.connection_page.is_none() && !self.appearance_page;
        let header = if column {
            notification_top(w) + d.power.row_height
        } else {
            quick_tiles_top()
                + if notifications {
                    d.power.row_height
                } else if self.connection_page.is_some() {
                    48.0
                } else {
                    0.0
                }
        }
        .min(rect.h);
        let mut bands = Vec::new();
        if main && !column {
            let brightness = (d.shade.brightness_y - 16.0).clamp(0.0, header);
            bands.push(Band {
                y: 0.0,
                end: brightness,
                delay: 0.0,
            });
            bands.push(Band {
                y: brightness,
                end: header,
                delay: 0.08,
            });
        } else {
            bands.push(Band {
                y: 0.0,
                end: header,
                delay: if column { 0.12 } else { 0.0 },
            });
        }
        let stride = if notifications {
            notification_stride()
        } else if self.connection_page.is_some() {
            d.power.row_height
        } else if self.appearance_page {
            d.quick_settings.chip_height + d.quick_settings.chip_gap
        } else {
            d.quick_settings.tile_height + d.quick_settings.tile_gap
        }
        .max(1.0);
        let scroll = if column {
            self.notice_scroll.value()
        } else {
            self.shade_scroll
        }
        .max(0.0);
        let mut y = header;
        let mut row = 0;
        while y < rect.h {
            let end = (y + stride - if row == 0 { scroll % stride } else { 0.0 }).min(rect.h);
            bands.push(Band {
                y,
                end,
                delay: (if column { 0.26 } else { 0.16 } + row as f32 * 0.07).min(0.55),
            });
            y = end;
            row += 1;
        }
        bands
    }

    pub fn shade_visible_contains(&self, w: f32, h: f32, point: Point<f64, Logical>) -> bool {
        if tablet_mode() && config::design().shade.fullscreen {
            [false, true]
                .into_iter()
                .filter(|right| !right || shade_split(w))
                .any(|right| {
                    shade_contour(w, h, self.quick_settings_progress(), right)
                        .contains_rounded(point, sheet_radius())
                })
        } else {
            shade_contains(
                w,
                h,
                self.quick_settings_placement(w, h, self.quick_settings_progress())
                    .unmap(point),
            )
        }
    }
    pub fn shade_content_visible(&self, w: f32, h: f32, point: Point<f64, Logical>) -> bool {
        if !self.shade_visible_contains(w, h, point) {
            return false;
        }
        if !tablet_mode() || !config::design().shade.fullscreen {
            return true;
        }
        let column = shade_split(w) && point.x >= f64::from(notification_panel(w, h).x);
        let y = point.y as f32 - quick_settings_rect(w, h).y;
        self.reveal_bands(w, h, column)
            .iter()
            .find(|b| y >= b.y && y < b.end)
            .is_some_and(|b| phase(self.quick_settings_progress(), b.delay) >= 0.1)
    }

    pub(super) fn reveal_shade_frame(
        &mut self,
        frame: &mut OverlayFrame,
        theme: &Theme,
        w: f32,
        h: f32,
        progress: f32,
    ) {
        // Settled frames keep the original header/body crops and draw count.
        if progress == 1.0 {
            return;
        }
        if !self.shade_surface.select(theme.cache_key()) {
            let mut surface = Surface::new(1, 1);
            surface.clear(theme.surface_container_low);
            let buffer = surface.to_memory_buffer();
            self.shade_surface.store(CachedLayer {
                key: theme.cache_key(),
                surface,
                buffer,
            });
        }
        let mut pieces = std::mem::take(&mut frame.pieces).into_iter();
        if let Some(scrim) = pieces.next() {
            frame.pieces.push(scrim);
        }
        let rect = quick_settings_rect(w, h);
        let mut painted = [false; 2];
        for (layer, _) in pieces {
            let column = shade_split(w) && layer.loc.x >= f64::from(notification_panel(w, h).x);
            let contour = shade_contour(w, h, progress, column);
            let clip = Some(RoundedClip {
                rect: contour,
                radius: sheet_radius(),
            });
            if !painted[usize::from(column)] {
                frame.pieces.push((
                    LayerFrame {
                        rounded: clip,
                        source: None,
                        buffer: self.shade_surface.as_ref().unwrap().buffer.clone(),
                        loc: (f64::from(contour.x), f64::from(contour.y)).into(),
                        size: Some(
                            (contour.w.round() as i32, contour.h.ceil().max(1.0) as i32).into(),
                        ),
                    },
                    (progress * 3.0).clamp(0.0, 1.0),
                ));
                painted[usize::from(column)] = true;
            }
            let source = layer.source.unwrap();
            let top = layer.loc.y as f32;
            for band in self.reveal_bands(w, h, column) {
                let start = top.max(rect.y + band.y);
                let end = (top + source.h).min(rect.y + band.end);
                let alpha = phase(progress, band.delay);
                if end <= start || alpha <= 0.001 {
                    continue;
                }
                frame.pieces.push((
                    LayerFrame {
                        rounded: clip,
                        buffer: layer.buffer.clone(),
                        source: Some(RectF {
                            y: source.y + start - top,
                            h: end - start,
                            ..source
                        }),
                        loc: (layer.loc.x, f64::from(start)).into(),
                        size: Some(
                            (
                                source.w.round() as i32,
                                (end - start).round().max(1.0) as i32,
                            )
                                .into(),
                        ),
                    },
                    alpha,
                ));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expansion_reveals_top_first_with_ordered_opacity_and_cached_native_content() {
        let mut fonts = Fonts::load().unwrap();
        for (w, h) in [(320.0, 640.0), (1280.0, 800.0)] {
            let mut shell = Shell::new(vec![], ThemeSpec::default());
            shell.set_quick_settings_open(true);
            let theme = ThemeSpec::default().light();
            let mut cached = None;
            for p in [0.2, 0.55, 0.8, 0.35, 1.0, 1.03] {
                shell.drag_shade(p);
                let frame = shell.render_quick_settings(
                    &theme,
                    &mut fonts,
                    w,
                    h,
                    p,
                    0.5,
                    ColorVariant::Neutral,
                    None,
                );
                let texture = shell
                    .qs_layer
                    .as_ref()
                    .unwrap()
                    .surface
                    .pixmap
                    .data()
                    .as_ptr();
                if let Some(previous) = cached {
                    assert_eq!(texture, previous, "drag must reuse the raster");
                }
                cached = Some(texture);
                let rect = quick_settings_rect(w, h);
                let contour = shade_contour(w, h, p, false);
                assert_eq!(contour.y, rect.y, "the top must not slide off screen");
                if p < 1.0 {
                    assert!(contour.h < rect.h);
                }
                if p > 1.0 {
                    assert!(contour.h > rect.h && contour.h <= rect.h + 16.0);
                }
                let mut alphas = Vec::new();
                for (layer, alpha) in &frame.pieces {
                    if let Some(src) = layer.source {
                        let size = layer.size.unwrap();
                        assert!((size.w as f32 - src.w).abs() <= 1.0);
                        assert!((size.h as f32 - src.h).abs() <= 1.0, "do not stretch text");
                        if layer.loc.x == f64::from(rect.x) {
                            alphas.push(*alpha);
                        }
                    }
                }
                assert!(
                    alphas.windows(2).all(|a| a[0] >= a[1]),
                    "upper groups appear first"
                );
                if p == 0.55 {
                    assert!(alphas.len() >= 3 && alphas[0] > alphas[1] && alphas[1] > alphas[2]);
                    let hidden = (
                        f64::from(rect.x + rect.w / 2.0),
                        f64::from(contour.bottom() + 2.0),
                    )
                        .into();
                    assert!(!shell.shade_content_visible(w, h, hidden));
                    assert!(!shell.shade_visible_contains(w, h, hidden));
                    if shade_split(w) {
                        let right = notification_panel(w, h);
                        assert!(shell.shade_visible_contains(
                            w,
                            h,
                            (
                                f64::from(right.x + right.w / 2.0),
                                f64::from(right.y + 60.0)
                            )
                                .into()
                        ));
                    }
                }
            }
        }
    }
}
