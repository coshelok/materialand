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
    let design = crate::config::design();
    let width = design
        .app_menu
        .width
        .min((w - 2.0 * design.app_menu.margin).max(1.0));
    let height = design
        .app_menu
        .height
        .min((h - 2.0 * design.app_menu.margin).max(1.0));
    RectF {
        x: (point.x as f32 - width / 2.0).clamp(
            design.app_menu.margin,
            (w - width - design.app_menu.margin).max(design.app_menu.margin),
        ),
        y: (point.y as f32 - height - design.app_menu.margin).clamp(
            design.app_menu.margin,
            (h - height - design.app_menu.margin).max(design.app_menu.margin),
        ),
        w: width,
        h: height,
    }
}
pub fn app_menu_button(card: RectF) -> RectF {
    let design = crate::config::design();
    RectF {
        x: card.x + design.app_menu.row_inset,
        y: card.y + design.app_menu.row_y,
        w: card.w - 2.0 * design.app_menu.row_inset,
        h: design.app_menu.row_height,
    }
}
pub fn app_menu_home_button(card: RectF) -> RectF {
    RectF {
        y: card.y + crate::config::design().app_menu.home_row_y,
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
        let design = crate::config::design();
        let menu = self.app_menu.as_ref()?;
        let rect = app_menu_rect(w, h, menu.point);
        let mut surface = Surface::new(
            (rect.w + 2.0 * design.shadow.padding).ceil() as u32,
            (rect.h + 2.0 * design.shadow.padding).ceil() as u32,
        );
        surface.shadow_round_rect(
            design.shadow.padding,
            design.shadow.padding,
            rect.w,
            rect.h,
            design.shape.large,
            design.shadow.spread,
            theme.shadow,
        );
        surface.fill_round_rect(
            design.shadow.padding,
            design.shadow.padding,
            rect.w,
            rect.h,
            design.shape.large,
            theme.surface_container_low,
        );
        let title = fit_text(
            fonts,
            TextStyle::title_small(),
            if menu.error {
                "Could not save changes"
            } else {
                &menu.app.name
            },
            rect.w - 2.0 * design.app_menu.title_x,
        );
        fonts.draw_text_role(
            &mut surface,
            design.shadow.padding + design.app_menu.title_x,
            design.shadow.padding + design.app_menu.title_y,
            TextStyle::title_small(),
            &title,
            theme.on_surface_variant,
        );
        for row in 0..2 {
            let button = if row == 0 {
                app_menu_button(rect)
            } else {
                app_menu_home_button(rect)
            };
            let y = button.y - rect.y + design.shadow.padding;
            surface.fill_round_rect(
                design.shadow.padding + design.app_menu.row_inset,
                y,
                button.w,
                button.h,
                design.shape.medium,
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
                design.shadow.padding + design.app_menu.icon_x,
                y + button.h / 2.0,
                if row == 1 {
                    "home"
                } else if enabled {
                    "remove"
                } else {
                    "add"
                },
                design.app_menu.icon_size,
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
                design.shadow.padding + design.app_menu.label_x,
                y + design.app_menu.label_y,
                TextStyle::label_large(),
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
                f64::from(rect.x - design.shadow.padding),
                f64::from(rect.y - design.shadow.padding),
            )
                .into(),
        ))
    }
}
