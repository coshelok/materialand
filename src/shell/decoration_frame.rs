//! A server titlebar and exterior shadow, never a backdrop behind client pixels.
use super::*;

pub(super) fn paint_frame(
    surface: &mut Surface,
    layout: &DecorLayout,
    theme: &Theme,
    focused: bool,
) {
    let design = crate::config::design();
    surface.shadow_round_rect(
        design.shadow.padding,
        design.shadow.padding,
        layout.frame.w,
        layout.frame.h,
        design.shape.extra_large,
        design.shadow.spread,
        theme.shadow,
    );
    let fill = Theme::mix(
        theme.surface_for_elevation(1),
        theme.secondary_container,
        if focused {
            design.window.focused_tint
        } else {
            design.window.inactive_tint
        },
    );
    // Clip the rounded outer shape to its titlebar. No body fill or outline is
    // painted underneath translucent terminals or client-owned rounded edges.
    let shape = round_rect_path(
        design.shadow.padding,
        design.shadow.padding,
        layout.frame.w,
        layout.frame.h,
        design.shape.extra_large,
    );
    let titlebar = round_rect_path(
        design.shadow.padding,
        design.shadow.padding,
        layout.titlebar.w,
        layout.titlebar.h,
        0.0,
    );
    surface.fill_path_clipped(&shape, &titlebar, fill);
    // The shadow primitive includes its interior; punch out the whole client
    // rectangle so even its translucent pixels see the scene below the window.
    let client = tiny_skia::Rect::from_xywh(
        design.shadow.padding + layout.content.x - layout.frame.x,
        design.shadow.padding + layout.content.y - layout.frame.y,
        layout.content.w,
        layout.content.h,
    );
    if let Some(client) = client {
        let paint = tiny_skia::Paint {
            blend_mode: tiny_skia::BlendMode::Clear,
            anti_alias: false,
            ..Default::default()
        };
        surface
            .pixmap
            .fill_rect(client, &paint, SkTransform::identity(), None);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transparent_client_has_no_system_backdrop_or_light_outline() {
        let design = crate::config::design();
        let layout = decor_layout(Rectangle::new((100, 100).into(), (320, 240).into()));
        for theme in [ThemeSpec::default().light(), ThemeSpec::default().dark()] {
            for focused in [false, true] {
                let mut surface = Surface::new(
                    (layout.frame.w + 2.0 * design.shadow.padding) as u32,
                    (layout.frame.h + 2.0 * design.shadow.padding) as u32,
                );
                paint_frame(&mut surface, &layout, &theme, focused);
                let x0 = (design.shadow.padding + layout.content.x - layout.frame.x) as u32;
                let y0 = (design.shadow.padding + layout.content.y - layout.frame.y) as u32;
                for y in y0..y0 + 240 {
                    for x in x0..x0 + 320 {
                        assert_eq!(
                            surface.pixmap.pixel(x, y).unwrap().alpha(),
                            0,
                            "client pixel ({x}, {y}) must be transparent"
                        );
                    }
                }
                assert_eq!(
                    surface
                        .pixmap
                        .pixel(x0 + 160, design.shadow.padding as u32 + 22)
                        .unwrap()
                        .alpha(),
                    255
                );
                // Below the titlebar, only an exterior black shadow may remain.
                let edge = surface.pixmap.pixel(x0 - 1, y0 + 120).unwrap();
                assert_eq!((edge.red(), edge.green(), edge.blue()), (0, 0, 0));
            }
        }
    }
}
