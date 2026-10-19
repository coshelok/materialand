//! One context action is shared by launcher tiles and dock applications.
use super::*;
#[derive(Clone)]
pub struct AppMenu {
    pub app: AppEntry,
    pub point: Point<f64, Logical>,
    pub error: bool,
}
pub fn app_menu_rect(w: f32, h: f32, point: Point<f64, Logical>) -> RectF {
    let width = 260.0_f32.min((w - 24.0).max(1.0));
    let height = 104.0_f32.min((h - 24.0).max(1.0));
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
impl Shell {
    pub fn open_app_menu(&mut self, app: AppEntry, point: Point<f64, Logical>) {
        self.cancel_presses();
        self.consume_menu_release = false;
        self.app_menu = Some(AppMenu {
            app,
            point,
            error: false,
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
                "Could not save dock changes"
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
        let button = app_menu_button(rect);
        surface.fill_round_rect(
            SHADOW_PAD + 8.0,
            SHADOW_PAD + 44.0,
            button.w,
            button.h,
            shape::MEDIUM,
            theme.secondary_container,
        );
        let pinned = self.is_pinned(&menu.app);
        fonts.draw_icon_centered(
            &mut surface,
            SHADOW_PAD + 30.0,
            SHADOW_PAD + 68.0,
            if pinned { "remove" } else { "add" },
            22.0,
            theme.on_secondary_container,
        );
        fonts.draw_text_role(
            &mut surface,
            SHADOW_PAD + 52.0,
            SHADOW_PAD + 58.0,
            TextStyle::LABEL_LARGE,
            if pinned {
                "Unpin from dock"
            } else {
                "Pin to dock"
            },
            theme.on_secondary_container,
        );
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
