use traxide_contracts::geometry::{Point, Rect};

/// A uniform grid over the board; each cell lists the items whose bounds touch it.
#[derive(Debug, Clone)]
pub(crate) struct Buckets {
    origin: Point,
    size: i32,
    columns: usize,
    rows: usize,
    cells: Vec<Vec<usize>>,
}

impl Buckets {
    pub(crate) fn new(area: Rect, size: i32) -> Self {
        let span = |from: i32, to: i32| usize::try_from((to - from) / size + 1).unwrap_or(1);
        let (columns, rows) = (span(area.min.x, area.max.x), span(area.min.y, area.max.y));
        Self {
            origin: area.min,
            size,
            columns,
            rows,
            cells: vec![Vec::new(); columns * rows],
        }
    }

    pub(crate) fn insert(&mut self, bounds: Rect, item: usize) {
        for cell in self.cells_in(bounds) {
            self.cells[cell].push(item);
        }
    }

    /// Items that may touch `area`, possibly more than once and possibly not at all.
    pub(crate) fn candidates(&self, area: Rect) -> impl Iterator<Item = usize> + '_ {
        self.cells_in(area)
            .flat_map(|cell| self.cells[cell].iter().copied())
    }

    fn cells_in(&self, area: Rect) -> impl Iterator<Item = usize> + use<> {
        // Items beyond the board still land in the outermost cells, so nothing is lost.
        let clamp = |value: i32, from: i32, count: usize| {
            usize::try_from((value - from).div_euclid(self.size))
                .unwrap_or(0)
                .min(count - 1)
        };
        let (x0, x1) = (
            clamp(area.min.x, self.origin.x, self.columns),
            clamp(area.max.x, self.origin.x, self.columns),
        );
        let (y0, y1) = (
            clamp(area.min.y, self.origin.y, self.rows),
            clamp(area.max.y, self.origin.y, self.rows),
        );
        let columns = self.columns;
        (y0..=y1).flat_map(move |y| (x0..=x1).map(move |x| y * columns + x))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rect(x1: i32, y1: i32, x2: i32, y2: i32) -> Rect {
        Rect {
            min: Point::new(x1, y1),
            max: Point::new(x2, y2),
        }
    }

    #[test]
    fn finds_items_in_the_cells_they_span() {
        let mut buckets = Buckets::new(rect(0, 0, 100, 100), 10);
        buckets.insert(rect(5, 5, 25, 5), 7);
        assert!(buckets.candidates(rect(21, 0, 22, 9)).any(|i| i == 7));
        assert!(!buckets.candidates(rect(50, 50, 60, 60)).any(|i| i == 7));
    }

    #[test]
    fn items_beyond_the_area_are_kept_at_its_edge() {
        let mut buckets = Buckets::new(rect(0, 0, 100, 100), 10);
        buckets.insert(rect(-50, -50, -40, -40), 3);
        assert!(buckets.candidates(rect(-45, -45, -44, -44)).any(|i| i == 3));
    }
}
