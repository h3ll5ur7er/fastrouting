//! Joining pads to planes: a pad of a net with a plane that the plane does not
//! reach gets a short track to a via onto it, before anything is routed, so
//! that its net needs no track across the board.

use traxide_contracts::geometry::{Point, Segment, Shape};
use traxide_contracts::{
    ConnectionId, ConstraintSet, Design, LayerId, LayoutAccess, NetId, Pad, PadId, Routes, Track,
    Via,
};

/// How far beyond its pad a via onto a plane may go.
const LONGEST_REACH: i32 = 2_000_000;

/// The spacing of the places tried for a via: a via may fit only between the
/// straight ways out of a pad, where a coarser search misses it.
const PLACE_STEP: i32 = 50_000;

/// The design with the vias onto planes as copper of the board, so that
/// planning counts their pads as joined.
#[derive(Debug, Clone)]
pub struct PlaneJoins {
    pub design: Design,
    /// The tracks and vias added, which are routes to store.
    pub copper: Routes,
}

/// Joins each pad of a net with a plane, which no plane of its net joins yet,
/// to one by a via beside it, clear of the copper in `layout` and of the vias
/// before it, which are added to `layout`. A pad with no room for a via keeps
/// its connections.
pub fn join_planes(
    design: &Design,
    rules: &ConstraintSet,
    layout: &mut impl LayoutAccess,
) -> PlaneJoins {
    let on_planes = super::existing::pads_on_planes(design);
    let mut pads: Vec<PadId> = (0..design.pads.len())
        .map(PadId::from_index)
        .filter(|&pad| !on_planes[pad.index()] && has_a_plane_to_join(design, design.pad(pad)))
        .collect();
    pads.sort_by_key(|&pad| design.pad(pad).name.as_str());
    let farthest = pads.iter().map(|&pad| reach(design.pad(pad))).max();
    let places = places_nearest_first(farthest.unwrap_or(0));
    let mut copper = Routes::default();
    for pad in pads {
        let Some(join) = join_of(design, rules, layout, design.pad(pad), &places) else {
            continue;
        };
        // Under an id of its own, as no connection owns it, only so that the
        // vias after it keep clear of it.
        layout.add_route(ConnectionId::from_index(copper.vias.len()), &join);
        copper.extend(join);
    }
    let mut design = design.clone();
    design.existing.extend(copper.clone());
    PlaneJoins { design, copper }
}

/// Whether `pad` belongs to a net with other pads and a plane on a layer
/// that takes no tracks, and has copper on a layer that does.
fn has_a_plane_to_join(design: &Design, pad: &Pad) -> bool {
    let Some(net) = pad.net else { return false };
    design.net(net).pads.len() > 1
        && design
            .planes
            .iter()
            .any(|p| p.net == net && !design.layers[p.layer.index()].routable)
        && pad
            .copper
            .iter()
            .any(|c| design.layers[c.layer.index()].routable)
}

/// How far from its centre a via may join `pad` to a plane.
fn reach(pad: &Pad) -> i64 {
    let radius = pad
        .copper
        .iter()
        .map(|c| {
            let bounds = c.shape.bounds();
            (bounds.max.x - bounds.min.x).max(bounds.max.y - bounds.min.y) / 2
        })
        .max()
        .unwrap_or(0);
    i64::from(radius) + i64::from(LONGEST_REACH)
}

/// The points of a grid of PLACE_STEP no farther than `reach` from the
/// origin, by distance; ties go by position.
fn places_nearest_first(reach: i64) -> Vec<(i64, Point)> {
    let steps = i32::try_from(reach / i64::from(PLACE_STEP)).unwrap_or(0);
    let mut places: Vec<(i64, Point)> = (-steps..=steps)
        .flat_map(|x| (-steps..=steps).map(move |y| Point::new(x * PLACE_STEP, y * PLACE_STEP)))
        .map(|p| (p.distance(Point::default()), p))
        .filter(|&(distance, _)| distance <= reach)
        .collect();
    places.sort_by_key(|&(distance, p)| (distance, p.y, p.x));
    places
}

/// The shortest track from the centre of `pad` to a via beside it that
/// reaches a plane of its net, both clear of `layout`, at one of `places`
/// around the centre.
fn join_of(
    design: &Design,
    rules: &ConstraintSet,
    layout: &impl LayoutAccess,
    pad: &Pad,
    places: &[(i64, Point)],
) -> Option<Routes> {
    let net = pad.net?;
    let width = rules.track_width(net);
    let starts: Vec<_> = pad
        .copper
        .iter()
        .filter(|c| design.layers[c.layer.index()].routable)
        .collect();
    let reach = reach(pad);
    places
        .iter()
        .take_while(|&&(distance, _)| distance <= reach)
        .find_map(|&(_, offset)| {
            let at = pad.position + offset;
            starts.iter().find_map(|start| {
                let via = via_onto_a_plane(design, rules, net, start.layer, at)?;
                let beside_the_pad = !via_copper(design, &via).any(|(layer, shape)| {
                    layer == start.layer && shape.is_closer_than(&start.shape, 0)
                });
                let track = Segment::new(pad.position, at);
                (beside_the_pad
                    && layout.is_via_clear(&via)
                    && layout.is_track_clear(net, start.layer, track, width))
                .then(|| Routes {
                    tracks: vec![Track {
                        net,
                        layer: start.layer,
                        width,
                        path: vec![pad.position, at],
                    }],
                    vias: vec![via],
                })
            })
        })
}

/// The first via of the net's kinds at `at` that reaches from `layer` to a
/// plane of `net` it joins.
fn via_onto_a_plane(
    design: &Design,
    rules: &ConstraintSet,
    net: NetId,
    layer: LayerId,
    at: Point,
) -> Option<Via> {
    rules.via_kinds(net).iter().find_map(|&kind| {
        let via = Via {
            net,
            kind,
            position: at,
        };
        let joins = design.planes.iter().filter(|p| p.net == net).any(|plane| {
            design.via_kind(kind).connects(layer, plane.layer)
                && via_copper(design, &via).any(|(on, shape)| {
                    on == plane.layer && super::existing::joins_plane(design, plane, &shape)
                })
        });
        joins.then_some(via)
    })
}

fn via_copper<'d>(design: &'d Design, via: &Via) -> impl Iterator<Item = (LayerId, Shape)> + 'd {
    let offset = via.position;
    design.via_kind(via.kind).copper.iter().map(move |c| {
        (
            c.layer,
            c.shape.map(|p| Point::new(p.x + offset.x, p.y + offset.y)),
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use traxide_contracts::testing::{DesignBuilder, at, mm};
    use traxide_contracts::{KeepoutScope, Violation};
    use traxide_rules::Purpose;

    /// A 4-layer board 30 × 20 mm whose second layer is a GND plane all over.
    fn four_layers() -> DesignBuilder {
        DesignBuilder::new(30.0, 20.0)
            .layers(4)
            .plane_layer(1)
            .plane("GND", 1, at(0.0, 0.0), at(30.0, 20.0))
    }

    fn joined(design: &Design) -> PlaneJoins {
        let rules = traxide_rules::compile(
            design,
            Purpose::Routing {
                grid_pitch: 100_000,
            },
        );
        let mut layout = traxide_layout_access::Layout::build(design, &rules);
        join_planes(design, &rules, &mut layout)
    }

    fn broken_rules(design: &Design, copper: &Routes) -> Vec<Violation> {
        let rules = traxide_rules::compile(design, Purpose::Verification);
        traxide_verification::verify(design, &rules, copper).violations
    }

    #[test]
    fn smd_pads_of_a_net_with_a_plane_join_it_by_a_via_beside_them() {
        let design = four_layers()
            .smd("G1", Some("GND"), at(5.0, 5.0), 1.0, 0)
            .smd("G2", Some("GND"), at(25.0, 15.0), 1.0, 3)
            .build();
        let joins = joined(&design);
        assert_eq!((joins.copper.tracks.len(), joins.copper.vias.len()), (2, 2));
        for via in &joins.copper.vias {
            let near = [at(5.0, 5.0), at(25.0, 15.0)]
                .iter()
                .any(|&pad| via.position.distance(pad) < i64::from(mm(1.5)));
            assert!(near, "{via:?}");
        }
        assert!(crate::plan(&joins.design).connections.is_empty());
        assert_eq!(broken_rules(&design, &joins.copper), []);
    }

    #[test]
    fn the_via_keeps_clear_of_the_copper_of_other_nets() {
        // Pads of another net on all four sides of G1, 1.2 mm from its centre.
        let design = four_layers()
            .smd("G1", Some("GND"), at(5.0, 5.0), 1.0, 0)
            .tht("J1", Some("GND"), at(25.0, 15.0), 1.6)
            .smd("S1", None, at(6.2, 5.0), 0.6, 0)
            .smd("S2", None, at(3.8, 5.0), 0.6, 0)
            .smd("S3", None, at(5.0, 6.2), 0.6, 0)
            .smd("S4", None, at(5.0, 3.8), 0.6, 0)
            .build();
        let joins = joined(&design);
        assert_eq!(joins.copper.vias.len(), 1);
        assert_eq!(broken_rules(&design, &joins.copper), []);
    }

    #[test]
    fn a_via_finds_room_between_the_straight_ways_out() {
        // Vias may not go near the eight straight lines out of G1's centre.
        let mut builder = four_layers()
            .smd("G1", Some("GND"), at(5.0, 5.0), 1.0, 0)
            .tht("J1", Some("GND"), at(25.0, 15.0), 1.6);
        for (dx, dy) in [
            (1, 0),
            (1, 1),
            (0, 1),
            (-1, 1),
            (-1, 0),
            (-1, -1),
            (0, -1),
            (1, -1),
        ] {
            let end = at(5.0 + 3.0 * f64::from(dx), 5.0 + 3.0 * f64::from(dy));
            let line = Shape::track(Segment::new(at(5.0, 5.0), end), mm(0.1));
            builder = builder.keepout(line, None, KeepoutScope::ViasOnly);
        }
        let design = builder.build();
        let joins = joined(&design);
        assert_eq!(joins.copper.vias.len(), 1);
        assert_eq!(broken_rules(&design, &joins.copper), []);
    }

    #[test]
    fn pads_a_plane_joins_already_get_no_via() {
        let design = four_layers()
            .tht("J1", Some("GND"), at(5.0, 5.0), 1.6)
            .tht("J2", Some("GND"), at(25.0, 15.0), 1.6)
            .build();
        assert_eq!(joined(&design).copper, Routes::default());
    }

    #[test]
    fn a_pad_without_room_for_a_via_keeps_its_connection() {
        let design = four_layers()
            .smd("G1", Some("GND"), at(5.0, 5.0), 1.0, 0)
            .tht("J1", Some("GND"), at(25.0, 15.0), 1.6)
            .keepout(
                Shape::circle(at(5.0, 5.0), mm(10.0)),
                None,
                KeepoutScope::ViasOnly,
            )
            .build();
        let joins = joined(&design);
        assert_eq!(joins.copper, Routes::default());
        assert_eq!(crate::plan(&joins.design).connections.len(), 1);
    }

    #[test]
    fn nets_without_a_plane_get_no_vias() {
        let design = four_layers()
            .smd("A1", Some("A"), at(5.0, 5.0), 1.0, 0)
            .smd("A2", Some("A"), at(25.0, 15.0), 1.0, 0)
            .build();
        assert_eq!(joined(&design).copper, Routes::default());
    }

    #[test]
    fn a_pour_on_a_layer_that_takes_tracks_gets_no_vias() {
        let design = DesignBuilder::new(30.0, 20.0)
            .plane("GND", 1, at(0.0, 0.0), at(30.0, 20.0))
            .smd("G1", Some("GND"), at(5.0, 5.0), 1.0, 0)
            .smd("G2", Some("GND"), at(25.0, 15.0), 1.0, 0)
            .build();
        assert_eq!(joined(&design).copper, Routes::default());
    }
}
