use crate::{Point, Polygon, Rect, Segment, Shape};

/// One or more polygons that do not overlap, such as the outline of a board,
/// or the outlines of the boards of a panel.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Region {
    polygons: Vec<Polygon>,
}

impl Region {
    /// `None` without a polygon.
    pub fn new(polygons: Vec<Polygon>) -> Option<Self> {
        (!polygons.is_empty()).then_some(Self { polygons })
    }

    pub fn polygons(&self) -> &[Polygon] {
        &self.polygons
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
    }

    pub fn edges(&self) -> impl Iterator<Item = Segment> + '_ {
        self.polygons.iter().flat_map(Polygon::edges)
    }

    /// Whether `shape` lies inside the region, at least `clearance` from its edges.
    pub fn holds(&self, shape: &Shape, clearance: i32) -> bool {
        self.polygons
            .iter()
            .any(|polygon| shape.is_inside(polygon, clearance))
    }
}

impl From<Polygon> for Region {
    fn from(polygon: Polygon) -> Self {
        Self {
            polygons: vec![polygon],
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
    fn copper_between_two_boards_is_on_neither() {
        let region = Region::new(vec![square(0), square(20)]).expect("polygons");
        let bridge = Shape::track(Segment::new(Point::new(8, 5), Point::new(22, 5)), 1);
        assert!(!region.holds(&bridge, 0));
        let on_one = Shape::track(Segment::new(Point::new(2, 5), Point::new(8, 5)), 1);
        assert!(region.holds(&on_one, 1));
    }
}
