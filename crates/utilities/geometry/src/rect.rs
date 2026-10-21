use crate::Point;

/// An axis-aligned rectangle; `min` and `max` are inclusive.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Rect {
    pub min: Point,
    pub max: Point,
}

impl Rect {
    pub fn around(points: impl IntoIterator<Item = Point>) -> Option<Self> {
        let mut points = points.into_iter();
        let first = points.next()?;
        Some(points.fold(
            Self {
                min: first,
                max: first,
            },
            |rect, p| rect.including(p),
        ))
    }

    pub fn including(self, p: Point) -> Self {
        Self {
            min: Point::new(self.min.x.min(p.x), self.min.y.min(p.y)),
            max: Point::new(self.max.x.max(p.x), self.max.y.max(p.y)),
        }
    }

    pub fn union(self, other: Self) -> Self {
        self.including(other.min).including(other.max)
    }

    pub fn inflated(self, by: i32) -> Self {
        Self {
            min: Point::new(self.min.x - by, self.min.y - by),
            max: Point::new(self.max.x + by, self.max.y + by),
        }
    }

    /// Whether the rectangles lie at least `distance` apart along an axis, so
    /// that no point of one is closer than `distance` to any of the other.
    pub fn is_apart_from(self, other: Self, distance: i64) -> bool {
        let gap = |low: i32, high: i32| i64::from(low) - i64::from(high) >= distance;
        gap(other.min.x, self.max.x)
            || gap(self.min.x, other.max.x)
            || gap(other.min.y, self.max.y)
            || gap(self.min.y, other.max.y)
    }

    pub fn intersects(self, other: Self) -> bool {
        self.min.x <= other.max.x
            && other.min.x <= self.max.x
            && self.min.y <= other.max.y
            && other.min.y <= self.max.y
    }

    /// In square nanometres, which overflow `i32` above 46 µm square.
    pub fn area(self) -> i64 {
        i64::from(self.max.x - self.min.x) * i64::from(self.max.y - self.min.y)
    }

    pub fn contains_rect(self, other: Self) -> bool {
        self.contains(other.min) && self.contains(other.max)
    }

    pub fn contains(self, p: Point) -> bool {
        (self.min.x..=self.max.x).contains(&p.x) && (self.min.y..=self.max.y).contains(&p.y)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rectangles_a_gap_apart_are_apart_by_that_gap_and_no_more() {
        let a = Rect {
            min: Point::new(0, 0),
            max: Point::new(10, 10),
        };
        let b = Rect {
            min: Point::new(15, 3),
            max: Point::new(20, 30),
        };
        assert!(a.is_apart_from(b, 5) && b.is_apart_from(a, 5));
        assert!(!a.is_apart_from(b, 6));
        let below = Rect {
            min: Point::new(-20, 12),
            max: Point::new(-1, 13),
        };
        assert!(a.is_apart_from(below, 2));
        assert!(
            !a.is_apart_from(below, 3),
            "only the larger gap along one axis counts"
        );
    }

    fn rect(x1: i32, y1: i32, x2: i32, y2: i32) -> Rect {
        Rect {
            min: Point::new(x1, y1),
            max: Point::new(x2, y2),
        }
    }

    #[test]
    fn the_area_of_a_board_sized_rectangle_fits() {
        assert_eq!(
            rect(0, 0, 400_000_000, 300_000_000).area(),
            120_000_000_000_000_000
        );
    }

    #[test]
    fn rectangles_sharing_an_edge_intersect() {
        assert!(rect(0, 0, 10, 10).intersects(rect(10, 0, 20, 10)));
    }

    #[test]
    fn separate_rectangles_do_not_intersect() {
        assert!(!rect(0, 0, 10, 10).intersects(rect(11, 0, 20, 10)));
    }

    #[test]
    fn around_spans_all_points() {
        let bounds = Rect::around([Point::new(5, -2), Point::new(-1, 7), Point::new(3, 3)]);
        assert_eq!(bounds, Some(rect(-1, -2, 5, 7)));
    }

    #[test]
    fn around_nothing_is_nothing() {
        assert_eq!(Rect::around([]), None);
    }
}
