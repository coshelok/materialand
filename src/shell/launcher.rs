//! Launcher layer and GPU animation.
use super::*;
impl Shell {
    pub(super) fn scrim_buffer(&mut self, theme: &Theme) -> MemoryRenderBuffer {
        let key = theme.cache_key();
        if let Some((k, buf)) = &self.scrim_cache {
            if *k == key {
                return buf.clone();
            }
        }
        let mut s = Surface::new(1, 1);
        s.clear(theme.scrim);
        let buf = s.to_memory_buffer();
        self.scrim_cache = Some((key, buf.clone()));
        buf
    }
    pub fn render_launcher(
        &mut self,
        theme: &Theme,
        fonts: &mut Fonts,
        win_w: f32,
        win_h: f32,
        progress: f32,
        mask: Option<CircleMask>,
    ) -> OverlayFrame {
        let p = progress.clamp(0.0, 1.0);
        let ease = CubicBezier::EMPHASIZED_DECELERATE.eval(p);
        let card = launcher_card(win_w, win_h);
        let origin = self.launcher_origin.unwrap_or(RectF {
            x: 0.0,
            y: 0.0,
            w: config::TOPBAR_HEIGHT,
            h: config::TOPBAR_HEIGHT,
        });
        const LAUNCHER_SHADOW: f32 = 18.0;
        let pad = LAUNCHER_SHADOW + 2.0;
        // Top-left of the content layer in screen space.
        let ox = card.x - pad;
        let oy = card.y - pad;

        // Content layer: laid out at its final geometry, cached across frames.
        let layer_key = self.launcher_layer_key(win_w, win_h, theme);
        if self
            .launcher_layer
            .as_ref()
            .map(|l| l.key != layer_key)
            .unwrap_or(true)
        {
            let mut layer = Surface::new(
                (card.w + 2.0 * pad).ceil() as u32,
                (card.h + 2.0 * pad).ceil() as u32,
            );
            self.paint_launcher_layer(&mut layer, fonts, theme, win_w, win_h);
            let buffer = layer.to_memory_buffer();
            self.launcher_layer = Some(CachedLayer {
                key: layer_key,
                surface: layer,
                buffer,
            });
        }

        // Container transform: the card morphs from the control that opened it
        // into its final rectangle while it fades in.
        let lerp = |a: f32, b: f32| a + (b - a) * ease;
        let anim = RectF {
            x: lerp(origin.x, card.x),
            y: lerp(origin.y, card.y),
            w: lerp(origin.w, card.w),
            h: lerp(origin.h, card.h),
        };
        let sx = (anim.w / card.w).clamp(0.01, 1.0);
        let sy = (anim.h / card.h).clamp(0.01, 1.0);

        if let Some(mask) = mask {
            // Radial reveal: the circular clip is applied on the CPU, so the
            // card and the press pieces are composited into one surface first.
            let cached = self.launcher_layer.as_ref().expect("just cached");
            let mut content = cached.surface.duplicate();
            for (press, (px, py)) in self.launcher_press_surfaces(fonts, theme, win_w, win_h) {
                content.blit(
                    &press,
                    SkTransform::from_translate(px - ox, py - oy),
                    1.0,
                    None,
                );
            }
            let mut s = Surface::new(win_w as u32, win_h as u32);
            s.clear(Theme::with_alpha(theme.scrim, 0.45 * ease));
            let transform = SkTransform::from_scale(sx, sy)
                .post_translate(anim.x - pad * sx, anim.y - pad * sy);
            s.blit(&content, transform, ease.clamp(0.0, 1.0), None);
            let (buffer, loc) = masked_from(
                &s,
                Some(mask),
                &s.to_memory_buffer(),
                Point::from((0.0, 0.0)),
            );
            return OverlayFrame::single(buffer, loc, None, 1.0);
        }

        // GPU path: a stretched 1x1 scrim, the scaled card layer, then one
        // small overlay per pressed tile.
        let mut pieces: Vec<(LayerFrame, f32)> = Vec::with_capacity(2);
        if p > 0.001 {
            pieces.push((
                LayerFrame {
                    buffer: self.scrim_buffer(theme),
                    loc: Point::from((0.0, 0.0)),
                    size: Some(Size::from((win_w.round() as i32, win_h.round() as i32))),
                },
                0.45 * ease,
            ));
        }
        {
            let layer = self.launcher_layer.as_ref().expect("just cached");
            let layer_w = layer.surface.width() as f32;
            let layer_h = layer.surface.pixmap.height() as f32;
            pieces.push((
                LayerFrame {
                    buffer: layer.buffer.clone(),
                    loc: Point::from(((anim.x - pad * sx) as f64, (anim.y - pad * sy) as f64)),
                    size: Some(Size::from((
                        (layer_w * sx).round() as i32,
                        (layer_h * sy).round() as i32,
                    ))),
                },
                ease.clamp(0.0, 1.0),
            ));
        }
        for (press, (px, py)) in self.launcher_press_surfaces(fonts, theme, win_w, win_h) {
            // Keep the feedback glued to the scaled card while it animates.
            let loc = (
                anim.x as f64 + (px - card.x) as f64 * sx as f64,
                anim.y as f64 + (py - card.y) as f64 * sy as f64,
            );
            pieces.push((
                LayerFrame {
                    buffer: press.to_memory_buffer(),
                    loc: Point::from(loc),
                    size: Some(Size::from((
                        (config::LAUNCHER_TILE * sx).round() as i32,
                        (config::LAUNCHER_TILE * sy).round() as i32,
                    ))),
                },
                ease.clamp(0.0, 1.0),
            ));
        }
        OverlayFrame { pieces }
    }
    pub(super) fn launcher_layer_key(&self, win_w: f32, win_h: f32, theme: &Theme) -> u64 {
        let mut key = KeyHash::new()
            .i32(win_w.round() as i32)
            .i32(win_h.round() as i32)
            .u64(theme.cache_key())
            .str(&self.launcher_query);
        let query = self.launcher_query.to_lowercase();
        for app in &self.apps {
            if query.is_empty() || app.name.to_lowercase().contains(&query) {
                key = key.str(&app.icon).str(&app.name);
            }
        }
        key.finish()
    }
    pub(super) fn paint_launcher_layer(
        &self,
        layer: &mut Surface,
        fonts: &mut Fonts,
        theme: &Theme,
        win_w: f32,
        win_h: f32,
    ) {
        let card = launcher_card(win_w, win_h);
        // The elevation shadow is baked in: it scales with the container, so
        // it must not be redrawn on every animation frame.
        const LAUNCHER_SHADOW: f32 = 18.0;
        let pad = LAUNCHER_SHADOW + 2.0;
        let ox = card.x - pad;
        let oy = card.y - pad;
        layer.shadow_round_rect(
            card.x - ox,
            card.y - oy,
            card.w,
            card.h,
            config::LAUNCHER_RADIUS,
            LAUNCHER_SHADOW,
        );
        layer.fill_round_rect(
            card.x - ox,
            card.y - oy,
            card.w,
            card.h,
            config::LAUNCHER_RADIUS,
            theme.surface_container_low,
        );
        layer.stroke_round_rect(
            card.x - ox + 0.5,
            card.y - oy + 0.5,
            card.w - 1.0,
            card.h - 1.0,
            config::LAUNCHER_RADIUS,
            1.0,
            Theme::with_alpha(theme.outline, 0.1),
        );

        // Drag handle (Android-style).
        layer.fill_round_rect(
            card.x - ox + card.w / 2.0 - 16.0,
            card.y - oy + 12.0,
            32.0,
            4.0,
            2.0,
            theme.surface_container_highest,
        );

        // Search pill.
        let sp = search_rect(win_w, win_h);
        let spx = sp.x - ox;
        let spy = sp.y - oy;
        layer.fill_round_rect(
            spx,
            spy,
            sp.w,
            sp.h,
            sp.h / 2.0,
            theme.surface_container_highest,
        );
        fonts.draw_icon_centered(
            layer,
            spx + 24.0,
            spy + sp.h / 2.0,
            "search",
            24.0,
            theme.on_surface_variant,
        );
        let query = if self.launcher_query.is_empty() {
            "Search your apps"
        } else {
            &self.launcher_query
        };
        let search_style = TextStyle::BODY_LARGE;
        fonts.draw_text_role(
            layer,
            spx + 52.0,
            spy + sp.h / 2.0 - search_style.line_height / 2.0,
            search_style,
            query,
            if self.launcher_query.is_empty() {
                Theme::with_alpha(theme.on_surface_variant, 0.6)
            } else {
                theme.on_surface
            },
        );

        let cols = launcher_columns(card.w);
        let origin_g = launcher_grid_origin(win_w, win_h);
        let filtered = self.filtered_apps();
        for (slot, &idx) in filtered.iter().enumerate() {
            let (tx, ty) = launcher_tile_pos(origin_g, slot, cols);
            if ty + config::LAUNCHER_TILE > card.bottom() - 16.0 {
                break;
            }
            let app = &self.apps[idx];
            self.draw_launcher_tile(layer, fonts, theme, app, tx - ox, ty - oy, 1.0);
        }

        if filtered.is_empty() {
            let msg = "No apps found";
            let msg_style = TextStyle::BODY_LARGE;
            let mw = fonts.text_width_styled(msg_style, msg);
            fonts.draw_text_role(
                layer,
                card.x - ox + card.w / 2.0 - mw / 2.0,
                card.y - oy + 180.0,
                msg_style,
                msg,
                theme.on_surface_variant,
            );
        }
    }
    pub(super) fn launcher_press_surfaces(
        &self,
        fonts: &mut Fonts,
        theme: &Theme,
        win_w: f32,
        win_h: f32,
    ) -> Vec<(Surface, (f32, f32))> {
        let mut out = Vec::new();
        let card = launcher_card(win_w, win_h);
        let cols = launcher_columns(card.w);
        let origin = launcher_grid_origin(win_w, win_h);
        let filtered = self.filtered_apps();
        for (slot, &idx) in filtered.iter().enumerate() {
            let key = PressKey::Launcher(slot);
            let Some(anim) = self.press.get(&key) else {
                continue;
            };
            let ripple_active = anim.ripple.as_ref().is_some_and(|r| !r.progress.is_done());
            if !anim.pressed && !ripple_active && anim.scale.settled() {
                continue;
            }
            let (tx, ty) = launcher_tile_pos(origin, slot, cols);
            if ty + config::LAUNCHER_TILE > card.bottom() - 16.0 {
                break;
            }
            let mut s = Surface::new(config::LAUNCHER_TILE as u32, config::LAUNCHER_TILE as u32);
            // Erase the resting tile underneath before drawing the squeezed one.
            s.clear(theme.surface_container_low);
            let scale = anim.scale.value();
            self.draw_launcher_tile(&mut s, fonts, theme, &self.apps[idx], 0.0, 0.0, scale);
            let clip = round_rect_path(
                0.0,
                0.0,
                config::LAUNCHER_TILE,
                config::LAUNCHER_TILE,
                shape::LARGE,
            );
            self.draw_ripple(&mut s, key, &clip, theme.on_surface);
            out.push((s, (tx, ty)));
        }
        out
    }
}
