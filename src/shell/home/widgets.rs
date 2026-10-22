//! Fixed home search and drag placement guides.
use super::layout::HomeLayout;
use super::*;
impl Shell {
    pub(super) fn paint_home_widgets(
        &self,
        surface: &mut Surface,
        fonts: &mut Fonts,
        theme: &Theme,
        layout: HomeLayout,
    ) {
        let d = config::design();
        let r = layout.search;
        let x = r.x - layout.area.x;
        let y = r.y - layout.area.y;
        surface.fill_round_rect(
            x,
            y,
            r.w,
            r.h,
            r.h / 2.0,
            Theme::with_alpha(theme.surface_container_high, 0.9),
        );
        fonts.draw_icon_centered(
            surface,
            x + r.h / 2.0,
            y + r.h / 2.0,
            "search",
            d.statusbar.icon_size,
            theme.on_surface_variant,
        );
        let text = fit_text(
            fonts,
            TextStyle::body_large(),
            "Search apps",
            (r.w - 2.0 * r.h).max(1.0),
        );
        fonts.draw_text_centered(
            surface,
            (x + r.w / 2.0, y + r.h / 2.0),
            TextStyle::body_large(),
            &text,
            theme.on_surface_variant,
        );
        fonts.draw_icon_centered(
            surface,
            x + r.w - r.h / 2.0,
            y + r.h / 2.0,
            "apps",
            d.statusbar.icon_size,
            theme.on_surface_variant,
        );
        if self.dragged_identity.is_some() {
            for row in 0..layout.rows {
                for col in 0..layout.columns {
                    let cell = workspace::Cell {
                        page: self.home.page,
                        column: col,
                        row,
                    };
                    let tile = layout.tile(cell, self.home.page as f32, 0.0);
                    surface.circle(
                        tile.x - layout.area.x + tile.w / 2.0,
                        tile.y - layout.area.y + tile.h / 2.0,
                        d.home.page_dot_radius,
                        theme.outline_variant,
                    );
                    if self.home.drop_hint == Some(layout.slot(cell)) {
                        surface.fill_round_rect(
                            tile.x - layout.area.x,
                            tile.y - layout.area.y,
                            tile.w,
                            tile.h,
                            d.shape.large,
                            theme.state_layer(theme.primary, d.state.pressed),
                        );
                    }
                }
            }
        }
    }
}
