//! Shared dock/home shortcut actions for launcher, dock and home tiles.
use super::*;
#[derive(Clone)]
pub struct AppMenu {
    pub app: AppEntry,
    pub point: Point<f64, Logical>,
    pub error: bool,
    pub press_home: bool,
    pub selected: usize,
}
pub fn app_menu_rect(w: f32, h: f32, point: Point<f64, Logical>) -> RectF {
    let width = 260.0_f32.min((w - 24.0).max(1.0));
    let height = 160.0_f32.min((h - 24.0).max(1.0));
    RectF {
        x: (point.x as f32 - width / 2.0).clamp(12.0, (w - width - 12.0).max(12.0)),
        y: (point.y as f32 - height - 12.0).clamp(12.0, (h - height - 12.0).max(12.0)),
        w: width,
        h: height,
    }
}
pub fn app_menu_button(card: RectF) -> RectF {
    RectF {
        x: card.x + 8.0,
        y: card.y + 44.0,
        w: card.w - 16.0,
        h: 48.0,
    }
}
pub fn app_menu_home_button(card: RectF) -> RectF {
    RectF {
        y: card.y + 100.0,
        ..app_menu_button(card)
    }
}
impl Shell {
    pub fn open_app_menu(&mut self, app: AppEntry, point: Point<f64, Logical>) {
        self.cancel_presses();
        self.consume_menu_release = false;
        self.app_menu = Some(AppMenu {
            app,
            point,
            error: false,
            press_home: false,
            selected: 0,
        });
        self.press_pin = false;
    }
    pub fn close_app_menu(&mut self) {
        self.app_menu = None;
        self.press_pin = false;
        self.consume_menu_release = false;
    }
    pub fn activate_pin_menu(&mut self) {
        let Some(menu) = &self.app_menu else {
            return;
        };
        let app = menu.app.clone();
        match self.toggle_pin(app) {
            Ok(()) => self.close_app_menu(),
            Err(error) => {
                tracing::warn!(%error, "could not save dock pins");
                if let Some(menu) = &mut self.app_menu {
                    menu.error = true;
                }
            }
        }
    }
    pub fn activate_home_menu(&mut self) {
        let Some(menu) = &self.app_menu else {
            return;
        };
        let app = menu.app.clone();
        match self.toggle_home_app(app) {
            Ok(()) => self.close_app_menu(),
            Err(error) => {
                tracing::warn!(%error, "could not save home shortcuts");
                if let Some(menu) = &mut self.app_menu {
                    menu.error = true;
                }
            }
        }
    }
    pub fn render_app_menu(
        &self,
        theme: &Theme,
        fonts: &mut Fonts,
        w: f32,
        h: f32,
        mask: Option<CircleMask>,
    ) -> Option<(MemoryRenderBuffer, Point<f64, Logical>)> {
        let menu = self.app_menu.as_ref()?;
        let rect = app_menu_rect(w, h, menu.point);
        let mut surface = Surface::new(
            (rect.w + 2.0 * SHADOW_PAD).ceil() as u32,
            (rect.h + 2.0 * SHADOW_PAD).ceil() as u32,
        );
        surface.shadow_round_rect(
            SHADOW_PAD,
            SHADOW_PAD,
            rect.w,
            rect.h,
            shape::LARGE,
            SHADOW_SPREAD,
        );
        surface.fill_round_rect(
            SHADOW_PAD,
            SHADOW_PAD,
            rect.w,
            rect.h,
            shape::LARGE,
            theme.surface_container_low,
        );
        let title = fit_text(
            fonts,
            TextStyle::TITLE_SMALL,
            if menu.error {
                "Could not save changes"
            } else {
                &menu.app.name
            },
            rect.w - 32.0,
        );
        fonts.draw_text_role(
            &mut surface,
            SHADOW_PAD + 16.0,
            SHADOW_PAD + 14.0,
            TextStyle::TITLE_SMALL,
            &title,
            theme.on_surface_variant,
        );
        for row in 0..2 {
            let button = if row == 0 {
                app_menu_button(rect)
            } else {
                app_menu_home_button(rect)
            };
            let y = button.y - rect.y + SHADOW_PAD;
            surface.fill_round_rect(
                SHADOW_PAD + 8.0,
                y,
                button.w,
                button.h,
                shape::MEDIUM,
                if menu.selected == row {
                    theme.secondary_container
                } else {
                    theme.surface_container
                },
            );
            let enabled = if row == 0 {
                self.is_pinned(&menu.app)
            } else {
                self.is_on_home(&menu.app)
            };
            fonts.draw_icon_centered(
                &mut surface,
                SHADOW_PAD + 30.0,
                y + 24.0,
                if row == 1 {
                    "home"
                } else if enabled {
                    "remove"
                } else {
                    "add"
                },
                22.0,
                theme.on_secondary_container,
            );
            let label = match (row, enabled) {
                (0, true) => "Unpin from dock",
                (0, false) => "Pin to dock",
                (_, true) => "Remove from home",
                (_, false) => "Add to home screen",
            };
            fonts.draw_text_role(
                &mut surface,
                SHADOW_PAD + 52.0,
                y + 14.0,
                TextStyle::LABEL_LARGE,
                label,
                theme.on_secondary_container,
            );
        }
        let buffer = surface.to_memory_buffer();
        Some(masked_from(
            &surface,
            mask,
            &buffer,
            (
                f64::from(rect.x - SHADOW_PAD),
                f64::from(rect.y - SHADOW_PAD),
            )
                .into(),
        ))
    }
}
