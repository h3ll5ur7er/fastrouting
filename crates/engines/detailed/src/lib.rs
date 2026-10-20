//! Detailed Routing Engine: the algorithm that turns connections into exact,
//! rule-correct copper (volatility V7 of docs/Architecture/Volatilities.md).
//!
//! For now an A* search on an octilinear grid across all routable layers, in
//! batches of connections far enough apart to be searched side by side, with
//! rip-up and reroute: a connection that finds its way blocked by routed
//! copper may take that copper's place, and the connections it displaces are
//! routed again later.

mod grid;
mod history;
mod path;
mod search;

use std::cmp::Reverse;
use std::collections::VecDeque;
use std::sync::atomic::{AtomicUsize, Ordering};
use traxide_contracts::geometry::Rect;
use traxide_contracts::{
    Connection, ConnectionId, ConstraintSet, Corridors, Design, Event, LayoutAccess, Obstruction,
    Plan, Profile, Routes, RoutingResult,
};
use traxide_events::{CancelToken, Publisher};

/// How many connections a batch looks through for each connection it may hold.
const SCANNED_PER_BATCH_SLOT: usize = 4;

pub struct Router<'a> {
    finder: Finder<'a>,
    /// One per thread: each search needs its own memory.
    scratches: Vec<search::Scratch>,
}

impl std::fmt::Debug for Router<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Router")
            .field("profile", &self.finder.profile.name)
            .field("threads", &self.scratches.len())
            .finish_non_exhaustive()
    }
}

impl<'a> Router<'a> {
    pub fn new(design: &'a Design, rules: &'a ConstraintSet, profile: &'a Profile) -> Self {
        Self {
            finder: Finder {
                design,
                rules,
                profile,
            },
            scratches: vec![search::Scratch::default()],
        }
    }

    /// Searches the connections of a batch on up to `threads` threads. The
    /// routes are the same on any number of them.
    pub fn with_threads(mut self, threads: usize) -> Self {
        self.scratches
            .resize_with(threads.max(1), search::Scratch::default);
        self
    }

    /// Routes the connections of `plan` in order, adding their copper to `layout`.
    /// Connections that cannot touch each other's copper are searched side by
    /// side in batches and committed in order (see `next_batch`).
    /// A connection is ripped up at most the profile's `max_rip_ups` times; those
    /// that find no way within that are reported as unrouted.
    pub fn route_plan(
        &mut self,
        plan: &Plan,
        corridors: Option<&Corridors>,
        layout: &mut (impl LayoutAccess + Sync),
        cancel: &CancelToken,
        events: &impl Publisher<Event>,
    ) -> RoutingResult {
        let total = plan.connections.len();
        let mut routed: Vec<Option<Routes>> = vec![None; total];
        let mut rip_ups = vec![0_u32; total];
        let mut queue: VecDeque<ConnectionId> = (0..total).map(ConnectionId::from_index).collect();
        let mut unrouted = Vec::new();
        let may_rip_up = self.finder.profile.max_rip_ups > 0;
        let mut history =
            history::History::new(self.finder.design, self.finder.profile.history_cost);
        while !queue.is_empty() {
            if cancel.is_cancelled() {
                unrouted.extend(queue.drain(..));
                break;
            }
            let batch = self.next_batch(&mut queue, plan, corridors);
            let found = self.find_all(
                &batch,
                plan,
                corridors,
                layout,
                may_rip_up.then_some(search::Negotiation {
                    rip_ups: &rip_ups,
                    history: &history,
                }),
            );
            for (id, found) in batch.iter().copied().zip(found) {
                let Some(found) = found else {
                    unrouted.push(id);
                    continue;
                };
                let displaced = routed_obstructions(layout, &found);
                if !displaced.is_empty() {
                    record_collisions(&mut history, self.finder.design, layout, &found);
                }
                debug_assert!(
                    displaced.iter().all(|d| !batch.contains(d)),
                    "connections of one batch keep out of each other's territory"
                );
                for displaced in displaced {
                    layout.remove_route(displaced);
                    routed[displaced.index()] = None;
                    rip_ups[displaced.index()] += 1;
                    queue.push_back(displaced);
                }
                layout.add_route(id, &found);
                routed[id.index()] = Some(found);
                let done = routed.iter().filter(|r| r.is_some()).count();
                events.publish(Event::ConnectionRouted { done, total });
            }
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

    /// Takes up to the profile's `batch_size` connections from the front of
    /// `queue`, in order, whose territories overlap neither each other nor that
    /// of any connection they skip: so no connection of a batch can meet the
    /// copper of another, nor overtake one it could meet. Batches depend on the
    /// queue alone, never on the number of threads.
    fn next_batch(
        &self,
        queue: &mut VecDeque<ConnectionId>,
        plan: &Plan,
        corridors: Option<&Corridors>,
    ) -> Vec<ConnectionId> {
        let batch_size = self.finder.profile.batch_size as usize;
        // Bounds the cost of looking for room on boards with many connections.
        let scan_depth = batch_size * SCANNED_PER_BATCH_SLOT;
        let mut claimed: Vec<Rect> = Vec::new();
        let mut batch = Vec::new();
        let mut position = 0;
        while let Some(&id) = queue.get(position) {
            if batch.len() == batch_size || claimed.len() == scan_depth {
                break;
            }
            let territory = self
                .finder
                .territory(id, plan.connections[id.index()], corridors);
            let is_free = claimed.iter().all(|c| !c.intersects(territory));
            claimed.push(territory);
            if is_free {
                batch.extend(queue.remove(position));
            } else {
                position += 1;
            }
        }
        batch
    }

    /// The route each connection of `batch` finds in `layout`, in batch order.
    fn find_all(
        &mut self,
        batch: &[ConnectionId],
        plan: &Plan,
        corridors: Option<&Corridors>,
        layout: &(impl LayoutAccess + Sync),
        negotiation: Option<search::Negotiation>,
    ) -> Vec<Option<Routes>> {
        let finder = self.finder;
        let find = |scratch: &mut search::Scratch, id: ConnectionId| {
            let connection = plan.connections[id.index()];
            finder.find(scratch, id, connection, corridors, layout, negotiation)
        };
        let workers = self.scratches.len().min(batch.len());
        if workers <= 1 {
            let scratch = &mut self.scratches[0];
            return batch.iter().map(|&id| find(scratch, id)).collect();
        }
        // The largest search runs on the first scratch, which batches of one
        // connection have grown already, so the other scratches only grow as
        // large as the smaller searches they get: filling fresh memory for
        // every thread cost more than the threads saved.
        let mut order: Vec<usize> = (0..batch.len()).collect();
        order.sort_by_key(|&item| {
            let id = batch[item];
            Reverse(
                finder
                    .territory(id, plan.connections[id.index()], corridors)
                    .area(),
            )
        });
        let next = AtomicUsize::new(1);
        let work = |scratch: &mut search::Scratch, mut position: usize| {
            let mut done = Vec::new();
            while let Some(&item) = order.get(position) {
                done.push((item, find(scratch, batch[item])));
                position = next.fetch_add(1, Ordering::Relaxed);
            }
            done
        };
        let (first, others) = self.scratches[..workers]
            .split_first_mut()
            .unwrap_or_else(|| unreachable!("at least two workers"));
        let mut found: Vec<Option<Routes>> = vec![None; batch.len()];
        std::thread::scope(|scope| {
            let helpers: Vec<_> = others
                .iter_mut()
                .map(|scratch| {
                    let (next, work) = (&next, &work);
                    scope.spawn(move || work(scratch, next.fetch_add(1, Ordering::Relaxed)))
                })
                .collect();
            let mine = work(first, 0);
            // A helper only panics on a bug in the search; pass it on.
            let theirs = helpers.into_iter().flat_map(|helper| {
                helper
                    .join()
                    .unwrap_or_else(|panic| std::panic::resume_unwind(panic))
            });
            for (item, routes) in mine.into_iter().chain(theirs) {
                found[item] = routes;
            }
        });
        found
    }
}

/// What a search needs besides its memory; shared by all threads.
#[derive(Clone, Copy)]
struct Finder<'a> {
    design: &'a Design,
    rules: &'a ConstraintSet,
    profile: &'a Profile,
}

impl Finder<'_> {
    /// Where the connection's copper may go, grown by the clearance other copper
    /// keeps from it: two connections with apart territories cannot meet.
    fn territory(
        &self,
        id: ConnectionId,
        connection: Connection,
        corridors: Option<&Corridors>,
    ) -> Rect {
        let anywhere = search::window(self.design, self.profile, connection, None);
        let window = match corridors.filter(|c| c.bounds(id).is_some()) {
            Some(corridors) => anywhere.union(search::window(
                self.design,
                self.profile,
                connection,
                Some((corridors, id)),
            )),
            None => anywhere,
        };
        let via_radius = self
            .rules
            .via_kinds(connection.net)
            .iter()
            .flat_map(|kind| &self.design.via_kinds[kind.index()].copper)
            .map(|copper| {
                let bounds = copper.shape.bounds();
                (bounds.max.x - bounds.min.x).max(bounds.max.y - bounds.min.y) / 2 + 1
            })
            .max()
            .unwrap_or(0);
        let copper_reach = via_radius.max(self.rules.track_width(connection.net) / 2 + 1);
        window.inflated(copper_reach + self.rules.largest_clearance())
    }

    /// A route inside the connection's corridor; when the corridor turns out to
    /// be blocked, as global routing only estimates room, anywhere in reach.
    fn find(
        &self,
        scratch: &mut search::Scratch,
        id: ConnectionId,
        connection: Connection,
        corridors: Option<&Corridors>,
        layout: &impl LayoutAccess,
        negotiation: Option<search::Negotiation>,
    ) -> Option<Routes> {
        let in_corridor = corridors
            .filter(|c| c.bounds(id).is_some())
            .and_then(|c| self.search(scratch, connection, layout, negotiation, Some((c, id))));
        in_corridor.or_else(|| self.search(scratch, connection, layout, negotiation, None))
    }

    fn search(
        &self,
        scratch: &mut search::Scratch,
        connection: Connection,
        layout: &impl LayoutAccess,
        negotiation: Option<search::Negotiation>,
        corridor: Option<(&Corridors, ConnectionId)>,
    ) -> Option<Routes> {
        let search = search::Search::new(
            self.design,
            self.rules,
            self.profile,
            connection,
            layout,
            negotiation,
            corridor,
        );
        let steps = search.run(scratch)?;
        Some(path::to_routes(
            &steps,
            connection.net,
            self.rules.track_width(connection.net),
        ))
    }
}

/// Makes the places where `routes` overlap routed copper dearer for later searches.
fn record_collisions(
    history: &mut history::History,
    design: &Design,
    layout: &impl LayoutAccess,
    routes: &Routes,
) {
    for track in &routes.tracks {
        for pair in track.path.windows(2) {
            let segment = traxide_contracts::geometry::Segment::new(pair[0], pair[1]);
            if let Obstruction::Routed(_) =
                layout.track_obstruction(track.net, track.layer, segment, track.width)
            {
                history.record_track(track.layer, segment, track.width);
            }
        }
    }
    for via in &routes.vias {
        if let Obstruction::Routed(_) = layout.via_obstruction(via) {
            let copper = &design.via_kind(via.kind).copper;
            let radius = copper
                .iter()
                .map(|c| {
                    let bounds = c.shape.bounds();
                    (bounds.max.x - bounds.min.x) / 2
                })
                .max()
                .unwrap_or(0);
            history.record_via(via.position, radius, copper.iter().map(|c| c.layer));
        }
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
