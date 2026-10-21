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

use search::NoRoute;
use std::cmp::Reverse;
use std::collections::VecDeque;
use std::sync::atomic::{AtomicUsize, Ordering};
use traxide_contracts::geometry::{Point, Rect, Segment};
use traxide_contracts::{
    Connection, ConnectionId, ConstraintSet, Corridors, Design, Event, LayoutAccess, Obstruction,
    Plan, Profile, Routes, RoutingResult,
};
use traxide_events::{CancelToken, Publisher};

/// How many connections a batch looks through for each connection it may hold.
const SCANNED_PER_BATCH_SLOT: usize = 4;

/// Measured on the corpus: one round left Issue034 3 connections short of
/// complete. Two cost the Mars-64 boards 6 connections until fine-pitch pins
/// escaped before routing; since, two route as many boards completely as
/// four and 4 connections more in all, in a tenth less time, half on the
/// churning boards (TRX-56).
const STALLED_ROUNDS: usize = 2;

/// How much wider each window of a search that found no way grows.
const WIDER_WINDOW: i32 = 4;

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
    /// that find no way within that are reported as unrouted. Once
    /// STALLED_ROUNDS rounds of rip-ups, as many as there are connections each,
    /// have routed no more connections than before, negotiation is going round
    /// in circles: the connections left search without ripping up any other.
    pub fn route_plan(
        &mut self,
        plan: &Plan,
        corridors: Option<&Corridors>,
        layout: &mut (impl LayoutAccess + Sync),
        cancel: &CancelToken,
        events: &impl Publisher<Event>,
    ) -> RoutingResult {
        let total = plan.connections.len();
        let mut state = Negotiated {
            routed: vec![None; total],
            rip_ups: vec![0; total],
            queue: (0..total).map(ConnectionId::from_index).collect(),
            history: history::History::new(self.finder.design, self.finder.profile.history_cost),
            may_rip_up: self.finder.profile.max_rip_ups > 0,
            most_routed: 0,
            rip_ups_since_most: 0,
        };
        let mut unrouted = Vec::new();
        let (mut hemmed_in, mut exits_taken, mut cut_off) = (Vec::new(), Vec::new(), Vec::new());
        while !state.queue.is_empty() {
            if cancel.is_cancelled() {
                unrouted.extend(state.queue.drain(..));
                break;
            }
            let batch = self.next_batch(&mut state.queue, plan, corridors);
            let found = self.find_all(&batch, plan, corridors, layout, state.negotiation());
            let mut no_way = Vec::new();
            for (id, found) in batch.iter().copied().zip(found) {
                match found {
                    Ok(found) => {
                        state.commit(self.finder.design, layout, id, found, &batch, events)
                    }
                    Err(NoRoute::NotFound) => no_way.push(id),
                    Err(NoRoute::Blocked {
                        hemmed_in: fixed,
                        exits_taken: taken,
                    }) => {
                        hemmed_in.extend(fixed);
                        exits_taken.extend(taken);
                        unrouted.push(id);
                    }
                }
            }
            // After the batch, one at a time, so that a wider window cannot meet
            // the copper of a connection searched beside it.
            for id in no_way {
                let connection = plan.connections[id.index()];
                match self.find_wider(connection, layout, state.negotiation()) {
                    Some(found) => state.commit(self.finder.design, layout, id, found, &[], events),
                    None => {
                        if self.is_cut_off(connection, layout) {
                            cut_off.push(connection);
                        }
                        unrouted.push(id);
                    }
                }
            }
        }
        let Negotiated { routed, .. } = state;
        unrouted.sort_unstable();
        for pads in [&mut hemmed_in, &mut exits_taken] {
            pads.sort_unstable();
            pads.dedup();
        }
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
            hemmed_in,
            exits_taken,
            cut_off,
        }
    }

    /// A search in windows grown WIDER_WINDOW times wider each, alone, until one
    /// finds a way or the last covered the whole board: the way round a row of
    /// headers or the board's own wiring is often longer than the margin.
    fn find_wider(
        &mut self,
        connection: Connection,
        layout: &impl LayoutAccess,
        negotiation: Option<search::Negotiation>,
    ) -> Option<Routes> {
        let board = self.finder.design.outline.bounds();
        let mut wider = self.finder.profile.clone();
        while !search::window(self.finder.design, wider.detour_margin, connection, None)
            .contains_rect(board)
        {
            wider.detour_margin = wider.detour_margin.saturating_mul(WIDER_WINDOW);
            let finder = Finder {
                profile: &wider,
                ..self.finder
            };
            if let Ok(found) = finder.search(
                &mut self.scratches[0],
                connection,
                layout,
                negotiation,
                None,
            ) {
                return Some(found);
            }
        }
        None
    }

    /// Whether the board's own copper, keepouts and edge leave no way at all
    /// between the pads of `connection`, whatever was routed.
    fn is_cut_off(&mut self, connection: Connection, layout: &impl LayoutAccess) -> bool {
        let board = self.finder.design.outline.bounds();
        let whole_board = Profile {
            detour_margin: (board.max.x - board.min.x).max(board.max.y - board.min.y),
            ..self.finder.profile.clone()
        };
        search::Search::new(
            self.finder.design,
            self.finder.rules,
            &whole_board,
            connection,
            layout,
            None,
            None,
        )
        .through_routed_copper()
        .run(&mut self.scratches[0])
        .is_err()
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
    ) -> Vec<Result<Routes, NoRoute>> {
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
        let mut found: Vec<Result<Routes, NoRoute>> =
            batch.iter().map(|_| Err(NoRoute::NotFound)).collect();
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
        let anywhere = search::window(self.design, self.profile.detour_margin, connection, None);
        let window = match corridors.filter(|c| c.bounds(id).is_some()) {
            Some(corridors) => anywhere.union(search::window(
                self.design,
                self.profile.detour_margin,
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
    ) -> Result<Routes, NoRoute> {
        let in_corridor = corridors.filter(|c| c.bounds(id).is_some()).and_then(|c| {
            self.search(scratch, connection, layout, negotiation, Some((c, id)))
                .ok()
        });
        in_corridor.map_or_else(
            || self.search(scratch, connection, layout, negotiation, None),
            Ok,
        )
    }

    fn search(
        &self,
        scratch: &mut search::Scratch,
        connection: Connection,
        layout: &impl LayoutAccess,
        negotiation: Option<search::Negotiation>,
        corridor: Option<(&Corridors, ConnectionId)>,
    ) -> Result<Routes, NoRoute> {
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
        Ok(path::to_routes(
            &steps,
            connection.net,
            self.rules.track_width(connection.net),
        ))
    }
}

/// Where routing stands: the routes so far and what negotiation has learnt.
struct Negotiated {
    routed: Vec<Option<Routes>>,
    rip_ups: Vec<u32>,
    queue: VecDeque<ConnectionId>,
    history: history::History,
    may_rip_up: bool,
    most_routed: usize,
    rip_ups_since_most: usize,
}

impl Negotiated {
    fn negotiation(&self) -> Option<search::Negotiation<'_>> {
        self.may_rip_up.then_some(search::Negotiation {
            rip_ups: &self.rip_ups,
            history: &self.history,
        })
    }

    /// Adds `found` for `id`, ripping up the routed connections it crosses,
    /// which never include another of `batch`.
    fn commit(
        &mut self,
        design: &Design,
        layout: &mut impl LayoutAccess,
        id: ConnectionId,
        found: Routes,
        batch: &[ConnectionId],
        events: &impl Publisher<Event>,
    ) {
        let displaced = routed_obstructions(layout, &found);
        if !displaced.is_empty() {
            record_collisions(&mut self.history, design, layout, &found);
        }
        debug_assert!(
            displaced.iter().all(|d| !batch.contains(d)),
            "connections of one batch keep out of each other's territory"
        );
        for displaced in displaced {
            if let Some(theirs) = &self.routed[displaced.index()] {
                events.publish(Event::RippedUp {
                    connection: displaced,
                    by: id,
                    at: where_they_meet(&found, theirs),
                });
            }
            layout.remove_route(displaced);
            self.routed[displaced.index()] = None;
            self.rip_ups[displaced.index()] += 1;
            self.rip_ups_since_most += 1;
            self.queue.push_back(displaced);
        }
        layout.add_route(id, &found);
        self.routed[id.index()] = Some(found);
        let total = self.routed.len();
        let done = self.routed.iter().filter(|r| r.is_some()).count();
        if done > self.most_routed {
            (self.most_routed, self.rip_ups_since_most) = (done, 0);
        } else if self.rip_ups_since_most >= STALLED_ROUNDS * total {
            self.may_rip_up = false;
        }
        events.publish(Event::ConnectionRouted { done, total });
    }
}

/// About where the copper of `ours` comes closest to that of `theirs`: the
/// middle between the nearest of each one's corners and the other's tracks.
fn where_they_meet(ours: &Routes, theirs: &Routes) -> Point {
    let pieces = |routes: &Routes| -> Vec<Segment> {
        let tracks = routes
            .tracks
            .iter()
            .flat_map(|t| t.path.windows(2).map(|pair| Segment::new(pair[0], pair[1])));
        let vias = routes
            .vias
            .iter()
            .map(|v| Segment::new(v.position, v.position));
        tracks.chain(vias).collect()
    };
    let (ours, theirs) = (pieces(ours), pieces(theirs));
    let nearest = |pieces: &[Segment], others: &[Segment]| {
        pieces
            .iter()
            .flat_map(|piece| [piece.a, piece.b])
            .flat_map(|corner| {
                others
                    .iter()
                    .map(move |other| (corner, other.closest_point(corner)))
            })
            .min_by_key(|&(corner, on_other)| corner.distance(on_other))
    };
    [nearest(&ours, &theirs), nearest(&theirs, &ours)]
        .into_iter()
        .flatten()
        .min_by_key(|&(corner, on_other)| corner.distance(on_other))
        .map_or(Point::default(), |(corner, on_other)| {
            Point::new(
                corner.x + (on_other.x - corner.x) / 2,
                corner.y + (on_other.y - corner.y) / 2,
            )
        })
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
