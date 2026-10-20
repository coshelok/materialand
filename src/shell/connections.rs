//! Real network and paired-device controls inside the system shade.
use super::*;

impl Shell {
    pub fn set_system_status(&mut self, status: crate::system::Status) {
        for (action, on) in [
            (QuickAction::Wifi, status.wifi.enabled),
            (QuickAction::Bluetooth, status.bluetooth.enabled),
        ] {
            let spring = &mut self.switches[quick_action_index(action)];
            spring.set_target(if on { 1.0 } else { 0.0 });
            if self.reduce_motion {
                spring.finish();
            }
        }
        self.system_status = status;
    }
}

impl Shell {
    pub(super) fn paint_connections(
        &self,
        layer: &mut Surface,
        fonts: &mut Fonts,
        theme: &Theme,
        rect: RectF,
        pad: f32,
    ) {
        let wifi = self.connection_page == Some(QuickAction::Wifi);
        let top = quick_tiles_top();
        let row_h = config::design().power.row_height;
        let heading = fit_text(
            fonts,
            TextStyle::title_small(),
            if wifi {
                "Wi-Fi networks · Open connection settings"
            } else {
                "Paired devices · Open pairing settings"
            },
            rect.w - 48.0,
        );
        fonts.draw_text_role(
            layer,
            pad + 24.0,
            pad + top,
            TextStyle::title_small(),
            &heading,
            theme.primary,
        );
        let mut body = Surface::new(layer.pixmap.width(), layer.pixmap.height());
        let labels: Vec<_> = if wifi {
            self.system_status
                .networks
                .iter()
                .map(|n| {
                    (
                        n.ssid.clone(),
                        format!(
                            "{}% · {}{}",
                            n.signal,
                            if n.active {
                                "Connected"
                            } else if n.secure {
                                "Secured"
                            } else {
                                "Open"
                            },
                            if n.secure && !n.active {
                                " · Use settings for passwords"
                            } else {
                                ""
                            }
                        ),
                    )
                })
                .collect()
        } else {
            self.system_status
                .devices
                .iter()
                .map(|d| {
                    (
                        d.name.clone(),
                        if d.connected {
                            "Connected · Click to disconnect"
                        } else {
                            "Paired · Click to connect"
                        }
                        .into(),
                    )
                })
                .collect()
        };
        if labels.is_empty() {
            fonts.draw_text_role(
                &mut body,
                pad + 24.0,
                pad + top + 56.0,
                TextStyle::body_medium(),
                if wifi {
                    "No networks available"
                } else {
                    "No paired devices"
                },
                theme.on_surface_variant,
            );
        }
        for (i, (name, status)) in labels.iter().enumerate() {
            let y = pad + top + 48.0 + i as f32 * row_h - self.shade_scroll;
            let name = fit_text(fonts, TextStyle::title_small(), name, rect.w - 48.0);
            fonts.draw_text_role(
                &mut body,
                pad + 24.0,
                y + 4.0,
                TextStyle::title_small(),
                &name,
                theme.on_surface,
            );
            let status = fit_text(fonts, TextStyle::body_small(), status, rect.w - 48.0);
            fonts.draw_text_role(
                &mut body,
                pad + 24.0,
                y + 28.0,
                TextStyle::body_small(),
                &status,
                theme.on_surface_variant,
            );
        }
        let viewport = round_rect_path(
            pad,
            pad + top + 48.0,
            rect.w,
            (rect.h - top - 48.0).max(1.0),
            0.0,
        );
        let rounded = round_rect_path(pad, pad, rect.w, rect.h, config::design().shape.extra_large);
        layer.blit_intersection(&body, &[&viewport, &rounded]);
        if let Some(error) = &self.system_status.error {
            let error = fit_text(fonts, TextStyle::body_small(), error, rect.w - 48.0);
            fonts.draw_text_role(
                layer,
                pad + 24.0,
                pad + rect.h - 22.0,
                TextStyle::body_small(),
                &error,
                theme.error,
            );
        }
    }
}
