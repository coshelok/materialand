//! Fixed header and a GPU-cropped body share one cached shade texture.
use super::*;

impl Shell {
    #[allow(clippy::too_many_arguments)]
    pub(super) fn render_tablet_shade(
        &mut self,
        theme: &Theme,
        fonts: &mut Fonts,
        w: f32,
        h: f32,
        progress: f32,
        brightness: f32,
        variant: ColorVariant,
    ) -> OverlayFrame {
        let rect = quick_settings_rect(w, h);
        let split = shade_split(w);
        let notifications_open = self.notifications_open;
        if split {
            self.notifications_open = false;
        }
        let pad = config::design().shadow.padding;
        let max = self.shade_scroll_limit(w, h);
        let full = RectF {
            h: rect.h + max,
            ..rect
        };
        let stable = self.switches.iter().all(|s| s.settled())
            && !self
                .press
                .keys()
                .any(|k| matches!(k, PressKey::Tile(_) | PressKey::Chip(_)));
        let mut key = KeyHash::new()
            .u64(theme.cache_key())
            .f32(full.w)
            .f32(full.h)
            .str(&self.clock)
            .str(&self.calendar.now.date_text())
            .str(&format!("{:?}", self.system_status))
            .u64(self.notifications.revision)
            .bool(self.notifications_open)
            .str(&format!("{:?}", self.connection_page))
            .str(variant.name())
            .f32(brightness)
            .bool(self.brightness_available)
            .bool(self.brightness_error)
            .bool(self.night_light_available);
        for switch in &self.switches {
            key = key.f32(switch.value());
        }
        let key = key.finish();
        if !stable || !self.qs_layer.select(key) {
            let mut surface = Surface::new(
                (full.w + 2.0 * pad).ceil() as u32,
                (full.h + 2.0 * pad).ceil() as u32,
            );
            let scroll = self.shade_scroll;
            self.shade_scroll = 0.0;
            self.paint_quick_settings(&mut surface, fonts, theme, full, brightness, variant, split);
            self.shade_scroll = scroll;
            let buffer = surface.to_memory_buffer();
            self.qs_layer.store(CachedLayer {
                key,
                surface,
                buffer,
            });
        }
        if !self.shade_backdrop.select(theme.cache_key()) {
            let mut surface = Surface::new(1, 1);
            surface.clear(theme.surface_container_low);
            let buffer = surface.to_memory_buffer();
            self.shade_backdrop.store(CachedLayer {
                key: theme.cache_key(),
                surface,
                buffer,
            });
        }
        let placement = self.quick_settings_placement(w, h, progress);
        let target = placement.target;
        let alpha = 1.0;
        let mut frame = OverlayFrame::single(
            self.shade_backdrop.as_ref().unwrap().buffer.clone(),
            (0.0, f64::from(target.y)).into(),
            Some((w.ceil() as i32, rect.h.ceil() as i32).into()),
            alpha,
        );
        // Notifications keep their Clear all row fixed too.
        let header = (quick_tiles_top()
            + if self.notifications_open {
                config::design().power.row_height
            } else if self.connection_page.is_some() {
                48.0
            } else {
                0.0
            })
        .min(rect.h);
        for (y, height, scroll) in [
            (0.0, header, 0.0),
            (header, rect.h - header, self.shade_scroll.clamp(0.0, max)),
        ] {
            if height < 1.0 {
                continue;
            }
            let r = placement.map_rect(RectF {
                y: rect.y + y,
                h: height,
                ..rect
            });
            frame.pieces.push((
                LayerFrame {
                    buffer: self.qs_layer.as_ref().unwrap().buffer.clone(),
                    source: Some(RectF {
                        x: pad,
                        y: pad + y + scroll,
                        w: rect.w,
                        h: height,
                    }),
                    loc: (f64::from(r.x), f64::from(r.y)).into(),
                    size: Some((r.w.round() as i32, r.h.round() as i32).into()),
                },
                alpha,
            ));
        }
        self.notifications_open = notifications_open;
        if split {
            frame.pieces.extend(
                self.render_notification_column(theme, fonts, w, h, progress)
                    .pieces,
            );
        }
        frame
    }

    fn render_notification_column(
        &mut self,
        theme: &Theme,
        fonts: &mut Fonts,
        w: f32,
        h: f32,
        progress: f32,
    ) -> OverlayFrame {
        let rect = notification_panel(w, h);
        let max = self.notification_scroll_max(w, h);
        let full = RectF {
            h: rect.h + max,
            ..rect
        };
        let top = notification_top(w);
        let header = top + config::design().power.row_height;
        let key = KeyHash::new()
            .u64(theme.cache_key())
            .u64(self.notifications.revision)
            .bool(self.notifications.available)
            .f32(full.w)
            .f32(full.h)
            .finish();
        if !self.notice_layer.select(key) {
            let mut surface = Surface::new(full.w.ceil() as u32, full.h.ceil() as u32);
            surface.clear(theme.surface_container_low);
            fonts.draw_text_role(
                &mut surface,
                config::design().quick_settings.padding,
                config::design().quick_settings.title_y,
                TextStyle::headline_small(),
                "Notifications",
                theme.on_surface,
            );
            self.paint_notification_content(&mut surface, fonts, theme, full, 0.0, top, 0.0);
            let buffer = surface.to_memory_buffer();
            self.notice_layer.store(CachedLayer {
                key,
                surface,
                buffer,
            });
        }
        let placement = self.quick_settings_placement(w, h, progress);
        let dy = placement.target.y - placement.source.y;
        let mut frame = OverlayFrame { pieces: vec![] };
        for (y, height, scroll) in [
            (0.0, header, 0.0),
            (
                header,
                (rect.h - header).max(0.0),
                self.notice_scroll.value().clamp(0.0, max),
            ),
        ] {
            if height < 1.0 {
                continue;
            }
            frame.pieces.push((
                LayerFrame {
                    buffer: self.notice_layer.as_ref().unwrap().buffer.clone(),
                    source: Some(RectF {
                        x: 0.0,
                        y: y + scroll,
                        w: rect.w,
                        h: height,
                    }),
                    loc: (f64::from(rect.x), f64::from(rect.y + y + dy)).into(),
                    size: Some((rect.w.round() as i32, height.round() as i32).into()),
                },
                1.0,
            ));
        }
        frame
    }
}
