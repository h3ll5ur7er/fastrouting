use traxide_contracts::geometry::{Point, Polygon, Rect, Shape};
use traxide_contracts::testing::{DesignBuilder, at, mm};
use traxide_contracts::{
    Connection, ConnectionId, Corridors, Design, Event, KeepoutScope, LayerId, Plan, Profile,
    Routes, RoutingResult, Violation,
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
        history_cost: 0,
        heuristic_weight: 1.0,
        global_routing: false,
        batch_size: 1,
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
    route_in(design, profile, pairs, None)
}

fn route_in(
    design: &Design,
    profile: &Profile,
    pairs: &[(&str, &str)],
    corridors: Option<&Corridors>,
) -> RoutingResult {
    route_on_threads(design, profile, pairs, corridors, 1)
}

fn route_on_threads(
    design: &Design,
    profile: &Profile,
    pairs: &[(&str, &str)],
    corridors: Option<&Corridors>,
    threads: usize,
) -> RoutingResult {
    let rules = traxide_rules::compile(
        design,
        Purpose::Routing {
            grid_pitch: 100_000,
        },
    );
    let mut layout = Layout::build(design, &rules);
    let connections = pairs
        .iter()
        .map(|&(from, to)| connection(design, from, to))
        .collect();
    let plan = Plan { connections };
    let cancel = CancelToken::new();
    Router::new(design, &rules, profile)
        .with_threads(threads)
        .route_plan(&plan, corridors, &mut layout, &cancel, &Silent)
}

/// A corridor of 1 mm cells on a board 20 cells wide, made of the given cells.
fn corridor(cells: impl IntoIterator<Item = (i32, i32)>) -> Corridors {
    let mut cells: Vec<u32> = cells
        .into_iter()
        .map(|(column, row)| (row * 20 + column) as u32)
        .collect();
    cells.sort_unstable();
    cells.dedup();
    Corridors::new(Point::new(0, 0), 1_000_000, 20, vec![cells])
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
fn connections_that_may_cross_are_searched_one_after_the_other() {
    let design = DesignBuilder::new(20.0, 20.0)
        .smd("A1", Some("A"), at(2.0, 10.0), 1.0, 0)
        .smd("A2", Some("A"), at(18.0, 10.0), 1.0, 0)
        .smd("B1", Some("B"), at(10.0, 2.0), 1.0, 0)
        .smd("B2", Some("B"), at(10.0, 18.0), 1.0, 0)
        .build();
    let in_one_batch = Profile {
        batch_size: 2,
        ..profile()
    };
    let result = route_on_threads(
        &design,
        &in_one_batch,
        &[("A1", "A2"), ("B1", "B2")],
        None,
        2,
    );
    assert!(result.unrouted.is_empty());
    assert!(
        !result.routes.vias.is_empty(),
        "the crossing needs the other layer"
    );
    assert_legal(&design, &result.routes);
}

/// Twelve two-pin nets with their pads scattered over a 2 mm lattice, so that
/// many of them cross and collide.
fn scattered_nets() -> (Design, Vec<(String, String)>) {
    let mut builder = DesignBuilder::new(22.0, 22.0);
    let mut sites: Vec<(i32, i32)> = (0..10).flat_map(|x| (0..10).map(move |y| (x, y))).collect();
    let mut seed = 7_u64;
    let mut pairs = Vec::new();
    for net in 0..12 {
        let mut ends = Vec::new();
        for end in 1..=2 {
            seed = seed.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1);
            let (x, y) = sites.swap_remove((seed >> 33) as usize % sites.len());
            let name = format!("N{net}.{end}");
            builder = builder.smd(
                &name,
                Some(&format!("N{net}")),
                at(2.0 + 2.0 * f64::from(x), 2.0 + 2.0 * f64::from(y)),
                0.8,
                0,
            );
            ends.push(name);
        }
        pairs.push((ends[0].clone(), ends[1].clone()));
    }
    (builder.build(), pairs)
}

#[test]
fn the_routes_do_not_depend_on_the_number_of_threads() {
    let (design, pairs) = scattered_nets();
    let pairs: Vec<(&str, &str)> = pairs
        .iter()
        .map(|(a, b)| (a.as_str(), b.as_str()))
        .collect();
    let batched = Profile {
        batch_size: 8,
        heuristic_weight: 1.2,
        ..profile()
    };
    let on_one = route_on_threads(&design, &batched, &pairs, None, 1);
    assert!(on_one.unrouted.is_empty());
    assert_legal(&design, &on_one.routes);
    for threads in [2, 4] {
        let on_more = route_on_threads(&design, &batched, &pairs, None, threads);
        assert_eq!(on_more.routes, on_one.routes, "on {threads} threads");
    }
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
            history_cost: 0,
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

fn one_way_for_two() -> Design {
    DesignBuilder::new(20.0, 6.0)
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
        .build()
}

#[test]
fn gives_up_when_two_connections_need_the_same_only_way() {
    let design = one_way_for_two();
    let result = route(&design, &[("X1", "X2"), ("Y1", "Y2")]);
    assert_eq!(result.unrouted.len(), 1);
    assert_legal(&design, &result.routes);
}

#[test]
fn stops_ripping_up_once_a_round_of_rip_ups_routes_no_more_connections() {
    let design = one_way_for_two();
    let rules = traxide_rules::compile(
        &design,
        Purpose::Routing {
            grid_pitch: 100_000,
        },
    );
    let mut layout = Layout::build(&design, &rules);
    let plan = Plan {
        connections: vec![
            connection(&design, "X1", "X2"),
            connection(&design, "Y1", "Y2"),
        ],
    };
    let commits = std::cell::Cell::new(0);
    let count = |event: Event| {
        if matches!(event, Event::ConnectionRouted { .. }) {
            commits.set(commits.get() + 1);
        }
    };
    let result = Router::new(&design, &rules, &profile()).route_plan(
        &plan,
        None,
        &mut layout,
        &CancelToken::new(),
        &count,
    );
    assert_eq!(result.unrouted.len(), 1);
    // Each taking the gap from the other until both were ripped up ten times
    // committed 21 routes.
    assert!(commits.get() <= 5, "{} routes committed", commits.get());
}

#[test]
fn a_connection_searched_from_its_tighter_end_still_runs_from_its_first_pad() {
    // The small pad offers fewer ways in than the large through-hole pad, so the
    // search starts there; the routes must still read from A1 to A2.
    let design = DesignBuilder::new(20.0, 10.0)
        .tht("A1", Some("A"), at(4.0, 5.0), 2.2)
        .smd("A2", Some("A"), at(15.0, 5.0), 0.3, 0)
        .keepout(
            wall(at(9.5, 0.0), at(10.5, 10.0)),
            Some(0),
            KeepoutScope::Everything,
        )
        .build();
    let routes = route(&design, &[("A1", "A2")]).routes;
    assert_eq!(
        routes.vias.len(),
        1,
        "A1 reaches the bottom layer itself; one via back up"
    );
    let starts: Vec<Point> = routes.tracks.iter().map(|t| t.path[0]).collect();
    let ends: Vec<Point> = routes
        .tracks
        .iter()
        .map(|t| *t.path.last().expect("a path"))
        .collect();
    assert_eq!(starts[0], at(4.0, 5.0));
    assert_eq!(ends[ends.len() - 1], at(15.0, 5.0));
    assert_eq!(
        routes.vias[0].position, ends[0],
        "the via joins the two tracks"
    );
    assert_legal(&design, &routes);
}

#[test]
fn keeps_inside_its_corridor() {
    let design = DesignBuilder::new(20.0, 12.0)
        .smd("A1", Some("A"), at(2.5, 8.5), 1.0, 0)
        .smd("A2", Some("A"), at(17.5, 8.5), 1.0, 0)
        .build();
    // Up from A1, along row 3, down to A2: a detour the straight line would not take.
    let detour = corridor(
        (1..=4)
            .flat_map(|c| (2..=9).map(move |r| (c, r)))
            .chain((1..=18).flat_map(|c| (2..=4).map(move |r| (c, r))))
            .chain((16..=18).flat_map(|c| (2..=9).map(move |r| (c, r)))),
    );
    let result = route_in(&design, &profile(), &[("A1", "A2")], Some(&detour));
    assert_eq!(result.unrouted, []);
    for point in result.routes.tracks.iter().flat_map(|t| &t.path) {
        assert!(
            detour.allows(ConnectionId(0), *point),
            "{point:?} leaves the corridor"
        );
    }
    assert_legal(&design, &result.routes);
}

#[test]
fn searches_the_whole_window_when_its_corridor_is_blocked() {
    let design = DesignBuilder::new(20.0, 12.0)
        .smd("A1", Some("A"), at(2.5, 6.5), 1.0, 0)
        .smd("A2", Some("A"), at(17.5, 6.5), 1.0, 0)
        .keepout(
            wall(at(9.5, 3.0), at(10.5, 12.0)),
            None,
            KeepoutScope::Everything,
        )
        .build();
    // Straight across, right through the wall; the way around its top end lies outside.
    let straight = corridor((1..=18).flat_map(|c| (5..=7).map(move |r| (c, r))));
    let result = route_in(&design, &profile(), &[("A1", "A2")], Some(&straight));
    assert_eq!(result.unrouted, []);
    assert_legal(&design, &result.routes);
}

#[test]
fn a_pad_with_no_way_out_is_named() {
    // A1 lies under a keepout: no track may leave it.
    let design = DesignBuilder::new(20.0, 10.0)
        .keepout(
            wall(at(1.0, 4.0), at(3.0, 6.0)),
            None,
            KeepoutScope::Everything,
        )
        .smd("A1", Some("A"), at(2.0, 5.0), 0.5, 0)
        .smd("A2", Some("A"), at(18.0, 5.0), 0.5, 0)
        .build();
    let result = route(&design, &[("A1", "A2")]);
    assert_eq!(result.unrouted, [connection(&design, "A1", "A2")]);
    assert_eq!(result.hemmed_in, [design.pad_named("A1")]);
}

#[test]
fn a_pad_over_the_board_edge_is_left_from_its_copper_on_the_board() {
    // A1's centre lies on the board's left edge, as an edge connector's might.
    let design = DesignBuilder::new(20.0, 10.0)
        .smd("A1", Some("A"), at(0.0, 5.0), 1.0, 0)
        .smd("A2", Some("A"), at(15.0, 5.0), 1.0, 0)
        .build();
    let result = route(&design, &[("A1", "A2")]);
    assert_eq!(result.unrouted, []);
    assert_legal(&design, &result.routes);
    let start = result.routes.tracks[0].path[0];
    assert!(
        start.x > 0 && start.x <= mm(0.5),
        "starts on A1's copper: {start:?}"
    );
}
