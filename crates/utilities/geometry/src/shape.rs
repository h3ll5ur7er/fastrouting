use crate::{Point, Polygon, Rect, Segment};

/// The copper outline of a pad, track, via or obstacle.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Shape {
    /// All points within `radius` of `segment`: a track, or a circle when the
    /// segment has length zero.
    Capsule {
        segment: Segment,
        radius: i32,
    },
    Polygon(Polygon),
}

impl Shape {
    pub fn circle(center: Point, diameter: i32) -> Self {
        Self::Capsule {
            segment: Segment::new(center, center),
            radius: diameter / 2,
        }
    }

    pub fn track(segment: Segment, width: i32) -> Self {
        Self::Capsule {
            segment,
            radius: width / 2,
        }
    }

    pub fn bounds(&self) -> Rect {
        match self {
            Self::Capsule { segment, radius } => segment.bounds().inflated(*radius),
            Self::Polygon(polygon) => polygon.bounds(),
        }
    }

    /// Whether the two shapes come closer than `clearance` (overlapping counts).
    pub fn is_closer_than(&self, other: &Self, clearance: i32) -> bool {
        let clearance = i64::from(clearance);
        match (self, other) {
            (
                Self::Capsule {
                    segment: s,
                    radius: r,
                },
                Self::Capsule {
                    segment: t,
                    radius: q,
                },
            ) => s.is_closer_than(*t, clearance + i64::from(*r) + i64::from(*q)),
            (Self::Capsule { segment, radius }, Self::Polygon(polygon))
            | (Self::Polygon(polygon), Self::Capsule { segment, radius }) => {
                if polygon
                    .bounds()
                    .is_apart_from(segment.bounds(), clearance + i64::from(*radius))
                {
                    return false;
                }
                polygon.contains(segment.a)
                    || polygon
                        .edges()
                        .any(|edge| edge.is_closer_than(*segment, clearance + i64::from(*radius)))
            }
            (Self::Polygon(p), Self::Polygon(q)) => {
                if p.bounds().is_apart_from(q.bounds(), clearance) {
                    return false;
                }
                p.contains(q.points()[0])
                    || q.contains(p.points()[0])
                    || p.edges()
                        .any(|e| q.edges().any(|f| e.is_closer_than(f, clearance)))
            }
        }
    }

    /// Whether this shape lies inside `polygon`, at least `clearance` from its edges.
    pub fn is_inside(&self, polygon: &Polygon, clearance: i32) -> bool {
        let any_point = match self {
            Self::Capsule { segment, .. } => segment.a,
            Self::Polygon(p) => p.points()[0],
        };
        polygon.contains(any_point)
            && !polygon.edges().any(|edge| {
                Self::Capsule {
                    segment: edge,
                    radius: 0,
                }
                .is_closer_than(self, clearance)
            })
    }

    /// The point of the shape nearest to `p`: `p` itself when the shape covers
    /// it, else a point on its rim, rounded inwards.
    pub fn nearest_point(&self, p: Point) -> Point {
        match self {
            Self::Capsule { segment, radius } => {
                let centre = segment.closest_point(p);
                let distance = centre.distance(p);
                if distance <= i64::from(*radius) {
                    return p;
                }
                let toward = |from: i32, to: i32| {
                    let step = i64::from(to - from) * i64::from(*radius) / distance;
                    from + i32::try_from(step).unwrap_or(0)
                };
                Point::new(toward(centre.x, p.x), toward(centre.y, p.y))
            }
            Self::Polygon(polygon) if polygon.contains(p) => p,
            Self::Polygon(polygon) => polygon
                .edges()
                .map(|edge| edge.closest_point(p))
                .min_by_key(|q| q.squared_distance(p))
                .unwrap_or(p),
        }
    }

    pub fn map(&self, f: impl Fn(Point) -> Point) -> Self {
        match self {
            Self::Capsule { segment, radius } => Self::Capsule {
                segment: Segment::new(f(segment.a), f(segment.b)),
                radius: *radius,
            },
            Self::Polygon(polygon) => Self::Polygon(polygon.map(f)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_nearest_point_of_a_disc_is_on_its_rim_towards_the_point() {
        let disc = Shape::circle(Point::new(0, 0), 20);
        assert_eq!(disc.nearest_point(Point::new(30, 0)), Point::new(10, 0));
        assert_eq!(disc.nearest_point(Point::new(3, 4)), Point::new(3, 4));
    }

    #[test]
    fn the_nearest_point_of_a_polygon_is_on_its_nearest_edge() {
        let square = Shape::Polygon(Polygon::rectangle(Rect {
            min: Point::new(0, 0),
            max: Point::new(10, 10),
        }));
        assert_eq!(square.nearest_point(Point::new(15, 5)), Point::new(10, 5));
        assert_eq!(square.nearest_point(Point::new(5, 5)), Point::new(5, 5));
    }

    fn square(x: i32, y: i32, size: i32) -> Shape {
        Shape::Polygon(Polygon::rectangle(Rect {
            min: Point::new(x, y),
            max: Point::new(x + size, y + size),
        }))
    }

    #[test]
    fn circles_are_as_close_as_the_gap_between_their_rims() {
        let a = Shape::circle(Point::new(0, 0), 1_000);
        let b = Shape::circle(Point::new(1_200, 0), 1_000);
        assert!(!a.is_closer_than(&b, 200));
        assert!(a.is_closer_than(&b, 201));
    }

    #[test]
    fn a_track_through_a_pad_is_closer_than_anything() {
        let pad = square(0, 0, 1_000);
        let track = Shape::track(
            Segment::new(Point::new(-500, 500), Point::new(1_500, 500)),
            100,
        );
        assert!(pad.is_closer_than(&track, 0));
    }

    #[test]
    fn a_track_inside_a_large_pad_overlaps_it() {
        let pad = square(0, 0, 10_000);
        let track = Shape::track(
            Segment::new(Point::new(4_000, 5_000), Point::new(6_000, 5_000)),
            100,
        );
        assert!(track.is_closer_than(&pad, 0));
    }

    #[test]
    fn a_track_beside_a_pad_keeps_its_clearance() {
        let pad = square(0, 0, 1_000);
        let track = Shape::track(
            Segment::new(Point::new(1_300, -500), Point::new(1_300, 1_500)),
            200,
        );
        assert!(!track.is_closer_than(&pad, 200));
        assert!(track.is_closer_than(&pad, 201));
    }

    #[test]
    fn nested_polygons_overlap() {
        assert!(square(0, 0, 100).is_closer_than(&square(40, 40, 10), 0));
        assert!(square(40, 40, 10).is_closer_than(&square(0, 0, 100), 0));
    }

    #[test]
    fn separate_polygons_are_as_close_as_their_gap() {
        assert!(!square(0, 0, 100).is_closer_than(&square(150, 0, 100), 50));
        assert!(square(0, 0, 100).is_closer_than(&square(150, 0, 100), 51));
    }

    #[test]
    fn a_shape_inside_an_outline_keeps_its_clearance_to_the_edge() {
        let outline = Polygon::rectangle(Rect {
            min: Point::new(0, 0),
            max: Point::new(10_000, 10_000),
        });
        let near_edge = Shape::circle(Point::new(600, 5_000), 1_000);
        assert!(near_edge.is_inside(&outline, 100));
        assert!(!near_edge.is_inside(&outline, 101));
    }

    #[test]
    fn a_shape_outside_an_outline_is_not_inside() {
        let outline = Polygon::rectangle(Rect {
            min: Point::new(0, 0),
            max: Point::new(10_000, 10_000),
        });
        assert!(!Shape::circle(Point::new(20_000, 5_000), 1_000).is_inside(&outline, 0));
    }
}
