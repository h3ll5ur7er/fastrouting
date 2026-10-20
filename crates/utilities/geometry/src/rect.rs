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

    pub fn intersects(self, other: Self) -> bool {
        self.min.x <= other.max.x
            && other.min.x <= self.max.x
            && self.min.y <= other.max.y
            && other.min.y <= self.max.y
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

    fn rect(x1: i32, y1: i32, x2: i32, y2: i32) -> Rect {
        Rect {
            min: Point::new(x1, y1),
            max: Point::new(x2, y2),
        }
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
