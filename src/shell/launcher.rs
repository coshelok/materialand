//! Launcher layer and GPU animation.
use super::*;
impl Shell {
    pub(crate) fn scrim_buffer(&mut self, theme: &Theme) -> MemoryRenderBuffer {
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
        let design = crate::config::design();
        let p = progress.clamp(0.0, 1.0);
        let content_alpha = if tablet_mode() && design.launcher.fullscreen {
            1.0
        } else {
            p
        };
        let card = launcher_card(win_w, win_h);
        let pad = design.launcher.shadow_spread + design.launcher.shadow_extra_padding;
        // Top-left of the content layer in screen space.
        let ox = card.x - pad;
        let oy = card.y - pad;

        // Content layer: laid out at its final geometry, cached across frames.
        let layer_key = self.launcher_layer_key(win_w, win_h, theme);
        if !self.launcher_layer.select(layer_key) {
            let mut layer = Surface::new(
                (card.w + 2.0 * pad).ceil() as u32,
                (card.h + 2.0 * pad).ceil() as u32,
            );
            self.paint_launcher_layer(&mut layer, fonts, theme, win_w, win_h);
            let buffer = layer.to_memory_buffer();
            self.launcher_layer.store(CachedLayer {
                key: layer_key,
                surface: layer,
                buffer,
            });
        }

        // Container transform: the card morphs from the control that opened it
        // into its final rectangle while it fades in.
        let anim = self.launcher_placement(win_w, win_h, progress).target;
        let sx = (anim.w / card.w).clamp(0.01, design.motion.overshoot_limit);
        let sy = (anim.h / card.h).clamp(0.01, design.motion.overshoot_limit);

        if let Some(mask) = mask {
            // Radial reveal: the circular clip is applied on the CPU, so the
            // card and the press pieces are composited into one surface first.
            let cached = self.launcher_layer.as_ref().expect("just cached");
            let mut content = cached.surface.duplicate();
            let viewport = launcher_viewport(win_w, win_h);
            let mut grid = Surface::new(viewport.w.ceil() as u32, viewport.h.ceil() as u32);
            let mut origin = launcher_grid_origin(win_w, win_h);
            origin.1 -= self.launcher_scroll();
            for (slot, idx) in self.filtered_apps().into_iter().enumerate() {
                let (x, y) = launcher_tile_pos(origin, slot, launcher_columns(card.w));
                if y >= viewport.bottom() {
                    break;
                }
                if y + design.launcher.tile_size <= viewport.y {
                    continue;
                }
                self.draw_launcher_tile(
                    &mut grid,
                    fonts,
                    theme,
                    &self.apps[idx],
                    x - viewport.x,
                    y - viewport.y,
                    1.0,
                );
            }
            content.blit(
                &grid,
                SkTransform::from_translate(viewport.x - ox, viewport.y - oy),
                1.0,
                None,
            );

            for (press, (px, py)) in self.launcher_press_surfaces(fonts, theme, win_w, win_h) {
                content.blit(
                    &press,
                    SkTransform::from_translate(px - ox, py - oy),
                    1.0,
                    None,
                );
            }
            let mut s = Surface::new(win_w as u32, win_h as u32);
            s.clear(Theme::with_alpha(
                theme.scrim,
                design.launcher.scrim_alpha * p,
            ));
            let transform = SkTransform::from_scale(sx, sy)
                .post_translate(anim.x - pad * sx, anim.y - pad * sy);
            s.blit(&content, transform, content_alpha, None);
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
                    source: None,
                    buffer: self.scrim_buffer(theme),
                    loc: Point::from((0.0, 0.0)),
                    size: Some(Size::from((win_w.round() as i32, win_h.round() as i32))),
                },
                design.launcher.scrim_alpha * p,
            ));
        }
        {
            let layer = self.launcher_layer.as_ref().expect("just cached");
            let layer_w = layer.surface.width() as f32;
            let layer_h = layer.surface.pixmap.height() as f32;
            pieces.push((
                LayerFrame {
                    source: None,
                    buffer: layer.buffer.clone(),
                    loc: Point::from(((anim.x - pad * sx) as f64, (anim.y - pad * sy) as f64)),
                    size: Some(Size::from((
                        (layer_w * sx).round() as i32,
                        (layer_h * sy).round() as i32,
                    ))),
                },
                content_alpha,
            ));
        }
        let viewport = launcher_viewport(win_w, win_h);
        let cols = launcher_columns(card.w);
        let mut origin = launcher_grid_origin(win_w, win_h);
        origin.1 -= self.launcher_scroll();
        let filtered = self.filtered_apps();
        for (slot, idx) in filtered.iter().copied().enumerate() {
            let (x, y) = launcher_tile_pos(origin, slot, cols);
            if y >= viewport.bottom() {
                break;
            }
            if y + design.launcher.tile_size <= viewport.y {
                continue;
            }
            let app = self.apps[idx].clone();
            let buffer = self.app_tile(fonts, theme, &app, 1.0, false);
            let tile = RectF {
                x,
                y,
                w: design.launcher.tile_size,
                h: design.launcher.tile_size,
            };
            if let Some(layer) = LayerFrame::clipped(
                buffer,
                tile,
                viewport,
                self.launcher_placement(win_w, win_h, progress),
            ) {
                pieces.push((layer, content_alpha));
            }
        }
        for (press, (px, py)) in self.launcher_press_surfaces(fonts, theme, win_w, win_h) {
            // Keep the feedback glued to the scaled card while it animates.
            let loc = (
                anim.x as f64 + (px - card.x) as f64 * sx as f64,
                anim.y as f64 + (py - card.y) as f64 * sy as f64,
            );
            pieces.push((
                LayerFrame {
                    source: None,
                    buffer: press.to_memory_buffer(),
                    loc: Point::from(loc),
                    size: Some(Size::from((
                        (press.width() as f32 * sx).round() as i32,
                        (press.pixmap.height() as f32 * sy).round() as i32,
                    ))),
                },
                content_alpha,
            ));
        }
        let max = launcher_scroll_max(win_w, win_h, filtered.len());
        if max > 0.0 {
            let thumb = (viewport.h * viewport.h / (viewport.h + max))
                .max(design.launcher.scrollbar_min_height)
                .min(viewport.h);
            let width = design.launcher.scrollbar_width;
            let key = KeyHash::new()
                .u64(theme.cache_key())
                .f32(width)
                .f32(thumb)
                .finish();
            if !self.launcher_scrollbar.select(key) {
                let mut surface = Surface::new(width.ceil() as u32, thumb.ceil() as u32);
                surface.fill_round_rect(
                    0.0,
                    0.0,
                    width,
                    thumb,
                    width / 2.0,
                    Theme::with_alpha(theme.on_surface_variant, design.launcher.scrollbar_alpha),
                );
                let buffer = surface.to_memory_buffer();
                self.launcher_scrollbar.store(CachedLayer {
                    key,
                    surface,
                    buffer,
                });
            }
            let r = self
                .launcher_placement(win_w, win_h, progress)
                .map_rect(RectF {
                    x: card.right() - design.launcher.scrollbar_right,
                    y: viewport.y
                        + (viewport.h - thumb) * (self.launcher_scroll() / max).clamp(0.0, 1.0),
                    w: width,
                    h: thumb,
                });
            pieces.push((
                LayerFrame {
                    buffer: self.launcher_scrollbar.as_ref().unwrap().buffer.clone(),
                    source: None,
                    loc: (f64::from(r.x), f64::from(r.y)).into(),
                    size: Some((r.w.ceil() as i32, r.h.ceil() as i32).into()),
                },
                content_alpha,
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
        let design = crate::config::design();
        let card = launcher_card(win_w, win_h);
        // The elevation shadow is baked in: it scales with the container, so
        // it must not be redrawn on every animation frame.
        let pad = design.launcher.shadow_spread + design.launcher.shadow_extra_padding;
        let ox = card.x - pad;
        let oy = card.y - pad;
        let fullscreen = tablet_mode() && design.launcher.fullscreen;
        if !fullscreen {
            layer.shadow_round_rect(
                pad,
                pad,
                card.w,
                card.h,
                design.shape.extra_extra_large,
                design.launcher.shadow_spread,
                theme.shadow,
            );
        }
        layer.fill_round_rect(
            pad,
            pad,
            card.w,
            card.h,
            if fullscreen {
                0.0
            } else {
                design.shape.extra_extra_large
            },
            theme.surface_container_low,
        );
        if !fullscreen {
            layer.fill_round_rect(
                pad + card.w / 2.0 - design.launcher.handle_width / 2.0,
                pad + design.launcher.handle_y,
                design.launcher.handle_width,
                design.launcher.handle_height,
                design.launcher.handle_height / 2.0,
                theme.surface_container_highest,
            );
        }

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
            theme.surface_container_high,
        );
        fonts.draw_icon_centered(
            layer,
            spx + design.launcher.search_icon_x,
            spy + sp.h / 2.0,
            "search",
            design.launcher.search_icon_size,
            theme.on_surface_variant,
        );
        let query = if self.launcher_query.is_empty() {
            "Search your apps"
        } else {
            &self.launcher_query
        };
        let search_style = TextStyle::body_large();
        let query = fit_text(
            fonts,
            search_style,
            query,
            (sp.w - design.launcher.search_label_x - design.launcher.search_icon_x).max(1.0),
        );
        fonts.draw_text_role(
            layer,
            spx + design.launcher.search_label_x,
            spy + sp.h / 2.0 - search_style.line_height / 2.0,
            search_style,
            &query,
            if self.launcher_query.is_empty() {
                theme.on_surface_variant
            } else {
                theme.on_surface
            },
        );

        fonts.draw_text_role(
            layer,
            spx + design.launcher.section_label_x,
            spy + sp.h + design.launcher.section_label_y,
            TextStyle::label_large(),
            if self.launcher_query.is_empty() {
                "All apps"
            } else {
                "Search results"
            },
            theme.on_surface_variant,
        );
        let filtered = self.filtered_apps();
        if filtered.is_empty() {
            let msg = "No apps found";
            let msg_style = TextStyle::body_large();
            let mw = fonts.text_width_styled(msg_style, msg);
            fonts.draw_text_role(
                layer,
                card.x - ox + card.w / 2.0 - mw / 2.0,
                card.y - oy + design.launcher.empty_label_y,
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
        let design = crate::config::design();
        let mut out = Vec::new();
        let card = launcher_card(win_w, win_h);
        let cols = launcher_columns(card.w);
        let mut origin = launcher_grid_origin(win_w, win_h);
        origin.1 -= self.launcher_scroll();
        let viewport = launcher_viewport(win_w, win_h);
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
            if ty >= viewport.bottom() {
                break;
            }
            if ty + design.launcher.tile_size <= viewport.y {
                continue;
            }
            let mut s = Surface::new(
                design.launcher.tile_size as u32,
                design.launcher.tile_size as u32,
            );
            // Erase the resting tile underneath before drawing the squeezed one.
            s.clear(theme.surface_container_low);
            let scale = anim.scale.value();
            self.draw_launcher_tile(&mut s, fonts, theme, &self.apps[idx], 0.0, 0.0, scale);
            let clip = round_rect_path(
                0.0,
                0.0,
                design.launcher.tile_size,
                design.launcher.tile_size,
                design.shape.large,
            );
            self.draw_ripple(&mut s, key, &clip, theme.on_surface);
            let left = (viewport.x - tx).max(0.0).ceil();
            let top = (viewport.y - ty).max(0.0).ceil();
            let right = (viewport.right() - tx)
                .min(design.launcher.tile_size)
                .floor();
            let bottom = (viewport.bottom() - ty)
                .min(design.launcher.tile_size)
                .floor();
            if right <= left || bottom <= top {
                continue;
            }
            let mut cropped = Surface::new((right - left) as u32, (bottom - top) as u32);
            cropped.blit(&s, SkTransform::from_translate(-left, -top), 1.0, None);
            out.push((cropped, (tx + left, ty + top)));
        }
        out
    }
}
