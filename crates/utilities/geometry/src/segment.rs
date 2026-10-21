use crate::{Point, Rect};

/// A straight line from `a` to `b`; a segment of length zero is a point.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Segment {
    pub a: Point,
    pub b: Point,
}

impl Segment {
    pub const fn new(a: Point, b: Point) -> Self {
        Self { a, b }
    }

    pub fn bounds(self) -> Rect {
        Rect {
            min: self.a,
            max: self.a,
        }
        .including(self.b)
    }

    /// The point of the segment nearest to `p`.
    pub fn closest_point(self, p: Point) -> Point {
        let (dx, dy) = (
            i128::from(self.b.x - self.a.x),
            i128::from(self.b.y - self.a.y),
        );
        let squared_length = dx * dx + dy * dy;
        if squared_length == 0 {
            return self.a;
        }
        let along = (i128::from(p.x - self.a.x) * dx + i128::from(p.y - self.a.y) * dy)
            .clamp(0, squared_length);
        let at = |start: i32, step: i128| {
            let offset = (2 * step * along + squared_length).div_euclid(2 * squared_length);
            i32::try_from(i128::from(start) + offset).unwrap_or(start)
        };
        Point::new(at(self.a.x, dx), at(self.a.y, dy))
    }

    pub fn length(self) -> i64 {
        self.a.distance(self.b)
    }

    pub fn intersects(self, other: Self) -> bool {
        let d1 = orientation(other.a, other.b, self.a);
        let d2 = orientation(other.a, other.b, self.b);
        let d3 = orientation(self.a, self.b, other.a);
        let d4 = orientation(self.a, self.b, other.b);
        if d1 * d2 < 0 && d3 * d4 < 0 {
            return true;
        }
        (d1 == 0 && other.bounds().contains(self.a))
            || (d2 == 0 && other.bounds().contains(self.b))
            || (d3 == 0 && self.bounds().contains(other.a))
            || (d4 == 0 && self.bounds().contains(other.b))
    }

    /// Whether `p` is closer than `distance` to this segment.
    pub fn is_point_closer_than(self, p: Point, distance: i64) -> bool {
        let limit = i128::from(distance) * i128::from(distance);
        let along = self.b - self.a;
        let length_squared = along.dot(along);
        let projection = (p - self.a).dot(along);
        if length_squared == 0 || projection <= 0 {
            return self.a.squared_distance(p) < limit;
        }
        if projection >= length_squared {
            return self.b.squared_distance(p) < limit;
        }
        // Compared as cross² < d²·|ab|² to stay exact; dividing would round.
        let cross = along.cross(p - self.a);
        cross * cross < limit * length_squared
    }

    /// Whether any point of this segment is closer than `distance` to `other`.
    pub fn is_closer_than(self, other: Self, distance: i64) -> bool {
        if self.bounds().is_apart_from(other.bounds(), distance) {
            return false;
        }
        // Two segments that don't cross are closest at an endpoint of one of them.
        self.intersects(other)
            || other.is_point_closer_than(self.a, distance)
            || other.is_point_closer_than(self.b, distance)
            || self.is_point_closer_than(other.a, distance)
            || self.is_point_closer_than(other.b, distance)
    }
}

fn orientation(a: Point, b: Point, c: Point) -> i128 {
    (b - a).cross(c - a).signum()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_closest_point_lies_on_the_segment() {
        let s = Segment::new(Point::new(0, 0), Point::new(10, 0));
        assert_eq!(s.closest_point(Point::new(4, 7)), Point::new(4, 0));
        assert_eq!(s.closest_point(Point::new(-5, 3)), Point::new(0, 0));
        assert_eq!(s.closest_point(Point::new(15, -3)), Point::new(10, 0));
    }
    use proptest::prelude::*;

    fn seg(x1: i32, y1: i32, x2: i32, y2: i32) -> Segment {
        Segment::new(Point::new(x1, y1), Point::new(x2, y2))
    }

    #[test]
    fn crossing_segments_intersect() {
        assert!(seg(0, 0, 10, 10).intersects(seg(0, 10, 10, 0)));
    }

    #[test]
    fn touching_segments_intersect() {
        assert!(seg(0, 0, 10, 0).intersects(seg(10, 0, 10, 10)));
    }

    #[test]
    fn collinear_disjoint_segments_do_not_intersect() {
        assert!(!seg(0, 0, 10, 0).intersects(seg(11, 0, 20, 0)));
    }

    #[test]
    fn parallel_segments_do_not_intersect() {
        assert!(!seg(0, 0, 10, 0).intersects(seg(0, 1, 10, 1)));
    }

    #[test]
    fn a_point_exactly_at_the_distance_is_not_closer() {
        assert!(!seg(0, 0, 100, 0).is_point_closer_than(Point::new(50, 30), 30));
        assert!(seg(0, 0, 100, 0).is_point_closer_than(Point::new(50, 29), 30));
    }

    #[test]
    fn beyond_the_ends_distance_is_measured_to_the_endpoint() {
        assert!(!seg(0, 0, 100, 0).is_point_closer_than(Point::new(130, 40), 50));
        assert!(seg(0, 0, 100, 0).is_point_closer_than(Point::new(129, 40), 50));
    }

    #[test]
    fn a_zero_length_segment_behaves_as_a_point() {
        assert!(seg(5, 5, 5, 5).is_point_closer_than(Point::new(5, 8), 4));
        assert!(!seg(5, 5, 5, 5).is_point_closer_than(Point::new(5, 9), 4));
    }

    #[test]
    fn parallel_segments_are_as_close_as_their_gap() {
        assert!(!seg(0, 0, 100, 0).is_closer_than(seg(0, 200, 100, 200), 200));
        assert!(seg(0, 0, 100, 0).is_closer_than(seg(0, 200, 100, 200), 201));
    }

    #[test]
    fn exact_at_the_largest_coordinates() {
        let m = crate::MAX_COORDINATE;
        let long = seg(-m, -m, m, m);
        assert!(long.is_point_closer_than(Point::new(m, m - 2), 2));
        assert!(!long.is_point_closer_than(Point::new(m, m - 2), 1));
    }

    proptest! {
        #[test]
        fn closeness_is_symmetric(
            a in (-1000..1000i32, -1000..1000i32, -1000..1000i32, -1000..1000i32),
            b in (-1000..1000i32, -1000..1000i32, -1000..1000i32, -1000..1000i32),
            d in 0..500i64,
        ) {
            let (s, t) = (seg(a.0, a.1, a.2, a.3), seg(b.0, b.1, b.2, b.3));
            prop_assert_eq!(s.is_closer_than(t, d), t.is_closer_than(s, d));
            prop_assert_eq!(s.intersects(t), t.intersects(s));
        }

        #[test]
        fn closeness_agrees_with_sampled_distance(
            a in (-1000..1000i32, -1000..1000i32, -1000..1000i32, -1000..1000i32),
            p in (-1000..1000i32, -1000..1000i32),
        ) {
            let s = seg(a.0, a.1, a.2, a.3);
            let p = Point::new(p.0, p.1);
            let exact = exact_distance(s, p);
            let d = exact.ceil() as i64;
            prop_assert!(!s.is_point_closer_than(p, exact.floor() as i64));
            prop_assert!(s.is_point_closer_than(p, d + 1));
        }
    }

    fn exact_distance(s: Segment, p: Point) -> f64 {
        let (ax, ay, bx, by) = (s.a.x as f64, s.a.y as f64, s.b.x as f64, s.b.y as f64);
        let (px, py) = (p.x as f64, p.y as f64);
        let (dx, dy) = (bx - ax, by - ay);
        let len2 = dx * dx + dy * dy;
        let t = if len2 == 0.0 {
            0.0
        } else {
            (((px - ax) * dx + (py - ay) * dy) / len2).clamp(0.0, 1.0)
        };
        ((ax + t * dx - px).powi(2) + (ay + t * dy - py).powi(2)).sqrt()
    }
}
