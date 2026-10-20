use traxide_contracts::geometry::{Point, Rect};

/// The routing grid within a window of the board: nodes every `pitch`, aligned
/// to multiples of the pitch so that every connection shares one lattice.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Grid {
    pitch: i32,
    min: (i32, i32),
    max: (i32, i32),
}

/// A node of the grid, by its multiples of the pitch.
pub(crate) type Node = (i32, i32);

/// The eight octilinear directions, counter-clockwise from east.
pub(crate) const DIRECTIONS: [(i32, i32); 8] = [
    (1, 0),
    (1, 1),
    (0, 1),
    (-1, 1),
    (-1, 0),
    (-1, -1),
    (0, -1),
    (1, -1),
];

impl Grid {
    pub(crate) fn covering(window: Rect, pitch: i32) -> Self {
        let (min, max) = (window.min, window.max);
        Self {
            pitch,
            min: (min.x.div_euclid(pitch) + 1, min.y.div_euclid(pitch) + 1),
            max: (max.x.div_euclid(pitch), max.y.div_euclid(pitch)),
        }
    }

    pub(crate) fn point(&self, (x, y): Node) -> Point {
        Point::new(x * self.pitch, y * self.pitch)
    }

    pub(crate) fn node_count(&self) -> usize {
        self.columns() * usize::try_from(self.max.1 - self.min.1 + 1).unwrap_or(0)
    }

    fn columns(&self) -> usize {
        usize::try_from(self.max.0 - self.min.0 + 1).unwrap_or(0)
    }

    /// A dense index of a node inside the window, for arrays over all nodes.
    pub(crate) fn index(&self, (x, y): Node) -> usize {
        debug_assert!(self.contains((x, y)));
        (y - self.min.1) as usize * self.columns() + (x - self.min.0) as usize
    }

    pub(crate) fn node(&self, index: usize) -> Node {
        let columns = self.columns();
        (
            self.min.0 + (index % columns) as i32,
            self.min.1 + (index / columns) as i32,
        )
    }

    pub(crate) fn contains(&self, (x, y): Node) -> bool {
        (self.min.0..=self.max.0).contains(&x) && (self.min.1..=self.max.1).contains(&y)
    }

    /// The nodes inside `area`, and the ring around the node nearest to `centre`,
    /// so that even a pad smaller than the pitch has nodes to leave by.
    pub(crate) fn nodes_near(&self, area: Rect, centre: Point) -> Vec<Node> {
        let floor = |v: i32| v.div_euclid(self.pitch);
        let ceil = |v: i32| -(-v).div_euclid(self.pitch);
        let mut nodes = Vec::new();
        for y in ceil(area.min.y)..=floor(area.max.y) {
            for x in ceil(area.min.x)..=floor(area.max.x) {
                nodes.push((x, y));
            }
        }
        let nearest = (
            floor(centre.x + self.pitch / 2),
            floor(centre.y + self.pitch / 2),
        );
        for dy in -1..=1 {
            for dx in -1..=1 {
                nodes.push((nearest.0 + dx, nearest.1 + dy));
            }
        }
        nodes.sort_unstable();
        nodes.dedup();
        nodes.retain(|&n| self.contains(n));
        nodes
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nodes_are_multiples_of_the_pitch_inside_the_window() {
        let grid = Grid::covering(
            Rect {
                min: Point::new(-50, 0),
                max: Point::new(250, 100),
            },
            100,
        );
        assert!(grid.contains((0, 1)) && grid.contains((2, 1)));
        assert!(!grid.contains((-1, 1)) && !grid.contains((3, 1)));
        assert!(!grid.contains((0, 0)), "the window's edge is not inside it");
    }

    #[test]
    fn dense_indices_round_trip() {
        let grid = Grid::covering(
            Rect {
                min: Point::new(-550, 0),
                max: Point::new(250, 100),
            },
            100,
        );
        for node in [(-5, 1), (2, 1), (0, 1)] {
            assert_eq!(grid.node(grid.index(node)), node);
        }
        assert_eq!(grid.node_count(), 8);
    }

    #[test]
    fn a_tiny_pad_still_has_nodes_around_it() {
        let grid = Grid::covering(
            Rect {
                min: Point::new(-1_000, -1_000),
                max: Point::new(1_000, 1_000),
            },
            100,
        );
        let centre = Point::new(130, 140);
        let nodes = grid.nodes_near(
            Rect {
                min: centre,
                max: centre,
            },
            centre,
        );
        assert_eq!(nodes.len(), 9);
        assert!(nodes.contains(&(1, 1)));
    }
}
