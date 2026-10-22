//! One occupancy map drives drawing, drops and persistence. Empty cells stay empty.
use super::layout::HomeLayout;
use super::*;
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(super) struct Cell {
    pub page: usize,
    pub column: usize,
    pub row: usize,
}
impl HomeLayout {
    pub(super) fn anchor(&self, c: Cell) -> config::Anchor {
        config::Anchor {
            page: c.page,
            x: (c.column as f32 + 0.5) / self.columns as f32,
            y: (c.row as f32 + 0.5) / self.rows as f32,
        }
    }
    pub(super) fn cell(&self, anchor: config::Anchor) -> Cell {
        Cell {
            page: anchor.page,
            column: ((anchor.x * self.columns as f32) as usize).min(self.columns - 1),
            row: ((anchor.y * self.rows as f32) as usize).min(self.rows - 1),
        }
    }
    pub(super) fn cell_at_slot(&self, slot: usize) -> Cell {
        Cell {
            page: slot / self.per_page,
            column: slot % self.columns,
            row: (slot % self.per_page) / self.columns,
        }
    }
    pub(super) fn slot(&self, c: Cell) -> usize {
        c.page * self.per_page + c.row * self.columns + c.column
    }
}
impl Home {
    pub(super) fn cells(&self, layout: HomeLayout) -> HashMap<String, Cell> {
        let mut occupied = std::collections::HashSet::new();
        let mut result = HashMap::new();
        // Explicit positions have priority over old ordered shortcuts.
        for entry in self.workspace.entries.iter().filter(|s| s.anchor.is_some()) {
            let wanted = layout.cell(entry.anchor.unwrap());
            let cell = if !occupied.contains(&wanted) {
                wanted
            } else {
                (0..)
                    .map(|i| layout.cell_at_slot(wanted.page * layout.per_page + i))
                    .find(|c| !occupied.contains(c))
                    .unwrap()
            };
            occupied.insert(cell);
            result.insert(entry.id.clone(), cell);
        }
        for entry in self.workspace.entries.iter().filter(|s| s.anchor.is_none()) {
            let cell = (0..)
                .map(|i| layout.cell_at_slot(i))
                .find(|c| !occupied.contains(c))
                .unwrap();
            occupied.insert(cell);
            result.insert(entry.id.clone(), cell);
        }
        result
    }
    pub(super) fn anchored(&self, layout: HomeLayout) -> Vec<config::Shortcut> {
        let cells = self.cells(layout);
        self.workspace
            .entries
            .iter()
            .map(|entry| config::Shortcut {
                id: entry.id.clone(),
                anchor: Some(layout.anchor(cells[&entry.id])),
            })
            .collect()
    }
}
