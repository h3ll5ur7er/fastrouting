//! Detailed Routing Engine: the algorithm that turns connections into exact,
//! rule-correct copper (volatility V7 of docs/Architecture/Volatilities.md).
//!
//! For now an A* search on an octilinear grid across all routable layers, one
//! connection at a time, with rip-up and reroute: a connection that finds its
//! way blocked by routed copper may take that copper's place, and the
//! connections it displaces are routed again later.

mod grid;
mod path;
mod scratch;
mod search;

use std::collections::VecDeque;
use traxide_contracts::{
    Connection, ConnectionId, ConstraintSet, Design, Event, LayoutAccess, Obstruction, Plan,
    Profile, Routes, RoutingResult,
};
use traxide_events::{CancelToken, Publisher};

pub struct Router<'a> {
    design: &'a Design,
    rules: &'a ConstraintSet,
    profile: &'a Profile,
    scratch: search::Scratch,
}

impl std::fmt::Debug for Router<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Router")
            .field("profile", &self.profile.name)
            .finish_non_exhaustive()
    }
}

impl<'a> Router<'a> {
    pub fn new(design: &'a Design, rules: &'a ConstraintSet, profile: &'a Profile) -> Self {
        Self {
            design,
            rules,
            profile,
            scratch: search::Scratch::default(),
        }
    }

    /// Routes the connections of `plan` in order, adding their copper to `layout`.
    /// A connection is ripped up at most the profile's `max_rip_ups` times; those
    /// that find no way within that are reported as unrouted.
    pub fn route_plan(
        &mut self,
        plan: &Plan,
        layout: &mut impl LayoutAccess,
        cancel: &CancelToken,
        events: &impl Publisher<Event>,
    ) -> RoutingResult {
        let total = plan.connections.len();
        let mut routed: Vec<Option<Routes>> = vec![None; total];
        let mut rip_ups = vec![0_u32; total];
        let mut queue: VecDeque<ConnectionId> = (0..total).map(ConnectionId::from_index).collect();
        let mut unrouted = Vec::new();
        let may_rip_up = self.profile.max_rip_ups > 0;
        while let Some(id) = queue.pop_front() {
            if cancel.is_cancelled() {
                unrouted.push(id);
                unrouted.extend(queue.drain(..));
                break;
            }
            let connection = plan.connections[id.index()];
            match self.find(connection, layout, may_rip_up.then_some(&rip_ups[..])) {
                Some(found) => {
                    for displaced in routed_obstructions(layout, &found) {
                        layout.remove_route(displaced);
                        routed[displaced.index()] = None;
                        rip_ups[displaced.index()] += 1;
                        queue.push_back(displaced);
                    }
                    layout.add_route(id, &found);
                    routed[id.index()] = Some(found);
                }
                None => unrouted.push(id),
            }
            let done = routed.iter().filter(|r| r.is_some()).count();
            events.publish(Event::ConnectionRouted { done, total });
        }
        unrouted.sort_unstable();
        let mut routes = Routes::default();
        for found in routed.into_iter().flatten() {
            routes.extend(found);
        }
        RoutingResult {
            routes,
            unrouted: unrouted
                .into_iter()
                .map(|id| plan.connections[id.index()])
                .collect(),
        }
    }

    fn find(
        &mut self,
        connection: Connection,
        layout: &impl LayoutAccess,
        rip_ups: Option<&[u32]>,
    ) -> Option<Routes> {
        let search = search::Search::new(
            self.design,
            self.rules,
            self.profile,
            connection,
            layout,
            rip_ups,
        );
        let steps = search.run(&mut self.scratch)?;
        Some(path::to_routes(
            &steps,
            connection.net,
            self.rules.track_width(connection.net),
        ))
    }
}

/// The routed connections that `routes` would overlap, in id order.
fn routed_obstructions(layout: &impl LayoutAccess, routes: &Routes) -> Vec<ConnectionId> {
    let tracks = routes.tracks.iter().flat_map(|track| {
        track.path.windows(2).map(move |pair| {
            let segment = traxide_contracts::geometry::Segment::new(pair[0], pair[1]);
            layout.track_obstruction(track.net, track.layer, segment, track.width)
        })
    });
    let vias = routes.vias.iter().map(|via| layout.via_obstruction(via));
    let mut displaced: Vec<ConnectionId> = tracks
        .chain(vias)
        .flat_map(|obstruction| match obstruction {
            Obstruction::Routed(connections) => connections,
            // The search never crosses fixed obstructions, so none can appear here.
            Obstruction::None | Obstruction::Fixed => Vec::new(),
        })
        .collect();
    displaced.sort_unstable();
    displaced.dedup();
    displaced
}
