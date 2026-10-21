use traxide_contracts::geometry::{Point, Rect, Segment};
use traxide_contracts::{Design, LayerId, Via};

/// Edge length of a history cell: about the pitch of fine-pitch pins, so that a
/// conflict at one pin's way out makes its neighbours' ways out barely dearer.
const CELL: i32 = 500_000;

/// Where routes have collided so far: PathFinder's history cost. Each conflict
/// makes its place dearer for every later search, so connections learn to leave
/// a contested place to one of them instead of taking turns ripping each other
/// up there.
#[derive(Debug)]
pub(crate) struct History {
    origin: Point,
    columns: usize,
    rows: usize,
    layers: usize,
    /// What a conflict adds to its cells.
    increment: i64,
    /// The cost of crossing each cell, by layer, row and column.
    costs: Vec<i64>,
}

impl History {
    pub(crate) fn new(design: &Design, increment: i64) -> Self {
        let board = design.outline.bounds();
        let cells = |length: i32| usize::try_from(length / CELL + 1).unwrap_or(1);
        let (columns, rows) = (
            cells(board.max.x - board.min.x),
            cells(board.max.y - board.min.y),
        );
        Self {
            origin: board.min,
            columns,
            rows,
            layers: design.layers.len(),
            increment,
            costs: vec![0; design.layers.len() * rows * columns],
        }
    }

    /// What crossing `segment` on `layer` costs on top of its length: the
    /// history of the cell around its middle, for each cell length it runs.
    pub(crate) fn of_track(&self, layer: LayerId, segment: Segment) -> i64 {
        let middle = Point::new(
            segment.a.x / 2 + segment.b.x / 2,
            segment.a.y / 2 + segment.b.y / 2,
        );
        let cost = self
            .cell(layer.index(), middle)
            .map_or(0, |i| self.costs[i]);
        cost * segment.a.distance(segment.b) / i64::from(CELL)
    }

    /// What placing `via` costs on top of the via itself: the dearest history
    /// among the layers it spans.
    pub(crate) fn of_via(&self, via: &Via, layers: impl Iterator<Item = LayerId>) -> i64 {
        layers
            .filter_map(|layer| self.cell(layer.index(), via.position))
            .map(|i| self.costs[i])
            .max()
            .unwrap_or(0)
    }

    /// Makes the cells under a colliding track segment dearer.
    pub(crate) fn record_track(&mut self, layer: LayerId, segment: Segment, width: i32) {
        let area = Rect::around([segment.a, segment.b])
            .unwrap_or_else(|| unreachable!("two points"))
            .inflated(width / 2);
        self.record(layer.index(), area);
    }

    /// Makes the cells under a colliding via dearer, on every layer it spans.
    pub(crate) fn record_via(
        &mut self,
        position: Point,
        radius: i32,
        layers: impl Iterator<Item = LayerId>,
    ) {
        let area = Rect {
            min: position,
            max: position,
        }
        .inflated(radius);
        for layer in layers {
            self.record(layer.index(), area);
        }
    }

    fn record(&mut self, layer: usize, area: Rect) {
        let clamp = |at: i32, origin: i32, count: usize| {
            usize::try_from((at - origin).div_euclid(CELL))
                .unwrap_or(0)
                .min(count - 1)
        };
        let (first_column, last_column) = (
            clamp(area.min.x, self.origin.x, self.columns),
            clamp(area.max.x, self.origin.x, self.columns),
        );
        let (first_row, last_row) = (
            clamp(area.min.y, self.origin.y, self.rows),
            clamp(area.max.y, self.origin.y, self.rows),
        );
        for row in first_row..=last_row {
            for column in first_column..=last_column {
                let index = (layer * self.rows + row) * self.columns + column;
                self.costs[index] += self.increment;
            }
        }
    }

    fn cell(&self, layer: usize, at: Point) -> Option<usize> {
        let column = usize::try_from((at.x - self.origin.x).div_euclid(CELL)).ok()?;
        let row = usize::try_from((at.y - self.origin.y).div_euclid(CELL)).ok()?;
        (layer < self.layers && column < self.columns && row < self.rows)
            .then(|| (layer * self.rows + row) * self.columns + column)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use traxide_contracts::ViaKindId;
    use traxide_contracts::testing::{DesignBuilder, at};

    const LAYERS: [LayerId; 2] = [LayerId(0), LayerId(1)];

    fn history() -> History {
        History::new(&DesignBuilder::new(20.0, 20.0).build(), 2_000_000)
    }

    fn across_cell(x: f64) -> Segment {
        Segment::new(at(x, 10.1), at(x + 0.5, 10.1))
    }

    #[test]
    fn a_collision_makes_its_place_dearer_on_its_layer_only() {
        let mut history = history();
        history.record_track(LayerId(0), across_cell(5.0), 200_000);
        assert_eq!(history.of_track(LayerId(0), across_cell(5.0)), 2_000_000);
        assert_eq!(history.of_track(LayerId(1), across_cell(5.0)), 0);
        assert_eq!(history.of_track(LayerId(0), across_cell(12.0)), 0);
    }

    #[test]
    fn each_collision_adds_to_the_last() {
        let mut history = history();
        history.record_track(LayerId(0), across_cell(5.0), 200_000);
        history.record_track(LayerId(0), across_cell(5.0), 200_000);
        assert_eq!(history.of_track(LayerId(0), across_cell(5.0)), 4_000_000);
    }

    #[test]
    fn a_track_pays_for_the_length_it_runs_in_a_dear_place() {
        let mut history = history();
        history.record_track(LayerId(0), across_cell(5.0), 200_000);
        let tenth_of_a_cell = Segment::new(at(5.2, 10.1), at(5.25, 10.1));
        assert_eq!(history.of_track(LayerId(0), tenth_of_a_cell), 200_000);
    }

    #[test]
    fn a_colliding_via_makes_every_layer_it_spans_dearer() {
        let mut history = history();
        history.record_via(at(5.2, 10.2), 300_000, LAYERS.into_iter());
        let via = Via {
            net: traxide_contracts::NetId(0),
            kind: ViaKindId(0),
            position: at(5.2, 10.2),
        };
        assert_eq!(history.of_via(&via, LAYERS.into_iter()), 2_000_000);
        assert_eq!(history.of_track(LayerId(1), across_cell(5.0)), 2_000_000);
    }
}
