//! Clock and local app search use the same home geometry as pointer routing.
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
        let offset = self.home_offset();
        if layout.area.h > d.home.compact_threshold {
            let date = fit_text(
                fonts,
                TextStyle::title_medium(),
                &self.calendar.now.date_text(),
                layout.area.w,
            );
            let date_w = fonts.text_width_styled(TextStyle::title_medium(), &date);
            fonts.draw_text_role(
                surface,
                (layout.area.w - date_w) / 2.0,
                offset,
                TextStyle::title_medium(),
                &date,
                theme.on_surface,
            );
            let width = d.home.widget_width.min(layout.area.w);
            let cell = ((width - d.home.widget_gap) / 2.0).max(1.0);
            let style = TextStyle {
                size: d.home.clock_size.min(cell * 0.6),
                line_height: d.home.clock_line_height,
                ..TextStyle::headline_small()
            };
            for (index, text) in self.clock.split(':').take(2).enumerate() {
                let x = (layout.area.w - width) / 2.0 + index as f32 * (cell + d.home.widget_gap);
                let y = offset + d.home.widget_top;
                let fill = if index == 0 {
                    theme.primary_fixed
                } else {
                    theme.secondary_fixed
                };
                surface.fill_round_rect(
                    x,
                    y,
                    cell,
                    d.home.widget_height,
                    d.home.widget_radius,
                    fill,
                );
                let text_w = fonts.text_width_styled(style, text);
                fonts.draw_text_role(
                    surface,
                    x + (cell - text_w) / 2.0,
                    y + (d.home.widget_height - style.line_height) / 2.0,
                    style,
                    text,
                    if index == 0 {
                        theme.on_primary_fixed
                    } else {
                        theme.on_secondary_fixed
                    },
                );
            }
        } else {
            let style = TextStyle {
                size: d.home.compact_clock_size,
                line_height: d.home.clock_line_height,
                ..TextStyle::headline_small()
            };
            fonts.draw_text_role(
                surface,
                d.home.clock_x,
                offset,
                style,
                &self.clock,
                theme.on_surface,
            );
        }
        let r = layout.search;
        let x = r.x - layout.area.x;
        let y = r.y - layout.area.y;
        surface.fill_round_rect(x, y, r.w, r.h, r.h / 2.0, theme.secondary_fixed);
        fonts.draw_icon_centered(
            surface,
            x + r.h / 2.0,
            y + r.h / 2.0,
            "search",
            d.statusbar.icon_size,
            theme.on_secondary_fixed,
        );
        fonts.draw_text_role(
            surface,
            x + r.h,
            y + (r.h - TextStyle::body_large().line_height) / 2.0,
            TextStyle::body_large(),
            "Search apps",
            theme.on_secondary_fixed,
        );
        fonts.draw_icon_centered(
            surface,
            x + r.w - r.h / 2.0,
            y + r.h / 2.0,
            "apps",
            d.statusbar.icon_size,
            theme.on_secondary_fixed,
        );
    }
}
