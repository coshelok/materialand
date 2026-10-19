//! The same moving icon rectangles drive drawing, clicks and context menus.
use super::*;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HomeHit {
    App(usize),
    Previous,
    Next,
}
#[derive(Clone, Copy)]
pub struct HomeLayout {
    pub area: RectF,
    pub grid: RectF,
    pub columns: usize,
    pub per_page: usize,
    pub previous: RectF,
    pub next: RectF,
}
pub fn home_layout(w: f32, h: f32) -> HomeLayout {
    let top = config::TOPBAR_HEIGHT + 2.0 * config::TOPBAR_MARGIN + 12.0;
    let bottom = (h - config::DOCK_HEIGHT - config::DOCK_MARGIN - 16.0).max(top + 1.0);
    let width = (w - 48.0).clamp(1.0, 1120.0);
    let area = RectF {
        x: (w - width) / 2.0,
        y: top,
        w: width,
        h: (bottom - top).max(1.0),
    };
    let hero = if area.h > 430.0 { 172.0 } else { 96.0 };
    let grid = RectF {
        x: area.x,
        y: area.y + hero,
        w: area.w,
        h: (area.h - hero - 52.0).max(1.0),
    };
    let columns = (grid.w / 132.0).floor().max(1.0) as usize;
    let rows = (grid.h / 116.0).floor().max(1.0) as usize;
    let y = area.bottom() - 40.0;
    HomeLayout {
        area,
        grid,
        columns,
        per_page: columns * rows,
        previous: RectF {
            x: w / 2.0 - 96.0,
            y,
            w: 40.0,
            h: 36.0,
        },
        next: RectF {
            x: w / 2.0 + 56.0,
            y,
            w: 40.0,
            h: 36.0,
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
        let layout = home_layout(w, h);
        let slot = index % layout.per_page;
        let page = index / layout.per_page;
        let cell = layout.grid.w / layout.columns as f32;
        RectF {
            x: layout.grid.x
                + (slot % layout.columns) as f32 * cell
                + (cell - config::LAUNCHER_TILE) / 2.0
                + (page as f32 - self.home.position.value()) * (layout.grid.w + 32.0),
            y: layout.grid.y + (slot / layout.columns) as f32 * 116.0 + 8.0 + self.home_offset(),
            w: config::LAUNCHER_TILE,
            h: 104.0,
        }
    }
    pub fn home_hit(&self, w: f32, h: f32, point: Point<f64, Logical>) -> Option<HomeHit> {
        let layout = home_layout(w, h);
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
