//! Full-output workspace geometry with every cell available for shortcuts.
use super::workspace::Cell;
use super::*;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HomeHit {
    App(usize),
    Previous,
    Next,
    Search,
    Calendar,
}
#[derive(Clone, Copy)]
pub struct HomeLayout {
    pub area: RectF,
    pub grid: RectF,
    pub columns: usize,
    pub rows: usize,
    pub per_page: usize,
    pub previous: RectF,
    pub next: RectF,
    pub search: RectF,
    pub glance: RectF,
}
pub fn home_layout(w: f32, h: f32) -> HomeLayout {
    let d = config::design();
    let margin = d
        .home
        .margin_x
        .min(w / 8.0)
        .max((w - d.home.max_width) / 2.0);
    let top = top_inset() + d.home.top_gap;
    let bottom = (h - bottom_inset() - d.home.bottom_gap).max(top + 1.0);
    let area = RectF {
        x: margin,
        y: top,
        w: (w - 2.0 * margin).max(1.0),
        h: (bottom - top).max(1.0),
    };
    let footer = d.home.footer_height.min(area.h / 3.0);
    let glance_height: f32 = if tablet_mode() && h >= 540.0 {
        76.0
    } else {
        0.0
    };
    let glance = RectF {
        w: area.w.min(320.0),
        h: glance_height.min(56.0),
        ..area
    };
    let grid = RectF {
        y: area.y + glance_height,
        h: (area.h - footer - glance_height).max(1.0),
        ..area
    };
    let columns = ((grid.w / d.home.cell_width.max(d.launcher.tile_size)).floor() as usize)
        .clamp(1, d.home.max_columns);
    let rows = ((grid.h / d.home.cell_height.max(d.launcher.tile_size)).floor() as usize)
        .clamp(1, d.home.max_rows);
    let nav_y = grid.bottom() + (footer - d.home.search_height - d.home.nav_height).max(0.0) / 2.0;
    HomeLayout {
        area,
        glance,
        grid,
        columns,
        rows,
        per_page: columns * rows,
        previous: RectF {
            x: w / 2.0 - d.home.nav_offset,
            y: nav_y,
            w: d.home.nav_width,
            h: d.home.nav_height,
        },
        next: RectF {
            x: w / 2.0 + d.home.nav_offset - d.home.nav_width,
            y: nav_y,
            w: d.home.nav_width,
            h: d.home.nav_height,
        },
        search: RectF {
            x: (w - area.w.min(d.home.search_width)) / 2.0,
            y: area.bottom() - d.home.search_height.min(footer),
            w: area.w.min(d.home.search_width),
            h: d.home.search_height.min(footer),
        },
    }
}
impl HomeLayout {
    pub(super) fn tile(&self, cell: Cell, position: f32, offset: f32) -> RectF {
        let d = config::design();
        let cw = self.grid.w / self.columns as f32;
        let ch = self.grid.h / self.rows as f32;
        RectF {
            x: self.grid.x
                + cell.column as f32 * cw
                + (cw - d.launcher.tile_size) / 2.0
                + (cell.page as f32 - position) * (self.area.w + d.home.page_gap),
            y: self.grid.y + cell.row as f32 * ch + (ch - d.launcher.tile_size) / 2.0 + offset,
            w: d.launcher.tile_size,
            h: d.launcher.tile_size,
        }
    }
}
impl Shell {
    pub fn home_pages(&self, w: f32, h: f32) -> usize {
        self.home
            .cells(home_layout(w, h))
            .values()
            .map(|c| c.page + 1)
            .max()
            .unwrap_or(1)
    }
    pub fn home_tile_rect(&self, w: f32, h: f32, index: usize) -> RectF {
        let layout = home_layout(w, h);
        let cells = self.home.cells(layout);
        self.home_tile_in(layout, &cells, index)
    }
    pub(super) fn home_tile_in(
        &self,
        layout: HomeLayout,
        cells: &HashMap<String, Cell>,
        index: usize,
    ) -> RectF {
        let id = self
            .home
            .apps
            .get(index)
            .map(AppEntry::identity)
            .unwrap_or_default();
        let cell = cells
            .get(&id)
            .copied()
            .unwrap_or_else(|| layout.cell_at_slot(index));
        let mut rect = layout.tile(cell, self.home.position.value(), self.home_offset());
        if let Some((x, y)) = self.home.icon_motion.get(&id) {
            rect.x += x.value();
            rect.y += y.value();
        }
        rect
    }
    pub fn home_hit(&self, w: f32, h: f32, point: Point<f64, Logical>) -> Option<HomeHit> {
        let layout = home_layout(w, h);
        if layout.glance.h > 0.0 && layout.glance.contains(point) {
            return Some(HomeHit::Calendar);
        }
        if layout.search.contains(point) {
            return Some(HomeHit::Search);
        }
        if layout.grid.contains(point) {
            let cells = self.home.cells(layout);
            return (0..self.home.apps.len())
                .find(|i| self.home_tile_in(layout, &cells, *i).contains(point))
                .map(HomeHit::App);
        }
        if self.home_pages(w, h) > 1 || self.dragged_identity.is_some() {
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
