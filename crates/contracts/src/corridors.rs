use crate::ConnectionId;
use traxide_geometry::{Point, Rect};

/// Where global routing sends each connection: a band of coarse cells along its
/// way, which detailed routing keeps inside.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Corridors {
    origin: Point,
    cell_size: i32,
    columns: i32,
    /// For each connection of the plan, the indices of its cells, sorted.
    cells: Vec<Vec<u32>>,
}

impl Corridors {
    /// Cells are squares of `cell_size` in rows of `columns`, counted from `origin`.
    pub fn new(origin: Point, cell_size: i32, columns: i32, cells: Vec<Vec<u32>>) -> Self {
        Self {
            origin,
            cell_size,
            columns,
            cells,
        }
    }

    /// Whether `connection` may run through `at`; anywhere when it has no corridor.
    pub fn allows(&self, connection: ConnectionId, at: Point) -> bool {
        match self.cells.get(connection.index()) {
            Some(cells) if !cells.is_empty() => self
                .cell_at(at)
                .is_some_and(|cell| cells.binary_search(&cell).is_ok()),
            _ => true,
        }
    }

    /// The box around the corridor of `connection`, if it has one.
    pub fn bounds(&self, connection: ConnectionId) -> Option<Rect> {
        let corners = self
            .cells
            .get(connection.index())?
            .iter()
            .flat_map(|&cell| {
                let (column, row) = (cell as i32 % self.columns, cell as i32 / self.columns);
                let min = Point::new(
                    self.origin.x + column * self.cell_size,
                    self.origin.y + row * self.cell_size,
                );
                [
                    min,
                    Point::new(min.x + self.cell_size, min.y + self.cell_size),
                ]
            });
        Rect::around(corners)
    }

    fn cell_at(&self, at: Point) -> Option<u32> {
        let column = (at.x - self.origin.x).div_euclid(self.cell_size);
        let row = (at.y - self.origin.y).div_euclid(self.cell_size);
        (column >= 0 && column < self.columns && row >= 0)
            .then(|| (row * self.columns + column) as u32)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Cells of 10 nm, 4 per row; connection 0 may use cells 1 and 6, connection 1 anything.
    fn corridors() -> Corridors {
        Corridors::new(Point::new(0, 0), 10, 4, vec![vec![1, 6], vec![]])
    }

    #[test]
    fn a_connection_may_run_through_its_cells_only() {
        let corridors = corridors();
        assert!(corridors.allows(ConnectionId(0), Point::new(15, 5)));
        assert!(corridors.allows(ConnectionId(0), Point::new(25, 15)));
        assert!(!corridors.allows(ConnectionId(0), Point::new(5, 5)));
        assert!(!corridors.allows(ConnectionId(0), Point::new(-5, 5)));
    }

    #[test]
    fn a_connection_without_a_corridor_may_run_anywhere() {
        assert!(corridors().allows(ConnectionId(1), Point::new(1_000, 1_000)));
        assert!(corridors().allows(ConnectionId(7), Point::new(5, 5)));
    }

    #[test]
    fn bounds_span_all_cells_of_a_corridor() {
        let expected = Rect {
            min: Point::new(10, 0),
            max: Point::new(30, 20),
        };
        assert_eq!(corridors().bounds(ConnectionId(0)), Some(expected));
        assert_eq!(corridors().bounds(ConnectionId(1)), None);
    }
}
