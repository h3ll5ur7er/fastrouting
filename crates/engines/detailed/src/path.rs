use traxide_contracts::geometry::Point;
use traxide_contracts::{LayerId, NetId, Routes, Track, Via, ViaKindId};

/// A point of a found path, and the via taken to reach its layer, if any.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Step {
    pub(crate) at: Point,
    pub(crate) layer: LayerId,
    pub(crate) via: Option<ViaKindId>,
}

/// One track per stretch on a layer and a via at each change of layer that
/// takes one, with the points that lie on a straight line dropped.
pub(crate) fn to_routes(steps: &[Step], net: NetId, width: i32) -> Routes {
    let mut routes = Routes::default();
    let mut stretch: Vec<Point> = Vec::new();
    let mut layer = steps.first().map_or(LayerId(0), |s| s.layer);
    for step in steps {
        if step.layer != layer || step.via.is_some() {
            finish(&mut routes, &mut stretch, net, layer, width);
            if let Some(kind) = step.via {
                routes.vias.push(Via {
                    net,
                    kind,
                    position: step.at,
                });
            }
            layer = step.layer;
        }
        if stretch.last() != Some(&step.at) {
            stretch.push(step.at);
        }
    }
    finish(&mut routes, &mut stretch, net, layer, width);
    routes
}

/// The same path walked the other way: each via now belongs to the step that
/// follows it in the new direction.
pub(crate) fn reversed(steps: Vec<Step>) -> Vec<Step> {
    let count = steps.len();
    (0..count)
        .map(|j| {
            let step = steps[count - 1 - j];
            let via = if j == 0 { None } else { steps[count - j].via };
            Step { via, ..step }
        })
        .collect()
}

fn finish(routes: &mut Routes, stretch: &mut Vec<Point>, net: NetId, layer: LayerId, width: i32) {
    let path = straightened(std::mem::take(stretch));
    if path.len() >= 2 {
        routes.tracks.push(Track {
            net,
            layer,
            width,
            path: path.clone(),
        });
    }
    stretch.extend(path.last());
}

fn straightened(points: Vec<Point>) -> Vec<Point> {
    let mut path: Vec<Point> = Vec::with_capacity(points.len());
    for p in points {
        while path.len() >= 2 && continues_straight(path[path.len() - 2], path[path.len() - 1], p) {
            path.pop();
        }
        path.push(p);
    }
    path
}

fn continues_straight(a: Point, b: Point, c: Point) -> bool {
    let (ab, bc) = (b - a, c - b);
    let cross = i64::from(ab.x) * i64::from(bc.y) - i64::from(ab.y) * i64::from(bc.x);
    let dot = i64::from(ab.x) * i64::from(bc.x) + i64::from(ab.y) * i64::from(bc.y);
    cross == 0 && dot > 0
}

#[cfg(test)]
mod tests {
    use super::*;

    fn step(x: i32, y: i32, layer: u32, via: Option<u32>) -> Step {
        Step {
            at: Point::new(x, y),
            layer: LayerId(layer),
            via: via.map(ViaKindId),
        }
    }

    #[test]
    fn collinear_points_are_dropped() {
        let routes = to_routes(
            &[
                step(0, 0, 0, None),
                step(1, 0, 0, None),
                step(2, 0, 0, None),
                step(3, 1, 0, None),
            ],
            NetId(0),
            5,
        );
        assert_eq!(
            routes.tracks[0].path,
            [Point::new(0, 0), Point::new(2, 0), Point::new(3, 1)]
        );
    }

    #[test]
    fn a_change_of_layer_places_a_via_and_starts_a_new_track() {
        let routes = to_routes(
            &[
                step(0, 0, 0, None),
                step(5, 0, 0, None),
                step(5, 0, 1, Some(0)),
                step(9, 0, 1, None),
            ],
            NetId(3),
            5,
        );
        assert_eq!(
            routes.vias,
            [Via {
                net: NetId(3),
                kind: ViaKindId(0),
                position: Point::new(5, 0)
            }]
        );
        assert_eq!(routes.tracks.len(), 2);
        assert_eq!(
            (routes.tracks[0].layer, routes.tracks[1].layer),
            (LayerId(0), LayerId(1))
        );
        assert_eq!(routes.tracks[1].path, [Point::new(5, 0), Point::new(9, 0)]);
    }

    #[test]
    fn a_reversed_path_has_its_via_at_the_same_place() {
        let forward = vec![
            step(0, 0, 0, None),
            step(5, 0, 0, None),
            step(5, 0, 1, Some(0)),
            step(9, 0, 1, None),
        ];
        let backward = reversed(forward);
        assert_eq!(
            backward,
            [
                step(9, 0, 1, None),
                step(5, 0, 1, None),
                step(5, 0, 0, Some(0)),
                step(0, 0, 0, None)
            ]
        );
        let routes = to_routes(&backward, NetId(3), 5);
        assert_eq!(routes.vias.len(), 1);
        assert_eq!(routes.vias[0].position, Point::new(5, 0));
    }

    #[test]
    fn a_via_right_at_a_pad_leaves_no_empty_track() {
        let routes = to_routes(
            &[
                step(0, 0, 0, None),
                step(0, 0, 1, Some(0)),
                step(4, 0, 1, None),
            ],
            NetId(0),
            5,
        );
        assert_eq!(routes.tracks.len(), 1);
        assert_eq!(routes.vias.len(), 1);
    }
}
