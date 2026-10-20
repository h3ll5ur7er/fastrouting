use traxide_contracts::geometry::{Point, Polygon, Rect, Segment, Shape};
use traxide_contracts::testing::{DesignBuilder, at, mm};
use traxide_contracts::{
    Design, KeepoutScope, LayerId, LayoutAccess, NetId, Track, Via, ViaKindId,
};
use traxide_layout_access::Layout;

const TOP: LayerId = LayerId(0);
const BOTTOM: LayerId = LayerId(1);

/// Two 1 mm pads of nets A and B, side by side in the middle of a 20 × 10 mm board.
fn board() -> Design {
    DesignBuilder::new(20.0, 10.0)
        .smd("P1", Some("A"), at(5.0, 5.0), 1.0, 0)
        .smd("P2", Some("B"), at(15.0, 5.0), 1.0, 0)
        .build()
}

fn segment(from: Point, to: Point) -> Segment {
    Segment::new(from, to)
}

fn check(design: &Design, f: impl FnOnce(&mut Layout<'_>, NetId, NetId)) {
    let rules = traxide_rules::compile(design, traxide_rules::Purpose::Routing);
    let mut layout = Layout::build(design, &rules);
    f(&mut layout, design.net_named("A"), design.net_named("B"));
}

#[test]
fn an_empty_stretch_of_board_is_clear() {
    check(&board(), |layout, a, _| {
        assert!(layout.is_track_clear(a, TOP, segment(at(8.0, 2.0), at(12.0, 2.0)), mm(0.2)));
    });
}

#[test]
fn a_track_may_run_into_a_pad_of_its_own_net() {
    check(&board(), |layout, a, _| {
        assert!(layout.is_track_clear(a, TOP, segment(at(5.0, 5.0), at(10.0, 5.0)), mm(0.2)));
    });
}

#[test]
fn a_track_must_not_touch_a_pad_of_another_net() {
    check(&board(), |layout, a, _| {
        assert!(!layout.is_track_clear(a, TOP, segment(at(10.0, 5.0), at(15.0, 5.0)), mm(0.2)));
    });
}

#[test]
fn a_track_keeps_exactly_the_clearance_from_another_net() {
    // The pad of B ends at x = 14.5 mm; a 0.2 mm track ends 0.1 mm past its centre line.
    check(&board(), |layout, a, _| {
        let up_to = |x: i32| segment(at(10.0, 5.0), Point::new(x, mm(5.0)));
        assert!(layout.is_track_clear(a, TOP, up_to(mm(14.5 - 0.2 - 0.1)), mm(0.2)));
        assert!(!layout.is_track_clear(a, TOP, up_to(mm(14.5 - 0.2 - 0.1) + 1), mm(0.2)));
    });
}

#[test]
fn layers_do_not_interfere() {
    check(&board(), |layout, a, _| {
        assert!(layout.is_track_clear(a, BOTTOM, segment(at(10.0, 5.0), at(15.0, 5.0)), mm(0.2)));
    });
}

#[test]
fn copper_keeps_its_clearance_from_the_board_edge() {
    check(&board(), |layout, a, _| {
        assert!(layout.is_track_clear(a, TOP, segment(at(1.0, 0.3), at(3.0, 0.3)), mm(0.2)));
        assert!(!layout.is_track_clear(a, TOP, segment(at(1.0, 0.29), at(3.0, 0.29)), mm(0.2)));
        assert!(!layout.is_track_clear(a, TOP, segment(at(19.0, 5.0), at(21.0, 5.0)), mm(0.2)));
    });
}

#[test]
fn added_tracks_block_other_nets_only() {
    check(&board(), |layout, a, b| {
        let across = segment(at(10.0, 1.0), at(10.0, 9.0));
        layout.add_track(&Track {
            net: a,
            layer: TOP,
            width: mm(0.2),
            path: vec![across.a, across.b],
        });
        let crossing = segment(at(8.0, 3.0), at(12.0, 3.0));
        assert!(!layout.is_track_clear(b, TOP, crossing, mm(0.2)));
        assert!(layout.is_track_clear(a, TOP, crossing, mm(0.2)));
    });
}

#[test]
fn a_via_needs_room_on_every_layer_it_spans() {
    let design = DesignBuilder::new(20.0, 10.0)
        .smd("P1", Some("A"), at(5.0, 5.0), 1.0, 1)
        .smd("P2", Some("B"), at(18.0, 5.0), 1.0, 1)
        .build();
    let rules = traxide_rules::compile(&design, traxide_rules::Purpose::Routing);
    let mut layout = Layout::build(&design, &rules);
    let other = design.net_named("B");
    let via = |x: f64| Via {
        net: other,
        kind: ViaKindId(0),
        position: at(x, 5.0),
    };
    assert!(
        !layout.is_via_clear(&via(5.9)),
        "the pad on the bottom layer is in the way"
    );
    assert!(layout.is_via_clear(&via(8.0)));
    layout.add_via(&via(8.0));
    assert!(!layout.is_track_clear(
        design.net_named("A"),
        TOP,
        segment(at(7.0, 5.0), at(9.0, 5.0)),
        mm(0.2)
    ));
}

#[test]
fn keepouts_block_what_they_are_meant_to_block() {
    let zone = Shape::Polygon(Polygon::rectangle(Rect {
        min: at(9.0, 0.0),
        max: at(11.0, 10.0),
    }));
    let design = DesignBuilder::new(20.0, 10.0)
        .smd("P1", Some("A"), at(5.0, 5.0), 1.0, 0)
        .keepout(zone, None, KeepoutScope::ViasOnly)
        .build();
    let rules = traxide_rules::compile(&design, traxide_rules::Purpose::Routing);
    let layout = Layout::build(&design, &rules);
    let a = design.net_named("A");
    assert!(layout.is_track_clear(a, TOP, segment(at(8.0, 5.0), at(12.0, 5.0)), mm(0.2)));
    assert!(!layout.is_via_clear(&Via {
        net: a,
        kind: ViaKindId(0),
        position: at(10.0, 5.0)
    }));
}
