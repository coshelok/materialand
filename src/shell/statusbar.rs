//! Compact Android-style status area, independent of the desktop-mode topbar.
use super::*;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StatusAction {
    Clock,
    Notifications,
    Wifi,
    Bluetooth,
    Shade,
}
pub fn statusbar_rect(w: f32) -> RectF {
    RectF {
        x: 0.0,
        y: 0.0,
        w: w.max(1.0),
        h: config::design().statusbar.height,
    }
}
pub fn statusbar_button(w: f32, action: StatusAction) -> RectF {
    let d = &config::design().statusbar;
    let scale = ((w - 2.0 * d.padding).max(1.0)
        / (d.clock_width + 3.0 * d.item_width + d.battery_width))
        .min(1.0);
    let item = d.item_width * scale;
    let clock = d.clock_width * scale;
    let battery = d.battery_width * scale;
    let (x, width) = match action {
        StatusAction::Clock => (d.padding, clock),
        StatusAction::Notifications => (d.padding + clock, item),
        StatusAction::Wifi => (w - d.padding - battery - 2.0 * item, item),
        StatusAction::Bluetooth => (w - d.padding - battery - item, item),
        StatusAction::Shade => (w - d.padding - battery, battery),
    };
    RectF {
        x,
        y: 0.0,
        w: width,
        h: d.height,
    }
}
pub fn statusbar_hit(w: f32, pos: Point<f64, Logical>) -> Option<StatusAction> {
    if !tablet_mode() || !statusbar_rect(w).contains(pos) {
        return None;
    }
    Some(
        [
            StatusAction::Clock,
            StatusAction::Notifications,
            StatusAction::Wifi,
            StatusAction::Bluetooth,
        ]
        .into_iter()
        .find(|a| statusbar_button(w, *a).contains(pos))
        .unwrap_or(StatusAction::Shade),
    )
}
impl Shell {
    pub fn statusbar_visible(&self, fullscreen: bool, overview: bool) -> bool {
        tablet_mode()
            && (!fullscreen
                || overview
                || self.launcher_open
                || self.quick_settings_open
                || self.calendar.open
                || self.power_open
                || self.brightness_open
                || self.quick_settings_progress() > 0.001
                || self.launcher_progress() > 0.001
                || self.calendar_progress() > 0.001
                || self.power_progress() > 0.001
                || self.brightness_progress() > 0.001)
    }
    pub fn render_statusbar(
        &mut self,
        theme: &Theme,
        fonts: &mut Fonts,
        w: f32,
    ) -> (MemoryRenderBuffer, Point<f64, Logical>) {
        let d = &config::design().statusbar;
        let status = &self.system_status;
        let key = KeyHash::new()
            .u64(theme.cache_key())
            .u32(w.to_bits())
            .str(&self.clock)
            .bool(status.wifi.available)
            .bool(status.wifi.enabled)
            .bool(status.bluetooth.available)
            .bool(status.bluetooth.enabled)
            .str(&format!("{:?}", status.battery))
            .bool(self.notifications.available)
            .bool(!self.notifications.items.is_empty())
            .finish();
        if self.statusbar_cache.select(key) {
            return (
                self.statusbar_cache.as_ref().unwrap().buffer.clone(),
                (0.0, 0.0).into(),
            );
        }
        let mut s = Surface::new(w.ceil() as u32, d.height.ceil() as u32);
        s.fill_round_rect(
            0.0,
            0.0,
            w,
            d.height,
            0.0,
            Theme::with_alpha(theme.surface_container_low, d.background_alpha),
        );
        let clock = statusbar_button(w, StatusAction::Clock);
        let text = fit_text(fonts, TextStyle::label_large(), &self.clock, clock.w);
        fonts.draw_text_role(
            &mut s,
            clock.x,
            (d.height - TextStyle::label_large().line_height) / 2.0,
            TextStyle::label_large(),
            &text,
            theme.on_surface,
        );
        for (action, glyph, active) in [
            (
                StatusAction::Notifications,
                if self.notifications.available {
                    "notifications_none"
                } else {
                    "notifications_off"
                },
                self.notifications.available,
            ),
            (
                StatusAction::Wifi,
                if status.wifi.available && status.wifi.enabled {
                    "wifi"
                } else {
                    "wifi_off"
                },
                status.wifi.available && status.wifi.enabled,
            ),
            (
                StatusAction::Bluetooth,
                if status.bluetooth.available && status.bluetooth.enabled {
                    "bluetooth"
                } else {
                    "bluetooth_disabled"
                },
                status.bluetooth.available && status.bluetooth.enabled,
            ),
        ] {
            let r = statusbar_button(w, action);
            fonts.draw_icon_centered(
                &mut s,
                r.x + r.w / 2.0,
                r.h / 2.0,
                glyph,
                d.icon_size.min(r.w),
                if active {
                    theme.on_surface
                } else {
                    theme.on_surface_variant
                },
            );
            if action == StatusAction::Notifications && !self.notifications.items.is_empty() {
                s.circle(
                    r.x + r.w / 2.0 + d.icon_size / 3.0,
                    r.h / 3.0,
                    d.icon_size / 6.0,
                    theme.primary,
                );
            }
        }
        let r = statusbar_button(w, StatusAction::Shade);
        let (icon, percent) = status
            .battery
            .as_ref()
            .map(|b| {
                (
                    if b.charging {
                        "battery_charging_full"
                    } else if b.percent <= 15 {
                        "battery_alert"
                    } else {
                        "battery_full"
                    },
                    format!("{}%", b.percent),
                )
            })
            .unwrap_or(("battery_unknown", "—".into()));
        fonts.draw_icon_centered(
            &mut s,
            r.x + d.icon_size / 2.0,
            r.h / 2.0,
            icon,
            d.icon_size,
            theme.on_surface,
        );
        let percent = fit_text(
            fonts,
            TextStyle::label_medium(),
            &percent,
            (r.w - d.icon_size).max(1.0),
        );
        fonts.draw_text_role(
            &mut s,
            r.x + d.icon_size,
            (r.h - TextStyle::label_medium().line_height) / 2.0,
            TextStyle::label_medium(),
            &percent,
            theme.on_surface,
        );
        let buffer = s.to_memory_buffer();
        self.statusbar_cache.store(CachedLayer {
            key,
            surface: s,
            buffer: buffer.clone(),
        });
        (buffer, (0.0, 0.0).into())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn status_buttons_fit_small_screens_and_reserve_their_height() {
        for w in [320.0, 640.0, 1280.0] {
            let mut last = 0.0;
            for action in [
                StatusAction::Clock,
                StatusAction::Notifications,
                StatusAction::Wifi,
                StatusAction::Bluetooth,
                StatusAction::Shade,
            ] {
                let r = statusbar_button(w, action);
                assert!(r.x >= last && r.right() <= w);
                assert_eq!(
                    statusbar_hit(w, (f64::from(r.x + r.w / 2.0), f64::from(r.h / 2.0)).into()),
                    Some(action)
                );
                last = r.right();
            }
        }
        assert_eq!(top_inset(), config::design().statusbar.height);
    }
}
