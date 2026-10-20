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
