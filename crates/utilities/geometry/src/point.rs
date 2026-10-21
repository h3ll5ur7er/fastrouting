use std::ops::{Add, Sub};

/// Beyond about ±1.07 m, differences of coordinates would overflow `i32` and the
/// exact `i128` distance predicates, so designs must stay within this bound.
pub const MAX_COORDINATE: i32 = (1 << 30) - 1;

/// A point in nanometres.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Default)]
pub struct Point {
    pub x: i32,
    pub y: i32,
}

impl Point {
    pub const fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }

    pub fn is_within_bounds(self) -> bool {
        self.x.abs() <= MAX_COORDINATE && self.y.abs() <= MAX_COORDINATE
    }

    pub(crate) fn dot(self, other: Self) -> i128 {
        i128::from(self.x) * i128::from(other.x) + i128::from(self.y) * i128::from(other.y)
    }

    pub(crate) fn cross(self, other: Self) -> i128 {
        i128::from(self.x) * i128::from(other.y) - i128::from(self.y) * i128::from(other.x)
    }

    pub(crate) fn squared_distance(self, other: Self) -> i128 {
        let d = other - self;
        d.dot(d)
    }

    /// The Euclidean distance, rounded down.
    pub fn distance(self, other: Self) -> i64 {
        isqrt(self.squared_distance(other))
    }
}

impl Add for Point {
    type Output = Self;
    fn add(self, other: Self) -> Self {
        Self::new(self.x + other.x, self.y + other.y)
    }
}

impl Sub for Point {
    type Output = Self;
    fn sub(self, other: Self) -> Self {
        Self::new(self.x - other.x, self.y - other.y)
    }
}

/// Integer square root, rounded down; floating point would make results differ
/// between platforms in the last bit.
fn isqrt(value: i128) -> i64 {
    debug_assert!(value >= 0);
    let mut root = (value as f64).sqrt() as i128;
    while root * root > value {
        root -= 1;
    }
    while (root + 1) * (root + 1) <= value {
        root += 1;
    }
    root as i64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn distance_is_exact_for_pythagorean_triples() {
        assert_eq!(Point::new(0, 0).distance(Point::new(3_000, 4_000)), 5_000);
    }

    #[test]
    fn distance_rounds_down() {
        assert_eq!(Point::new(0, 0).distance(Point::new(1, 1)), 1);
    }

    #[test]
    fn distance_across_the_whole_coordinate_range_does_not_overflow() {
        let a = Point::new(-MAX_COORDINATE, -MAX_COORDINATE);
        let b = Point::new(MAX_COORDINATE, MAX_COORDINATE);
        assert_eq!(a.distance(b), 3_037_000_497);
    }
}
