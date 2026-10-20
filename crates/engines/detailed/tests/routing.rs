use traxide_contracts::geometry::{Point, Polygon, Rect, Shape};
use traxide_contracts::testing::{DesignBuilder, at};
use traxide_contracts::{
    Connection, Design, KeepoutScope, LayerId, Plan, Profile, Routes, RoutingResult, Violation,
};
use traxide_detailed::Router;
use traxide_events::{CancelToken, Silent};
use traxide_layout_access::Layout;
use traxide_rules::Purpose;

fn profile() -> Profile {
    Profile {
        name: "test".into(),
        grid_pitch: 100_000,
        via_cost: 2_000_000,
        bend_cost: 100_000,
        detour_margin: 5_000_000,
        rip_up_cost: 5_000_000,
        max_rip_ups: 10,
        heuristic_weight: 1.0,
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

/// Routes `pairs` as a plan, in that order.
fn route_with(design: &Design, profile: &Profile, pairs: &[(&str, &str)]) -> RoutingResult {
    let rules = traxide_rules::compile(design, Purpose::Routing);
    let mut layout = Layout::build(design, &rules);
    let plan = Plan {
        connections: pairs
            .iter()
            .map(|&(from, to)| connection(design, from, to))
            .collect(),
    };
    Router::new(design, &rules, profile).route_plan(
        &plan,
        &mut layout,
        &CancelToken::new(),
        &Silent,
    )
}

fn route(design: &Design, pairs: &[(&str, &str)]) -> RoutingResult {
    route_with(design, &profile(), pairs)
}

/// The routes break no rule, judged by the independent Verification Engine.
fn assert_legal(design: &Design, routes: &Routes) {
    let rules = traxide_rules::compile(design, Purpose::Verification);
    let verdict = traxide_verification::verify(design, &rules, routes);
    let broken: Vec<_> = verdict
        .violations
        .iter()
        .filter(|v| !matches!(v, Violation::Unconnected { .. }))
        .collect();
    assert!(broken.is_empty(), "{broken:?}");
}

fn wall(min: Point, max: Point) -> Shape {
    Shape::Polygon(Polygon::rectangle(Rect { min, max }))
}

fn is_octilinear(a: Point, b: Point) -> bool {
    let (dx, dy) = ((b.x - a.x).abs(), (b.y - a.y).abs());
    dx == 0 || dy == 0 || dx == dy
}

#[test]
fn a_clear_line_is_one_straight_track() {
    let design = DesignBuilder::new(20.0, 10.0)
        .smd("A1", Some("A"), at(5.0, 5.0), 1.0, 0)
        .smd("A2", Some("A"), at(15.0, 5.0), 1.0, 0)
        .build();
    let routes = route(&design, &[("A1", "A2")]).routes;
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
    let result = route(&design, &[("A1", "A2")]);
    assert!(result.unrouted.is_empty());
    assert!(result.routes.tracks[0].path.len() > 2, "goes around");
    assert_legal(&design, &result.routes);
}

#[test]
fn changes_layers_to_pass_a_wall_on_the_top() {
    let design = DesignBuilder::new(20.0, 10.0)
        .smd("A1", Some("A"), at(5.0, 5.0), 1.0, 0)
        .smd("A2", Some("A"), at(15.0, 5.0), 1.0, 0)
        .keepout(
            wall(at(9.5, 0.0), at(10.5, 10.0)),
            Some(0),
            KeepoutScope::Everything,
        )
        .build();
    let routes = route(&design, &[("A1", "A2")]).routes;
    assert_eq!(routes.vias.len(), 2);
    assert!(routes.tracks.iter().any(|t| t.layer == LayerId(1)));
    assert_legal(&design, &routes);
}

#[test]
fn reports_a_connection_it_cannot_route() {
    let design = DesignBuilder::new(20.0, 10.0)
        .smd("A1", Some("A"), at(5.0, 5.0), 1.0, 0)
        .smd("A2", Some("A"), at(15.0, 5.0), 1.0, 0)
        .keepout(
            wall(at(9.5, 0.0), at(10.5, 10.0)),
            None,
            KeepoutScope::Everything,
        )
        .build();
    let result = route(&design, &[("A1", "A2")]);
    assert_eq!(result.unrouted, [connection(&design, "A1", "A2")]);
    assert_eq!(result.routes, Routes::default());
}

#[test]
fn later_connections_keep_clear_of_earlier_ones() {
    let design = DesignBuilder::new(20.0, 20.0)
        .smd("A1", Some("A"), at(2.0, 10.0), 1.0, 0)
        .smd("A2", Some("A"), at(18.0, 10.0), 1.0, 0)
        .smd("B1", Some("B"), at(10.0, 2.0), 1.0, 0)
        .smd("B2", Some("B"), at(10.0, 18.0), 1.0, 0)
        .build();
    let result = route(&design, &[("A1", "A2"), ("B1", "B2")]);
    assert!(result.unrouted.is_empty());
    assert!(
        !result.routes.vias.is_empty(),
        "the crossing needs the other layer"
    );
    assert_legal(&design, &result.routes);
}

#[test]
fn reaches_pads_that_sit_off_the_grid() {
    let design = DesignBuilder::new(20.0, 10.0)
        .smd("A1", Some("A"), at(5.03, 5.07), 1.0, 0)
        .smd("A2", Some("A"), at(15.01, 3.33), 1.0, 0)
        .build();
    let routes = route(&design, &[("A1", "A2")]).routes;
    let path = &routes.tracks[0].path;
    assert_eq!(
        (path[0], path[path.len() - 1]),
        (at(5.03, 5.07), at(15.01, 3.33))
    );
    assert_legal(&design, &routes);
}

#[test]
fn pads_in_sight_of_each_other_are_joined_at_45_degrees_not_at_any_angle() {
    let design = DesignBuilder::new(20.0, 12.0)
        .smd("A1", Some("A"), at(5.0, 5.0), 1.0, 0)
        .smd("A2", Some("A"), at(15.0, 8.0), 1.0, 0)
        .build();
    let routes = route(&design, &[("A1", "A2")]).routes;
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
    let path = &route(&design, &[("A1", "A2")]).routes.tracks[0].path;
    assert_eq!(path, &[at(5.0, 5.0), at(12.0, 5.0), at(15.0, 8.0)]);
}

/// One layer, a wall down the middle with two gaps one track wide: a lower gap
/// at the board's bottom edge and a middle gap. The short connection Y, routed
/// first, could use either and takes the lower one; the long connection X
/// along the bottom edge can only reach the lower gap.
fn two_gaps() -> Design {
    DesignBuilder::new(20.0, 6.0)
        .layers(1)
        .without_vias()
        .keepout(
            wall(at(9.5, 0.0), at(10.5, 2.75)),
            None,
            KeepoutScope::Everything,
        )
        .keepout(
            wall(at(9.5, 3.25), at(10.5, 5.3)),
            None,
            KeepoutScope::Everything,
        )
        .smd("Y1", Some("Y"), at(8.0, 4.6), 0.4, 0)
        .smd("Y2", Some("Y"), at(12.0, 4.6), 0.4, 0)
        .smd("X1", Some("X"), at(2.0, 5.55), 0.4, 0)
        .smd("X2", Some("X"), at(18.0, 5.55), 0.4, 0)
        .build()
}

fn short_detours() -> Profile {
    Profile {
        detour_margin: 2_000_000,
        ..profile()
    }
}

#[test]
fn without_rip_up_a_connection_that_takes_anothers_only_way_leaves_it_unrouted() {
    let design = two_gaps();
    let result = route_with(
        &design,
        &Profile {
            max_rip_ups: 0,
            ..short_detours()
        },
        &[("Y1", "Y2"), ("X1", "X2")],
    );
    assert_eq!(result.unrouted, [connection(&design, "X1", "X2")]);
}

#[test]
fn rips_up_a_connection_in_the_way_and_routes_it_again_elsewhere() {
    let design = two_gaps();
    let result = route_with(&design, &short_detours(), &[("Y1", "Y2"), ("X1", "X2")]);
    assert_eq!(result.unrouted, []);
    assert_legal(&design, &result.routes);
}

#[test]
fn gives_up_when_two_connections_need_the_same_only_way() {
    let design = DesignBuilder::new(20.0, 6.0)
        .layers(1)
        .without_vias()
        .keepout(
            wall(at(9.5, 0.0), at(10.5, 5.3)),
            None,
            KeepoutScope::Everything,
        )
        .smd("Y1", Some("Y"), at(2.0, 5.55), 0.4, 0)
        .smd("Y2", Some("Y"), at(18.0, 5.55), 0.4, 0)
        .smd("X1", Some("X"), at(3.0, 5.55), 0.4, 0)
        .smd("X2", Some("X"), at(17.0, 5.55), 0.4, 0)
        .build();
    let result = route(&design, &[("X1", "X2"), ("Y1", "Y2")]);
    assert_eq!(result.unrouted.len(), 1);
    assert_legal(&design, &result.routes);
}
