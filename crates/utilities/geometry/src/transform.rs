use crate::Point;

/// Places a footprint's local coordinates on the board: mirror (for the back
/// side), then rotate counter-clockwise, then translate.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Transform {
    pub origin: Point,
    pub rotation_degrees: f64,
    pub mirror_x: bool,
}

impl Transform {
    pub const IDENTITY: Self = Self {
        origin: Point::new(0, 0),
        rotation_degrees: 0.0,
        mirror_x: false,
    };

    pub fn apply(&self, p: Point) -> Point {
        let p = if self.mirror_x {
            Point::new(-p.x, p.y)
        } else {
            p
        };
        rotate(p, self.rotation_degrees) + self.origin
    }

    pub fn then(self, outer: Self) -> impl Fn(Point) -> Point {
        move |p| outer.apply(self.apply(p))
    }
}

fn rotate(p: Point, degrees: f64) -> Point {
    let quarter_turns = degrees / 90.0;
    // Right angles, by far the most common, are rotated exactly; trigonometry
    // would round them and could differ between platforms.
    if quarter_turns.fract() == 0.0 {
        return match (quarter_turns as i64).rem_euclid(4) {
            0 => p,
            1 => Point::new(-p.y, p.x),
            2 => Point::new(-p.x, -p.y),
            _ => Point::new(p.y, -p.x),
        };
    }
    let (sin, cos) = degrees.to_radians().sin_cos();
    let (x, y) = (f64::from(p.x), f64::from(p.y));
    Point::new(
        (x * cos - y * sin).round() as i32,
        (x * sin + y * cos).round() as i32,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(origin: Point, rotation_degrees: f64, mirror_x: bool) -> Transform {
        Transform {
            origin,
            rotation_degrees,
            mirror_x,
        }
    }

    #[test]
    fn rotates_counter_clockwise_then_translates() {
        let t = at(Point::new(100, 100), 90.0, false);
        assert_eq!(t.apply(Point::new(10, 0)), Point::new(100, 110));
    }

    #[test]
    fn negative_right_angles_are_exact() {
        assert_eq!(
            at(Point::new(0, 0), -90.0, false).apply(Point::new(10, 0)),
            Point::new(0, -10)
        );
        assert_eq!(
            at(Point::new(0, 0), 270.0, false).apply(Point::new(10, 0)),
            Point::new(0, -10)
        );
    }

    #[test]
    fn mirrors_before_rotating() {
        let t = at(Point::new(0, 0), 90.0, true);
        assert_eq!(t.apply(Point::new(10, 0)), Point::new(0, -10));
    }

    #[test]
    fn other_angles_round_to_the_nearest_nanometre() {
        let t = at(Point::new(0, 0), 45.0, false);
        assert_eq!(t.apply(Point::new(1_000, 0)), Point::new(707, 707));
    }

    #[test]
    fn then_applies_the_inner_transform_first() {
        let pin = at(Point::new(10, 0), 0.0, false);
        let footprint = at(Point::new(1_000, 0), 180.0, false);
        assert_eq!(pin.then(footprint)(Point::new(0, 0)), Point::new(990, 0));
    }
}
