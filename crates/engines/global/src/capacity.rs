use traxide_contracts::geometry::{Point, Rect};
use traxide_contracts::{ConstraintSet, Design, LayerId};

/// Tracks that fit side by side across one cell: cells are as wide as this many
/// track pitches (track plus clearance), so their capacity counts in lanes.
pub(crate) const LANES: u8 = 4;

/// The coarse grid over the board and how many tracks each cell still has room
/// for, along each axis and on each layer.
#[derive(Debug)]
pub(crate) struct Cells {
    origin: Point,
    size: i32,
    pitch: i32,
    columns: i32,
    rows: i32,
    layers: usize,
    /// Lanes free for horizontal and vertical travel, by `index(cell, layer)`.
    horizontal: Vec<u8>,
    vertical: Vec<u8>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Axis {
    Horizontal,
    Vertical,
}

impl Cells {
    pub(crate) fn of(design: &Design, rules: &ConstraintSet) -> Self {
        let class = &design.rules.classes[design.rules.default_class.index()];
        let pitch = class.track_width + class.clearance.max(rules.clearance(None, None));
        let size = pitch * i32::from(LANES);
        let board = design.outline.bounds();
        let columns = (board.max.x - board.min.x) / size + 1;
        let rows = (board.max.y - board.min.y) / size + 1;
        let count = (columns * rows) as usize * design.layers.len();
        let mut cells = Self {
            origin: board.min,
            size,
            pitch,
            columns,
            rows,
            layers: design.layers.len(),
            horizontal: vec![LANES; count],
            vertical: vec![LANES; count],
        };
        let half_track = class.track_width / 2;
        let routable_area = board.inflated(-(rules.edge_clearance() + half_track));
        cells.block_outside(routable_area, design);
        for pad in &design.pads {
            for copper in &pad.copper {
                let reach = rules.clearance(pad.net, pad.net) + half_track;
                cells.block(copper.layer, copper.shape.bounds().inflated(reach));
            }
        }
        for keepout in &design.keepouts {
            let layers: Vec<LayerId> = match keepout.layer {
                Some(layer) => vec![layer],
                None => design.layer_ids().collect(),
            };
            for layer in layers {
                cells.block(layer, keepout.shape.bounds().inflated(half_track));
            }
        }
        for layer in design
            .layer_ids()
            .filter(|l| !design.layers[l.index()].routable)
        {
            cells.block(layer, board.inflated(size));
        }
        cells
    }

    pub(crate) fn origin(&self) -> Point {
        self.origin
    }

    pub(crate) fn size(&self) -> i32 {
        self.size
    }

    pub(crate) fn columns(&self) -> i32 {
        self.columns
    }

    pub(crate) fn rows(&self) -> i32 {
        self.rows
    }

    pub(crate) fn layers(&self) -> usize {
        self.layers
    }

    pub(crate) fn count(&self) -> usize {
        (self.columns * self.rows) as usize
    }

    pub(crate) fn cell_at(&self, at: Point) -> Option<(i32, i32)> {
        let column = (at.x - self.origin.x).div_euclid(self.size);
        let row = (at.y - self.origin.y).div_euclid(self.size);
        self.contains((column, row)).then_some((column, row))
    }

    pub(crate) fn contains(&self, (column, row): (i32, i32)) -> bool {
        (0..self.columns).contains(&column) && (0..self.rows).contains(&row)
    }

    pub(crate) fn index(&self, (column, row): (i32, i32), layer: usize) -> usize {
        (layer * self.rows as usize + row as usize) * self.columns as usize + column as usize
    }

    pub(crate) fn capacity(&self, cell: (i32, i32), layer: usize, axis: Axis) -> u8 {
        let index = self.index(cell, layer);
        match axis {
            Axis::Horizontal => self.horizontal[index],
            Axis::Vertical => self.vertical[index],
        }
    }

    /// `cells` and their neighbours, as sorted cell numbers for the corridor.
    pub(crate) fn widened(&self, cells: &[(i32, i32)]) -> Vec<u32> {
        let mut widened: Vec<u32> = cells
            .iter()
            .flat_map(|&(c, r)| {
                (-1..=1).flat_map(move |dr| (-1..=1).map(move |dc| (c + dc, r + dr)))
            })
            .filter(|&cell| self.contains(cell))
            .map(|(c, r)| (r * self.columns + c) as u32)
            .collect();
        widened.sort_unstable();
        widened.dedup();
        widened
    }

    /// The cells `area` touches.
    pub(crate) fn covered(&self, area: Rect) -> impl Iterator<Item = (i32, i32)> + use<> {
        let (first, last) = (self.clamped_cell(area.min), self.clamped_cell(area.max));
        (first.1..=last.1).flat_map(move |row| (first.0..=last.0).map(move |column| (column, row)))
    }

    fn lane_centre(&self, cell_start: i32, lane: u8) -> i32 {
        cell_start + self.pitch * i32::from(lane) + self.pitch / 2
    }

    /// Closes every lane that runs through `area` on `layer`.
    fn block(&mut self, layer: LayerId, area: Rect) {
        let (first, last) = (self.clamped_cell(area.min), self.clamped_cell(area.max));
        for row in first.1..=last.1 {
            for column in first.0..=last.0 {
                let index = self.index((column, row), layer.index());
                let (x0, y0) = (
                    self.origin.x + column * self.size,
                    self.origin.y + row * self.size,
                );
                for lane in 0..LANES {
                    if (area.min.y..=area.max.y).contains(&self.lane_centre(y0, lane)) {
                        self.horizontal[index] = self.horizontal[index].saturating_sub(1);
                    }
                    if (area.min.x..=area.max.x).contains(&self.lane_centre(x0, lane)) {
                        self.vertical[index] = self.vertical[index].saturating_sub(1);
                    }
                }
            }
        }
    }

    /// Closes the lanes outside `area`: beyond the board, or too close to its edge.
    fn block_outside(&mut self, area: Rect, design: &Design) {
        for layer in 0..self.layers {
            for row in 0..self.rows {
                for column in 0..self.columns {
                    let index = self.index((column, row), layer);
                    let (x0, y0) = (
                        self.origin.x + column * self.size,
                        self.origin.y + row * self.size,
                    );
                    let centre = Point::new(x0 + self.size / 2, y0 + self.size / 2);
                    for lane in 0..LANES {
                        let horizontal_lane = Point::new(centre.x, self.lane_centre(y0, lane));
                        if !area.contains(horizontal_lane)
                            || !design.outline.contains(horizontal_lane)
                        {
                            self.horizontal[index] = self.horizontal[index].saturating_sub(1);
                        }
                        let vertical_lane = Point::new(self.lane_centre(x0, lane), centre.y);
                        if !area.contains(vertical_lane) || !design.outline.contains(vertical_lane)
                        {
                            self.vertical[index] = self.vertical[index].saturating_sub(1);
                        }
                    }
                }
            }
        }
    }

    fn clamped_cell(&self, at: Point) -> (i32, i32) {
        let column = ((at.x - self.origin.x).div_euclid(self.size)).clamp(0, self.columns - 1);
        let row = ((at.y - self.origin.y).div_euclid(self.size)).clamp(0, self.rows - 1);
        (column, row)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use traxide_contracts::testing::{DesignBuilder, at};

    fn cells(design: &Design) -> Cells {
        Cells::of(
            design,
            &traxide_contracts::ConstraintSet::new(
                design.nets.iter().map(|n| n.class).collect(),
                design.rules.default_class,
                vec![200_000],
                vec![vec![]],
                vec![200_000],
                200_000,
            ),
        )
    }

    #[test]
    fn open_board_has_every_lane_free() {
        let design = DesignBuilder::new(20.0, 20.0).build();
        let cells = cells(&design);
        let middle = cells.cell_at(at(10.0, 10.0)).expect("on the board");
        assert_eq!(cells.capacity(middle, 0, Axis::Horizontal), LANES);
        assert_eq!(cells.capacity(middle, 0, Axis::Vertical), LANES);
    }

    #[test]
    fn a_pad_closes_the_lanes_it_covers_on_its_layer_only() {
        let design = DesignBuilder::new(20.0, 20.0)
            .smd("P", Some("A"), at(10.0, 10.0), 1.0, 0)
            .build();
        let cells = cells(&design);
        let middle = cells.cell_at(at(10.0, 10.0)).expect("on the board");
        assert!(cells.capacity(middle, 0, Axis::Horizontal) < LANES);
        assert_eq!(cells.capacity(middle, 1, Axis::Horizontal), LANES);
    }

    #[test]
    fn cells_at_the_board_edge_lose_their_outer_lanes() {
        let design = DesignBuilder::new(20.0, 20.0).build();
        let cells = cells(&design);
        assert!(cells.capacity((0, 5), 0, Axis::Vertical) < LANES);
    }
}
