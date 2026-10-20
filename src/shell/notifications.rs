//! Notification list geometry is shared by painting, scrolling and dismiss hits.
use super::*;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NotificationHit {
    Clear,
    Dismiss(u32),
}
pub fn notification_stride() -> f32 {
    let d = &config::design().quick_settings;
    d.tile_height + d.tile_gap
}
impl Shell {
    pub fn notification_scroll_max(&self, w: f32, h: f32) -> f32 {
        (quick_tiles_top()
            + config::design().power.row_height
            + self.notifications.items.len() as f32 * notification_stride()
            + config::design().quick_settings.padding
            - quick_settings_rect(w, h).h)
            .max(0.0)
    }
    pub fn notification_hit(
        &self,
        w: f32,
        h: f32,
        pos: Point<f64, Logical>,
    ) -> Option<NotificationHit> {
        let rect = quick_settings_rect(w, h);
        if !rect.contains(pos) {
            return None;
        }
        let y = pos.y as f32 - rect.y - quick_tiles_top();
        let header = config::design().power.row_height;
        if y < 0.0 {
            return None;
        }
        if y < header {
            return (!self.notifications.items.is_empty()
                && pos.x as f32 > rect.right() - header * 2.0)
                .then_some(NotificationHit::Clear);
        }
        let index = ((y - header + self.shade_scroll) / notification_stride()).floor() as usize;
        let notice = self.notifications.items.iter().rev().nth(index)?;
        let row_y = (y - header + self.shade_scroll) % notification_stride();
        (row_y < config::design().quick_settings.tile_height
            && pos.x as f32 > rect.right() - config::design().quick_settings.padding - header)
            .then_some(NotificationHit::Dismiss(notice.id))
    }
    pub(super) fn paint_notifications(
        &self,
        layer: &mut Surface,
        fonts: &mut Fonts,
        theme: &Theme,
        rect: RectF,
        pad: f32,
    ) {
        let d = &config::design().quick_settings;
        let header = config::design().power.row_height;
        let top = quick_tiles_top();
        if !self.notifications.items.is_empty() {
            fonts.draw_text_role(
                layer,
                pad + rect.w - 2.0 * header,
                pad + top + 8.0,
                TextStyle::label_large(),
                "Clear all",
                theme.primary,
            );
        } else {
            let text = if self.notifications.available {
                "No notifications"
            } else {
                self.notifications
                    .error
                    .as_deref()
                    .unwrap_or("Notification service unavailable")
            };
            let text = fit_text(
                fonts,
                TextStyle::body_medium(),
                text,
                rect.w - 2.0 * d.padding,
            );
            fonts.draw_text_role(
                layer,
                pad + d.padding,
                pad + top + header,
                TextStyle::body_medium(),
                &text,
                theme.on_surface_variant,
            );
        }
        let mut body = Surface::new(layer.pixmap.width(), layer.pixmap.height());
        for (i, notice) in self.notifications.items.iter().rev().enumerate() {
            let x = pad + d.padding;
            let y = pad + top + header + i as f32 * notification_stride() - self.shade_scroll;
            let width = rect.w - 2.0 * d.padding;
            body.fill_round_rect(
                x,
                y,
                width,
                d.tile_height,
                config::design().shape.large,
                theme.surface_container,
            );
            let line_x = x + d.label_x;
            let text_width = (width - 2.0 * d.label_x - header).max(1.0);
            for (text, style, offset, color) in [
                (
                    notice.app.as_str(),
                    TextStyle::label_small(),
                    8.0,
                    theme.primary,
                ),
                (
                    notice.summary.as_str(),
                    TextStyle::title_small(),
                    28.0,
                    theme.on_surface,
                ),
                (
                    notice.body.as_str(),
                    TextStyle::body_small(),
                    54.0,
                    theme.on_surface_variant,
                ),
            ] {
                let text = fit_text(fonts, style, &text.replace('\n', " "), text_width);
                fonts.draw_text_role(&mut body, line_x, y + offset, style, &text, color);
            }
            fonts.draw_icon_centered(
                &mut body,
                x + width - header / 2.0,
                y + d.tile_height / 2.0,
                "close",
                config::design().statusbar.icon_size,
                theme.on_surface_variant,
            );
        }
        let viewport = round_rect_path(
            pad,
            pad + top + header,
            rect.w,
            (rect.h - top - header).max(1.0),
            0.0,
        );
        let rounded = round_rect_path(pad, pad, rect.w, rect.h, config::design().shape.extra_large);
        layer.blit_intersection(&body, &[&viewport, &rounded]);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn scrolling_dismisses_the_visible_notification_and_never_a_header_row() {
        let mut shell = Shell::new(vec![], ThemeSpec::default());
        shell.notifications = crate::notifications::Snapshot {
            available: true,
            items: (1..=8)
                .map(|id| crate::notifications::Notice {
                    id,
                    app: "App".into(),
                    summary: "Message".into(),
                    body: "Body".into(),
                })
                .collect(),
            ..Default::default()
        };
        shell.shade_scroll = 2.0 * notification_stride();
        let (w, h) = (640.0, 480.0);
        let rect = quick_settings_rect(w, h);
        let header = config::design().power.row_height;
        let x = rect.right() - config::design().quick_settings.padding - header / 2.0;
        let y =
            rect.y + quick_tiles_top() + header + config::design().quick_settings.tile_height / 2.0;
        assert_eq!(
            shell.notification_hit(w, h, (f64::from(x), f64::from(y)).into()),
            Some(NotificationHit::Dismiss(6))
        );
        assert_eq!(
            shell.notification_hit(
                w,
                h,
                (f64::from(x), f64::from(rect.y + quick_tiles_top() + 10.0)).into()
            ),
            Some(NotificationHit::Clear)
        );
        assert_eq!(
            shell.notification_hit(
                w,
                h,
                (f64::from(x), f64::from(rect.y + quick_tiles_top() - 10.0)).into()
            ),
            None
        );
        assert!(shell.notification_scroll_max(w, h) > shell.shade_scroll);
    }
}
