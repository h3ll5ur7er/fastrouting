use traxide_contracts::geometry::Point;
use traxide_contracts::testing::{DesignBuilder, at, mm};
use traxide_contracts::{
    CopperItem, Design, LayerId, Routes, Track, Verdict, Via, ViaKindId, Violation,
};
use traxide_rules::{Purpose, compile};
use traxide_verification::verify;

/// Net A from (2, 5) to (18, 5) mm, net B's pad in the middle at (10, 8) mm.
fn board() -> Design {
    DesignBuilder::new(20.0, 10.0)
        .smd("A1", Some("A"), at(2.0, 5.0), 1.0, 0)
        .smd("A2", Some("A"), at(18.0, 5.0), 1.0, 0)
        .smd("B1", Some("B"), at(10.0, 8.0), 1.0, 0)
        .build()
}

fn track(design: &Design, net: &str, layer: u32, path: &[Point]) -> Track {
    Track {
        net: design.net_named(net),
        layer: LayerId(layer),
        width: mm(0.2),
        path: path.to_vec(),
    }
}

fn check(design: &Design, routes: Routes) -> Verdict {
    verify(design, &compile(design, Purpose::Verification), &routes)
}

#[test]
fn a_clean_route_passes() {
    let design = board();
    let routes = Routes {
        tracks: vec![track(&design, "A", 0, &[at(2.0, 5.0), at(18.0, 5.0)])],
        vias: vec![],
    };
    assert_eq!(check(&design, routes), Verdict::default());
}

#[test]
fn missing_copper_leaves_pads_unconnected() {
    let design = board();
    let verdict = check(&design, Routes::default());
    assert_eq!(
        verdict.violations,
        [Violation::Unconnected {
            net: design.net_named("A"),
            pads: vec![design.pad_named("A2")]
        }]
    );
}

#[test]
fn a_track_too_close_to_another_net_is_reported_once() {
    let design = board();
    let routes = Routes {
        tracks: vec![track(
            &design,
            "A",
            0,
            &[at(2.0, 5.0), at(8.0, 7.6), at(12.0, 7.6), at(18.0, 5.0)],
        )],
        vias: vec![],
    };
    let clearance: Vec<_> = check(&design, routes)
        .violations
        .into_iter()
        .filter(|v| matches!(v, Violation::Clearance { .. }))
        .collect();
    assert_eq!(
        clearance,
        [Violation::Clearance {
            layer: LayerId(0),
            a: CopperItem::Pad(design.pad_named("B1")),
            b: CopperItem::Track(0),
            required: mm(0.2)
        }]
    );
}

#[test]
fn copper_near_the_edge_is_reported() {
    let design = board();
    let routes = Routes {
        tracks: vec![track(
            &design,
            "A",
            0,
            &[at(2.0, 5.0), at(2.0, 0.2), at(18.0, 0.2), at(18.0, 5.0)],
        )],
        vias: vec![],
    };
    assert!(
        check(&design, routes)
            .violations
            .contains(&Violation::Edge {
                layer: LayerId(0),
                item: CopperItem::Track(0)
            })
    );
}

#[test]
fn copper_near_a_cutout_inside_the_board_is_reported() {
    use traxide_contracts::geometry::{Polygon, Rect};
    // A cutout from (9, 6) to (11, 9) mm, as a footprint may draw on Edge.Cuts;
    // the track passes 0.1 mm below it, closer than the 0.2 mm edge clearance.
    let mut design = board();
    let cutout = Polygon::rectangle(Rect {
        min: at(9.0, 6.0),
        max: at(11.0, 9.0),
    });
    design.outline = design.outline.clone().with_holes(vec![cutout]);
    let routes = Routes {
        tracks: vec![track(&design, "A", 0, &[at(2.0, 5.0), at(18.0, 5.0)])],
        vias: vec![],
    };
    assert!(
        check(&design, routes.clone()).is_clean(),
        "5 mm is clear of it"
    );
    let close = Routes {
        tracks: vec![track(
            &design,
            "A",
            0,
            &[
                at(2.0, 5.0),
                at(8.0, 5.0),
                at(8.0, 5.8),
                at(12.0, 5.8),
                at(12.0, 5.0),
                at(18.0, 5.0),
            ],
        )],
        vias: vec![],
    };
    assert!(check(&design, close).violations.contains(&Violation::Edge {
        layer: LayerId(0),
        item: CopperItem::Track(0)
    }));
}

#[test]
fn vias_connect_the_layers_they_span() {
    let design = board();
    let a = design.net_named("A");
    let routes = Routes {
        tracks: vec![
            track(&design, "A", 0, &[at(2.0, 5.0), at(5.0, 5.0)]),
            track(&design, "A", 1, &[at(5.0, 5.0), at(15.0, 5.0)]),
            track(&design, "A", 0, &[at(15.0, 5.0), at(18.0, 5.0)]),
        ],
        vias: vec![
            Via {
                net: a,
                kind: ViaKindId(0),
                position: at(5.0, 5.0),
            },
            Via {
                net: a,
                kind: ViaKindId(0),
                position: at(15.0, 5.0),
            },
        ],
    };
    assert_eq!(check(&design, routes), Verdict::default());
}

#[test]
fn copper_on_the_wrong_layer_does_not_connect() {
    let design = board();
    let routes = Routes {
        tracks: vec![track(&design, "A", 1, &[at(2.0, 5.0), at(18.0, 5.0)])],
        vias: vec![],
    };
    assert!(matches!(
        check(&design, routes).violations[..],
        [Violation::Unconnected { .. }]
    ));
}

#[test]
fn pads_of_different_nets_closer_than_their_clearance_are_named_once_a_pair() {
    // A1 and B1 through both layers, 0.1 mm apart; A1 and A2 as close, but of one net.
    let design = DesignBuilder::new(20.0, 10.0)
        .tht("A1", Some("A"), at(5.0, 5.0), 1.0)
        .tht("B1", Some("B"), at(6.1, 5.0), 1.0)
        .tht("A2", Some("A"), at(3.9, 5.0), 1.0)
        .tht("C1", Some("C"), at(15.0, 5.0), 1.0)
        .build();
    let rules = compile(&design, Purpose::Verification);
    assert_eq!(
        traxide_verification::pads_too_close(&design, &rules),
        [[design.pad_named("A1"), design.pad_named("B1")]]
    );
}

#[test]
fn pads_of_one_part_are_named_only_where_they_overlap() {
    // A footprint may set its pins closer than the board's clearance: U1's are
    // 0.1 mm apart. U2's overlap, which shorts their nets.
    let design = DesignBuilder::new(20.0, 10.0)
        .smd("U1-1", Some("A"), at(5.0, 5.0), 0.5, 0)
        .smd("U1-2", Some("B"), at(5.6, 5.0), 0.5, 0)
        .smd("U2-1", Some("A"), at(10.0, 5.0), 0.5, 0)
        .smd("U2-2", Some("B"), at(10.4, 5.0), 0.5, 0)
        .build();
    let rules = compile(&design, Purpose::Verification);
    assert_eq!(
        traxide_verification::pads_too_close(&design, &rules),
        [[design.pad_named("U2-1"), design.pad_named("U2-2")]]
    );
}

/// A 3-layer board whose middle layer is a GND plane over its left half, with
/// a GND pad on the top layer over the plane and one outside it.
fn board_with_a_plane() -> DesignBuilder {
    DesignBuilder::new(20.0, 10.0)
        .layers(3)
        .plane_layer(1)
        .plane("GND", 1, at(0.0, 0.0), at(10.0, 10.0))
        .smd("G1", Some("GND"), at(2.0, 5.0), 1.0, 0)
}

fn via(design: &Design, net: &str, position: Point) -> Via {
    Via {
        net: design.net_named(net),
        kind: ViaKindId(0),
        position,
    }
}

#[test]
fn a_plane_joins_the_through_hole_pads_and_vias_of_its_net_on_it() {
    let design = board_with_a_plane()
        .tht("J1", Some("GND"), at(8.0, 2.0), 1.6)
        .build();
    let routes = Routes {
        tracks: vec![track(&design, "GND", 0, &[at(2.0, 5.0), at(3.0, 5.0)])],
        vias: vec![via(&design, "GND", at(3.0, 5.0))],
    };
    assert_eq!(check(&design, routes), Verdict::default());
}

#[test]
fn copper_outside_the_plane_is_not_joined_by_it() {
    let design = board_with_a_plane()
        .tht("J1", Some("GND"), at(15.0, 2.0), 1.6)
        .build();
    let routes = Routes {
        tracks: vec![track(&design, "GND", 0, &[at(2.0, 5.0), at(3.0, 5.0)])],
        vias: vec![via(&design, "GND", at(3.0, 5.0))],
    };
    assert_eq!(
        check(&design, routes).violations,
        [Violation::Unconnected {
            net: design.net_named("GND"),
            pads: vec![design.pad_named("J1")]
        }]
    );
}

#[test]
fn a_plane_joins_nothing_where_a_plane_of_another_net_overlaps_it() {
    // Which of two overlapping pours fills the overlap is the board program's
    // choice: their order of priority, which the design does not know.
    let design = board_with_a_plane()
        .plane("VCC", 1, at(6.0, 0.0), at(10.0, 4.0))
        .tht("J1", Some("GND"), at(8.0, 2.0), 1.6)
        .tht("V1", Some("VCC"), at(18.0, 8.0), 1.6)
        .build();
    let routes = Routes {
        tracks: vec![track(&design, "GND", 0, &[at(2.0, 5.0), at(3.0, 5.0)])],
        vias: vec![via(&design, "GND", at(3.0, 5.0))],
    };
    assert_eq!(
        check(&design, routes).violations,
        [Violation::Unconnected {
            net: design.net_named("GND"),
            pads: vec![design.pad_named("J1")]
        }]
    );
}

#[test]
fn a_pour_on_a_layer_that_takes_tracks_joins_nothing() {
    // Tracks of other nets may cut such a pour apart, wherever routing put them.
    let design = DesignBuilder::new(20.0, 10.0)
        .plane("GND", 1, at(0.0, 0.0), at(20.0, 10.0))
        .tht("J1", Some("GND"), at(2.0, 2.0), 1.6)
        .tht("J2", Some("GND"), at(18.0, 2.0), 1.6)
        .build();
    assert_eq!(
        check(&design, Routes::default()).violations,
        [Violation::Unconnected {
            net: design.net_named("GND"),
            pads: vec![design.pad_named("J2")]
        }]
    );
}
