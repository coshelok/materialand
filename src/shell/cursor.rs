//! Named cursor fallback; client-provided cursor surfaces use the normal renderer.
use super::*;
use smithay::input::pointer::CursorIcon;
impl Shell {
    pub fn render_cursor(
        &mut self,
        theme: &Theme,
        icon: CursorIcon,
    ) -> (MemoryRenderBuffer, Point<i32, Logical>) {
        let design = crate::config::design();
        let centered = matches!(
            icon,
            CursorIcon::Text
                | CursorIcon::VerticalText
                | CursorIcon::NsResize
                | CursorIcon::EwResize
                | CursorIcon::ColResize
                | CursorIcon::RowResize
                | CursorIcon::Move
                | CursorIcon::AllScroll
        );
        let hotspot = if centered {
            (
                (design.cursor.size / 2.0).round() as i32,
                (design.cursor.size / 2.0).round() as i32,
            )
                .into()
        } else {
            (0, 0).into()
        };
        if let Some((dark, cached, buffer)) = &self.cursor_cache {
            if *dark == theme.is_dark && *cached == icon {
                return (buffer.clone(), hotspot);
            }
        }
        let mut canvas = Surface::new(design.cursor.size as u32, design.cursor.size as u32);
        let fill = if theme.is_dark {
            crate::theme::parse_role_color(&design.cursor.fill_dark)
                .expect("validated cursor color")
        } else {
            crate::theme::parse_role_color(&design.cursor.fill_light)
                .expect("validated cursor color")
        };
        let outline = if theme.is_dark {
            crate::theme::parse_role_color(&design.cursor.outline_dark)
                .expect("validated cursor color")
        } else {
            crate::theme::parse_role_color(&design.cursor.outline_light)
                .expect("validated cursor color")
        };
        let path = if matches!(icon, CursorIcon::Text | CursorIcon::VerticalText) {
            let mut path = PathBuilder::new();
            path.move_to(11.0, 5.0);
            path.line_to(21.0, 5.0);
            path.move_to(16.0, 5.0);
            path.line_to(16.0, 27.0);
            path.move_to(11.0, 27.0);
            path.line_to(21.0, 27.0);
            path.finish().expect("cursor path")
        } else if centered {
            let mut path = PathBuilder::new();
            if !matches!(icon, CursorIcon::EwResize | CursorIcon::ColResize) {
                path.move_to(16.0, 3.0);
                path.line_to(16.0, 29.0);
                path.move_to(10.0, 9.0);
                path.line_to(16.0, 3.0);
                path.line_to(22.0, 9.0);
                path.move_to(10.0, 23.0);
                path.line_to(16.0, 29.0);
                path.line_to(22.0, 23.0);
            }
            if !matches!(icon, CursorIcon::NsResize | CursorIcon::RowResize) {
                path.move_to(3.0, 16.0);
                path.line_to(29.0, 16.0);
                path.move_to(9.0, 10.0);
                path.line_to(3.0, 16.0);
                path.line_to(9.0, 22.0);
                path.move_to(23.0, 10.0);
                path.line_to(29.0, 16.0);
                path.line_to(23.0, 22.0);
            }
            path.finish().expect("cursor path")
        } else {
            crate::graphics::canvas::polygon_path(&[
                (1.5, 1.5),
                (1.5, 24.0),
                (7.0, 18.0),
                (11.0, 27.5),
                (15.5, 25.5),
                (11.5, 16.0),
                (19.5, 16.0),
            ])
        };
        let factor = design.cursor.size / 32.0;
        let path = path
            .transform(SkTransform::from_scale(factor, factor))
            .expect("valid cursor scale");
        canvas.stroke_path(&path, design.cursor.outline_width * factor, outline);
        if centered {
            canvas.stroke_path(&path, design.cursor.stroke_width * factor, fill);
        } else {
            canvas.fill_path(&path, fill);
        }
        let buffer = canvas.to_memory_buffer();
        self.cursor_cache = Some((theme.is_dark, icon, buffer.clone()));
        (buffer, hotspot)
    }
}
