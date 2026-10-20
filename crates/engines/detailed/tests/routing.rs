use traxide_contracts::geometry::{Polygon, Rect, Segment, Shape};
use traxide_contracts::testing::{DesignBuilder, at};
use traxide_contracts::{Connection, Design, KeepoutScope, LayerId, LayoutAccess, Profile, Routes};
use traxide_detailed::Router;
use traxide_layout_access::Layout;

fn profile() -> Profile {
    Profile {
        name: "test".into(),
        grid_pitch: 100_000,
        via_cost: 2_000_000,
        bend_cost: 100_000,
        detour_margin: 5_000_000,
    }
}

fn connection(design: &Design, from: &str, to: &str) -> Connection {
    let from = design.pad_named(from);
    Connection {
        net: design.pad(from).net.expect("pads of a net"),
        from,
        to: design.pad_named(to),
    }
}

/// Routes `pairs` in order, each against the copper of the ones before.
fn route(design: &Design, pairs: &[(&str, &str)]) -> Vec<Option<Routes>> {
    let rules = traxide_rules::compile(design, traxide_rules::Purpose::Routing);
    let mut layout = Layout::build(design, &rules);
    let profile = profile();
    let mut router = Router::new(design, &rules, &profile);
    pairs
        .iter()
        .map(|&(from, to)| router.route(connection(design, from, to), &mut layout))
        .collect()
}

/// Every segment and via of `routes` keeps its clearances from the design.
fn assert_legal(design: &Design, routes: &Routes) {
    let rules = traxide_rules::compile(design, traxide_rules::Purpose::Routing);
    let layout = Layout::build(design, &rules);
    for track in &routes.tracks {
        for pair in track.path.windows(2) {
            let segment = Segment::new(pair[0], pair[1]);
            assert!(
                layout.is_track_clear(track.net, track.layer, segment, track.width),
                "{segment:?} breaks a rule"
            );
        }
    }
    for via in &routes.vias {
        assert!(layout.is_via_clear(via), "{via:?} breaks a rule");
    }
}

fn wall(x_mm: f64, layer: Option<usize>) -> (Shape, Option<usize>) {
    (
        Shape::Polygon(Polygon::rectangle(Rect {
            min: at(x_mm, 0.0),
            max: at(x_mm + 1.0, 10.0),
        })),
        layer,
    )
}

#[test]
fn a_clear_line_is_one_straight_track() {
    let design = DesignBuilder::new(20.0, 10.0)
        .smd("A1", Some("A"), at(5.0, 5.0), 1.0, 0)
        .smd("A2", Some("A"), at(15.0, 5.0), 1.0, 0)
        .build();
    let routes = route(&design, &[("A1", "A2")]).remove(0).expect("routable");
    assert_eq!(routes.vias, []);
    assert_eq!(routes.tracks.len(), 1);
    assert_eq!(routes.tracks[0].layer, LayerId(0));
    assert_eq!(routes.tracks[0].path, [at(5.0, 5.0), at(15.0, 5.0)]);
}

#[test]
fn routes_around_a_pad_of_another_net() {
    let design = DesignBuilder::new(20.0, 10.0)
        .smd("A1", Some("A"), at(5.0, 5.0), 1.0, 0)
        .smd("A2", Some("A"), at(15.0, 5.0), 1.0, 0)
        .smd("B1", Some("B"), at(10.0, 5.0), 2.0, 0)
        .without_vias()
        .build();
    let routes = route(&design, &[("A1", "A2")]).remove(0).expect("routable");
    assert!(routes.tracks[0].path.len() > 2, "goes around");
    assert_legal(&design, &routes);
}

#[test]
fn changes_layers_to_pass_a_wall_on_the_top() {
    let (shape, layer) = wall(9.5, Some(0));
    let design = DesignBuilder::new(20.0, 10.0)
        .smd("A1", Some("A"), at(5.0, 5.0), 1.0, 0)
        .smd("A2", Some("A"), at(15.0, 5.0), 1.0, 0)
        .keepout(shape, layer, KeepoutScope::Everything)
        .build();
    let routes = route(&design, &[("A1", "A2")]).remove(0).expect("routable");
    assert_eq!(routes.vias.len(), 2);
    assert!(routes.tracks.iter().any(|t| t.layer == LayerId(1)));
    assert_legal(&design, &routes);
}

#[test]
fn reports_a_connection_it_cannot_route() {
    let (shape, layer) = wall(9.5, None);
    let design = DesignBuilder::new(20.0, 10.0)
        .smd("A1", Some("A"), at(5.0, 5.0), 1.0, 0)
        .smd("A2", Some("A"), at(15.0, 5.0), 1.0, 0)
        .keepout(shape, layer, KeepoutScope::Everything)
        .build();
    assert_eq!(route(&design, &[("A1", "A2")]), [None]);
}

#[test]
fn later_connections_keep_clear_of_earlier_ones() {
    let design = DesignBuilder::new(20.0, 20.0)
        .smd("A1", Some("A"), at(2.0, 10.0), 1.0, 0)
        .smd("A2", Some("A"), at(18.0, 10.0), 1.0, 0)
        .smd("B1", Some("B"), at(10.0, 2.0), 1.0, 0)
        .smd("B2", Some("B"), at(10.0, 18.0), 1.0, 0)
        .build();
    let results = route(&design, &[("A1", "A2"), ("B1", "B2")]);
    let mut all = Routes::default();
    for routes in results {
        all.extend(routes.expect("both are routable"));
    }
    assert!(!all.vias.is_empty(), "the crossing needs the other layer");
    let rules = traxide_rules::compile(&design, traxide_rules::Purpose::Routing);
    let mut layout = Layout::build(&design, &rules);
    // Each net's copper must keep clear of the other net's, so check B against A.
    for track in all.tracks.iter().filter(|t| t.net == design.net_named("A")) {
        layout.add_track(track);
    }
    for via in all.vias.iter().filter(|v| v.net == design.net_named("A")) {
        layout.add_via(via);
    }
    for track in all.tracks.iter().filter(|t| t.net == design.net_named("B")) {
        for pair in track.path.windows(2) {
            assert!(layout.is_track_clear(
                track.net,
                track.layer,
                Segment::new(pair[0], pair[1]),
                track.width
            ));
        }
    }
}

#[test]
fn reaches_pads_that_sit_off_the_grid() {
    let design = DesignBuilder::new(20.0, 10.0)
        .smd("A1", Some("A"), at(5.03, 5.07), 1.0, 0)
        .smd("A2", Some("A"), at(15.01, 3.33), 1.0, 0)
        .build();
    let routes = route(&design, &[("A1", "A2")]).remove(0).expect("routable");
    let path = &routes.tracks[0].path;
    assert_eq!(
        (path[0], path[path.len() - 1]),
        (at(5.03, 5.07), at(15.01, 3.33))
    );
    assert_legal(&design, &routes);
}

fn is_octilinear(
    a: traxide_contracts::geometry::Point,
    b: traxide_contracts::geometry::Point,
) -> bool {
    let (dx, dy) = ((b.x - a.x).abs(), (b.y - a.y).abs());
    dx == 0 || dy == 0 || dx == dy
}

#[test]
fn pads_in_sight_of_each_other_are_joined_at_45_degrees_not_at_any_angle() {
    let design = DesignBuilder::new(20.0, 12.0)
        .smd("A1", Some("A"), at(5.0, 5.0), 1.0, 0)
        .smd("A2", Some("A"), at(15.0, 8.0), 1.0, 0)
        .build();
    let routes = route(&design, &[("A1", "A2")]).remove(0).expect("routable");
    let path = &routes.tracks[0].path;
    assert_eq!(path.len(), 3, "one bend: {path:?}");
    assert!(
        path.windows(2).all(|p| is_octilinear(p[0], p[1])),
        "{path:?}"
    );
    assert_legal(&design, &routes);
}

#[test]
fn takes_the_other_bend_when_the_first_is_blocked() {
    // A pad of net B sits where the diagonal-first bend would pass.
    let design = DesignBuilder::new(20.0, 12.0)
        .smd("A1", Some("A"), at(5.0, 5.0), 1.0, 0)
        .smd("A2", Some("A"), at(15.0, 8.0), 1.0, 0)
        .smd("B1", Some("B"), at(8.0, 8.0), 1.0, 0)
        .without_vias()
        .build();
    let routes = route(&design, &[("A1", "A2")]).remove(0).expect("routable");
    let path = &routes.tracks[0].path;
    assert_eq!(path, &[at(5.0, 5.0), at(12.0, 5.0), at(15.0, 8.0)]);
}
