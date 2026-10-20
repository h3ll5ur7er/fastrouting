use crate::{Point, Rect, Segment};

/// A simple polygon, closed implicitly from the last point back to the first.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Polygon {
    points: Vec<Point>,
}

impl Polygon {
    /// `None` for fewer than three points: those enclose no area.
    pub fn new(points: Vec<Point>) -> Option<Self> {
        (points.len() >= 3).then_some(Self { points })
    }

    pub fn rectangle(rect: Rect) -> Self {
        Self {
            points: vec![
                rect.min,
                Point::new(rect.max.x, rect.min.y),
                rect.max,
                Point::new(rect.min.x, rect.max.y),
            ],
        }
    }

    pub fn points(&self) -> &[Point] {
        &self.points
    }

    pub fn edges(&self) -> impl Iterator<Item = Segment> + '_ {
        let next = self.points.iter().cycle().skip(1);
        self.points
            .iter()
            .zip(next)
            .map(|(&a, &b)| Segment::new(a, b))
    }

    pub fn bounds(&self) -> Rect {
        Rect::around(self.points.iter().copied()).expect("a polygon has at least three points")
    }

    /// Points on the boundary count as inside.
    pub fn contains(&self, p: Point) -> bool {
        let mut inside = false;
        for edge in self.edges() {
            if edge.is_point_closer_than(p, 1) {
                return true;
            }
            let (a, b) = (edge.a, edge.b);
            if (a.y > p.y) != (b.y > p.y) {
                // Is p left of the upward edge (or right of the downward one)? Exact in i128.
                let side = (b - a).cross(p - a);
                if (side > 0) == (b.y > a.y) {
                    inside = !inside;
                }
            }
        }
        inside
    }

    /// The rectangle this polygon is, if it is one with axis-parallel sides.
    pub fn as_rect(&self) -> Option<Rect> {
        let bounds = self.bounds();
        let corners = [
            bounds.min,
            Point::new(bounds.max.x, bounds.min.y),
            bounds.max,
            Point::new(bounds.min.x, bounds.max.y),
        ];
        (self.points.len() == 4 && corners.iter().all(|c| self.points.contains(c)))
            .then_some(bounds)
    }

    pub fn map(&self, f: impl Fn(Point) -> Point) -> Self {
        Self {
            points: self.points.iter().map(|&p| f(p)).collect(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn square() -> Polygon {
        Polygon::rectangle(Rect {
            min: Point::new(0, 0),
            max: Point::new(10, 10),
        })
    }

    #[test]
    fn contains_inner_points() {
        assert!(square().contains(Point::new(5, 5)));
    }

    #[test]
    fn contains_its_boundary() {
        assert!(square().contains(Point::new(10, 5)));
        assert!(square().contains(Point::new(0, 0)));
    }

    #[test]
    fn does_not_contain_outer_points() {
        assert!(!square().contains(Point::new(11, 5)));
        assert!(!square().contains(Point::new(5, -1)));
    }

    #[test]
    fn concave_notch_is_outside() {
        let u = Polygon::new(vec![
            Point::new(0, 0),
            Point::new(30, 0),
            Point::new(30, 30),
            Point::new(20, 30),
            Point::new(20, 10),
            Point::new(10, 10),
            Point::new(10, 30),
            Point::new(0, 30),
        ])
        .expect("eight points");
        assert!(!u.contains(Point::new(15, 20)));
        assert!(u.contains(Point::new(5, 20)));
    }

    #[test]
    fn an_axis_parallel_rectangle_is_recognised() {
        assert_eq!(
            square().as_rect(),
            Some(Rect {
                min: Point::new(0, 0),
                max: Point::new(10, 10)
            })
        );
        let tilted = Polygon::new(vec![
            Point::new(0, 5),
            Point::new(5, 0),
            Point::new(10, 5),
            Point::new(5, 10),
        ])
        .expect("four points");
        assert_eq!(tilted.as_rect(), None);
    }

    #[test]
    fn two_points_are_no_polygon() {
        assert_eq!(Polygon::new(vec![Point::new(0, 0), Point::new(1, 1)]), None);
    }
}
