//! The same moving icon rectangles drive drawing, clicks and context menus.
use super::*;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HomeHit {
    App(usize),
    Previous,
    Next,
    Search,
}
#[derive(Clone, Copy)]
pub struct HomeLayout {
    pub area: RectF,
    pub grid: RectF,
    pub columns: usize,
    pub per_page: usize,
    pub previous: RectF,
    pub next: RectF,
    pub search: RectF,
}
pub fn home_layout(w: f32, h: f32) -> HomeLayout {
    let design = crate::config::design();
    let top = top_inset() + design.home.top_gap;
    let bottom = (h - bottom_inset() - design.home.bottom_gap).max(top + 1.0);
    let width = (w - 2.0 * design.home.margin_x).clamp(1.0, design.home.max_width);
    let area = RectF {
        x: (w - width) / 2.0,
        y: top,
        w: width,
        h: (bottom - top).max(1.0),
    };
    let hero = if area.h > design.home.compact_threshold {
        design.home.hero_height
    } else {
        design.home.compact_hero_height
    };
    let grid = RectF {
        x: area.x,
        y: area.y + hero,
        w: area.w,
        h: (area.h - hero - design.home.footer_height).max(1.0),
    };
    let columns =
        ((grid.w / design.home.cell_width).floor().max(1.0) as usize).min(design.home.max_columns);
    let rows = (grid.h / design.home.cell_height).floor().max(1.0) as usize;
    let y = area.bottom() - design.home.nav_bottom;
    HomeLayout {
        area,
        grid,
        columns,
        per_page: columns * rows,
        previous: RectF {
            x: w / 2.0 - design.home.nav_offset,
            y,
            w: design.home.nav_width,
            h: design.home.nav_height,
        },
        next: RectF {
            x: w / 2.0 + design.home.nav_offset - design.home.nav_width,
            y,
            w: design.home.nav_width,
            h: design.home.nav_height,
        },
        search: RectF {
            x: area.x + (area.w - area.w.min(design.home.search_width)) / 2.0,
            y: area.bottom() - design.home.search_bottom - design.home.search_height,
            w: area.w.min(design.home.search_width),
            h: design.home.search_height,
        },
    }
}
impl Shell {
    pub fn home_pages(&self, w: f32, h: f32) -> usize {
        self.home
            .apps
            .len()
            .div_ceil(home_layout(w, h).per_page)
            .max(1)
    }
    pub fn home_tile_rect(&self, w: f32, h: f32, index: usize) -> RectF {
        let design = crate::config::design();
        let layout = home_layout(w, h);
        let slot = index % layout.per_page;
        let page = index / layout.per_page;
        let cell = layout.grid.w / layout.columns as f32;
        let (dx, dy) = self
            .home
            .apps
            .get(index)
            .and_then(|app| self.home.icon_motion.get(&app.identity()))
            .map(|(x, y)| (x.value(), y.value()))
            .unwrap_or((0.0, 0.0));
        RectF {
            x: layout.grid.x
                + (slot % layout.columns) as f32 * cell
                + (cell - design.launcher.tile_size) / 2.0
                + dx
                + (page as f32 - self.home.position.value())
                    * (layout.grid.w + design.home.page_gap),
            y: layout.grid.y
                + (slot / layout.columns) as f32 * design.home.cell_height
                + design.home.tile_offset_y
                + self.home_offset()
                + dy,
            w: design.launcher.tile_size,
            h: design.home.tile_height,
        }
    }
    pub fn home_hit(&self, w: f32, h: f32, point: Point<f64, Logical>) -> Option<HomeHit> {
        let layout = home_layout(w, h);
        if layout.search.contains(point) {
            return Some(HomeHit::Search);
        }
        if layout.grid.contains(point) {
            return (0..self.home.apps.len())
                .find(|i| self.home_tile_rect(w, h, *i).contains(point))
                .map(HomeHit::App);
        }
        if self.home_pages(w, h) > 1 {
            if layout.previous.contains(point) {
                return Some(HomeHit::Previous);
            }
            if layout.next.contains(point) {
                return Some(HomeHit::Next);
            }
        }
        None
    }
}
