use crate::{Point, Polygon, Rect, Segment, Shape};

/// One or more polygons that do not overlap, such as the outline of a board,
/// or the outlines of the boards of a panel.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Region {
    polygons: Vec<Polygon>,
    /// Cutouts inside the polygons: outside the region, and edges like theirs.
    holes: Vec<Polygon>,
}

impl Region {
    /// `None` without a polygon.
    pub fn new(polygons: Vec<Polygon>) -> Option<Self> {
        (!polygons.is_empty()).then_some(Self {
            polygons,
            holes: Vec::new(),
        })
    }

    #[must_use]
    pub fn with_holes(mut self, holes: Vec<Polygon>) -> Self {
        self.holes = holes;
        self
    }

    pub fn polygons(&self) -> &[Polygon] {
        &self.polygons
    }

    pub fn holes(&self) -> &[Polygon] {
        &self.holes
    }

    pub fn bounds(&self) -> Rect {
        self.polygons
            .iter()
            .map(Polygon::bounds)
            .reduce(Rect::union)
            .unwrap_or_else(|| unreachable!("a region has a polygon"))
    }

    pub fn contains(&self, p: Point) -> bool {
        self.polygons.iter().any(|polygon| polygon.contains(p))
            && !self.holes.iter().any(|hole| hole.contains(p))
    }

    /// The edges of the polygons and of the holes.
    pub fn edges(&self) -> impl Iterator<Item = Segment> + '_ {
        self.polygons
            .iter()
            .chain(&self.holes)
            .flat_map(Polygon::edges)
    }

    /// Whether `shape` lies inside the region, at least `clearance` from its edges.
    pub fn holds(&self, shape: &Shape, clearance: i32) -> bool {
        let point = match shape {
            Shape::Capsule { segment, .. } => segment.a,
            Shape::Polygon(polygon) => polygon.points()[0],
        };
        self.polygons
            .iter()
            .any(|polygon| shape.is_inside(polygon, clearance))
            && self.holes.iter().all(|hole| {
                !hole.contains(point)
                    && hole.edges().all(|edge| {
                        !Shape::Capsule {
                            segment: edge,
                            radius: 0,
                        }
                        .is_closer_than(shape, clearance)
                    })
            })
    }
}

impl From<Polygon> for Region {
    fn from(polygon: Polygon) -> Self {
        Self {
            polygons: vec![polygon],
            holes: Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn square(x: i32) -> Polygon {
        Polygon::rectangle(Rect {
            min: Point::new(x, 0),
            max: Point::new(x + 10, 10),
        })
    }

    #[test]
    fn contains_what_any_of_its_polygons_contains() {
        let region = Region::new(vec![square(0), square(20)]).expect("polygons");
        assert!(region.contains(Point::new(5, 5)));
        assert!(region.contains(Point::new(25, 5)));
        assert!(!region.contains(Point::new(15, 5)));
        assert_eq!(
            region.bounds(),
            Rect {
                min: Point::new(0, 0),
                max: Point::new(30, 10)
            }
        );
    }

    #[test]
    fn a_hole_is_outside_the_region_and_keeps_its_clearance() {
        let hole = Polygon::rectangle(Rect {
            min: Point::new(40, 40),
            max: Point::new(60, 60),
        });
        let board = Polygon::rectangle(Rect {
            min: Point::new(0, 0),
            max: Point::new(100, 100),
        });
        let region = Region::new(vec![board])
            .expect("a polygon")
            .with_holes(vec![hole]);
        assert!(!region.contains(Point::new(50, 50)));
        assert!(region.contains(Point::new(20, 50)));
        let inside_the_hole = Shape::track(Segment::new(Point::new(45, 50), Point::new(55, 50)), 2);
        assert!(!region.holds(&inside_the_hole, 0));
        let beside_the_hole = Shape::track(Segment::new(Point::new(35, 30), Point::new(35, 70)), 2);
        assert!(region.holds(&beside_the_hole, 4), "4 from the hole");
        assert!(!region.holds(&beside_the_hole, 5));
        assert_eq!(region.edges().count(), 8);
    }

    #[test]
    fn copper_between_two_boards_is_on_neither() {
        let region = Region::new(vec![square(0), square(20)]).expect("polygons");
        let bridge = Shape::track(Segment::new(Point::new(8, 5), Point::new(22, 5)), 1);
        assert!(!region.holds(&bridge, 0));
        let on_one = Shape::track(Segment::new(Point::new(2, 5), Point::new(8, 5)), 1);
        assert!(region.holds(&on_one, 1));
    }
}
