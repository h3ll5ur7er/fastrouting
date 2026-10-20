use crate::grid::{DIRECTIONS, Grid, Node};
use crate::path::{self, Step};
use crate::scratch::Stamped;
use std::cmp::Reverse;
use std::collections::BinaryHeap;
use traxide_contracts::geometry::{Point, Rect, Segment};
use traxide_contracts::{
    Connection, ConstraintSet, Design, LayerId, LayoutAccess, NetId, Obstruction, Pad, Profile,
    Via, ViaKindId,
};

/// No heading yet: just left a pad or a via.
const UNSET: u8 = 8;
const HEADINGS: usize = 9;
/// The slot of the target pad's centre, reached from any node near it.
const TARGET: u32 = u32::MAX;
/// The cost of an edge or via that may not be taken at all.
const BLOCKED: i64 = -1;

/// Search memory kept between connections, so each search costs no allocation.
#[derive(Debug, Default)]
pub(crate) struct Scratch {
    best: Stamped<i64>,
    /// The slot a state was reached from and the via taken, packed (see `Came`).
    came: Stamped<u64>,
    /// What crossing obstructions costs on top of an edge's length, or `BLOCKED`.
    edge_extra: Stamped<i64>,
    /// The same for a via at a node.
    via_extra: Stamped<i64>,
    /// What the last stretch to the target's centre costs, by layer and node.
    finish: Stamped<i64>,
}

/// A state of the search: a node, a layer and the heading it was entered with.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct State {
    node: Node,
    layer: u32,
    heading: u8,
}

#[derive(Debug, Clone, Copy)]
struct Came {
    from: Option<u32>,
    via: Option<ViaKindId>,
}

impl Came {
    fn pack(self) -> u64 {
        let from = self.from.map_or(0, |slot| u64::from(slot) + 1);
        let via = self.via.map_or(0, |kind| u64::from(kind.0) + 1);
        from | via << 32
    }

    fn unpack(packed: u64) -> Self {
        let from = (packed & 0xFFFF_FFFF) as u32;
        let via = (packed >> 32) as u32;
        Self {
            from: from.checked_sub(1),
            via: via.checked_sub(1).map(ViaKindId),
        }
    }
}

pub(crate) struct Search<'a, L> {
    design: &'a Design,
    profile: &'a Profile,
    layout: &'a L,
    net: NetId,
    width: i32,
    via_kinds: &'a [ViaKindId],
    from: &'a Pad,
    to: &'a Pad,
    grid: Grid,
    diagonal: i64,
    /// How often each connection has been ripped up; `None` when routed copper
    /// may not be crossed at all.
    rip_ups: Option<&'a [u32]>,
    /// The profile's heuristic weight in thousandths, so ranking stays in integers.
    weight_per_mille: i64,
}

impl<'a, L: LayoutAccess> Search<'a, L> {
    pub(crate) fn new(
        design: &'a Design,
        rules: &'a ConstraintSet,
        profile: &'a Profile,
        connection: Connection,
        layout: &'a L,
        rip_ups: Option<&'a [u32]>,
    ) -> Self {
        let (from, to) = (design.pad(connection.from), design.pad(connection.to));
        let window = Rect {
            min: from.position,
            max: from.position,
        }
        .including(to.position)
        .inflated(profile.detour_margin);
        let pitch = i64::from(profile.grid_pitch);
        Self {
            design,
            profile,
            layout,
            net: connection.net,
            width: rules.track_width(connection.net),
            via_kinds: rules.via_kinds(connection.net),
            from,
            to,
            grid: Grid::covering(window, profile.grid_pitch),
            diagonal: Point::new(0, 0)
                .distance(Point::new(profile.grid_pitch, profile.grid_pitch))
                .max(pitch),
            rip_ups,
            weight_per_mille: (profile.heuristic_weight * 1000.0).round() as i64,
        }
    }

    pub(crate) fn run(self, scratch: &mut Scratch) -> Option<Vec<Step>> {
        if let Some(direct) = self.direct_line() {
            return Some(direct);
        }
        let (from_access, to_access) = (self.access(self.from), self.access(self.to));
        // A search floods everything cheaper than the way into its target. Into a
        // pad hemmed in by its neighbours, such as a pin of a fine-pitch chip,
        // that is most of the board; out of it, only the pocket around it. So
        // the search starts at the end with fewer ways in or out.
        if to_access.len() < from_access.len() {
            let backwards = Search {
                from: self.to,
                to: self.from,
                ..self
            };
            return backwards
                .search(to_access, from_access, scratch)
                .map(path::reversed);
        }
        self.search(from_access, to_access, scratch)
    }

    fn search(
        self,
        start: Vec<(State, i64)>,
        target: Vec<(State, i64)>,
        scratch: &mut Scratch,
    ) -> Option<Vec<Step>> {
        let layers = self.design.layers.len();
        let nodes = self.grid.node_count();
        scratch.best.reset(nodes * layers * HEADINGS);
        scratch.came.reset(nodes * layers * HEADINGS);
        scratch.edge_extra.reset(nodes * layers * DIRECTIONS.len());
        scratch.via_extra.reset(nodes * self.via_kinds.len());
        scratch.finish.reset(nodes * layers);
        for (state, cost) in target {
            scratch.finish.set(self.layer_node(state), cost);
        }

        let mut best_target = i64::MAX;
        let mut target_came = None;
        let mut open = BinaryHeap::new();
        for (state, cost) in start {
            let slot = self.slot(state);
            if cost < scratch.best.get(slot as usize).unwrap_or(i64::MAX) {
                scratch.best.set(slot as usize, cost);
                scratch.came.set(
                    slot as usize,
                    Came {
                        from: None,
                        via: None,
                    }
                    .pack(),
                );
                open.push(Reverse((cost + self.ranked_estimate(state), cost, slot)));
            }
        }
        while let Some(Reverse((_, cost, slot))) = open.pop() {
            if slot == TARGET {
                return Some(self.steps(target_came?, scratch));
            }
            if cost > scratch.best.get(slot as usize).unwrap_or(i64::MAX) {
                continue;
            }
            let state = self.state(slot);
            let mut relax = |next: Option<State>,
                             step_cost: i64,
                             via: Option<ViaKindId>,
                             scratch: &mut Scratch| {
                let total = cost + step_cost;
                let came = Came {
                    from: Some(slot),
                    via,
                };
                match next {
                    Some(next) => {
                        let next_slot = self.slot(next);
                        if total < scratch.best.get(next_slot as usize).unwrap_or(i64::MAX) {
                            scratch.best.set(next_slot as usize, total);
                            scratch.came.set(next_slot as usize, came.pack());
                            open.push(Reverse((
                                total + self.ranked_estimate(next),
                                total,
                                next_slot,
                            )));
                        }
                    }
                    None if total < best_target => {
                        best_target = total;
                        target_came = Some(slot);
                        open.push(Reverse((total, total, TARGET)));
                    }
                    None => {}
                }
            };
            if let Some(finish) = scratch.finish.get(self.layer_node(state)) {
                relax(None, finish, None, scratch);
            }
            for (direction, (dx, dy)) in DIRECTIONS.iter().enumerate() {
                let turn = turn(state.heading, direction);
                let next = (state.node.0 + dx, state.node.1 + dy);
                // Turns sharper than 90° leave acute angles that trap etchant.
                if turn > 2 || !self.grid.contains(next) {
                    continue;
                }
                let edge = (self.layer_node(state) * DIRECTIONS.len()) + direction;
                let extra = scratch.edge_extra.get_or_insert_with(edge, || {
                    let segment = Segment::new(self.grid.point(state.node), self.grid.point(next));
                    self.track_extra(LayerId(state.layer), segment)
                        .unwrap_or(BLOCKED)
                });
                if extra != BLOCKED {
                    let length = if dx * dy == 0 {
                        i64::from(self.profile.grid_pitch)
                    } else {
                        self.diagonal
                    };
                    let next = State {
                        node: next,
                        layer: state.layer,
                        heading: direction as u8,
                    };
                    let cost = length + i64::from(turn) * self.profile.bend_cost + extra;
                    relax(Some(next), cost, None, scratch);
                }
            }
            for (position, &kind) in self.via_kinds.iter().enumerate() {
                let spans = self.design.via_kind(kind);
                let others = self.design.layer_ids().filter(|&l| {
                    l.0 != state.layer
                        && self.design.layers[l.index()].routable
                        && spans.connects(LayerId(state.layer), l)
                });
                for layer in others.collect::<Vec<_>>() {
                    let check = self.grid.index(state.node) * self.via_kinds.len() + position;
                    let extra = scratch.via_extra.get_or_insert_with(check, || {
                        let via = Via {
                            net: self.net,
                            kind,
                            position: self.grid.point(state.node),
                        };
                        self.via_extra(&via).unwrap_or(BLOCKED)
                    });
                    if extra != BLOCKED {
                        let next = State {
                            node: state.node,
                            layer: layer.0,
                            heading: UNSET,
                        };
                        relax(
                            Some(next),
                            self.profile.via_cost + extra,
                            Some(kind),
                            scratch,
                        );
                    }
                }
            }
        }
        None
    }

    /// What a track segment costs beyond its length: nothing when it is clear,
    /// the price of ripping up what it crosses when that is allowed, else `None`.
    fn track_extra(&self, layer: LayerId, segment: Segment) -> Option<i64> {
        match self.rip_ups {
            None => self
                .layout
                .is_track_clear(self.net, layer, segment, self.width)
                .then_some(0),
            Some(_) => self.rip_up_price(
                self.layout
                    .track_obstruction(self.net, layer, segment, self.width),
            ),
        }
    }

    fn via_extra(&self, via: &Via) -> Option<i64> {
        match self.rip_ups {
            None => self.layout.is_via_clear(via).then_some(0),
            Some(_) => self.rip_up_price(self.layout.via_obstruction(via)),
        }
    }

    /// Each crossed connection costs more the more often it was ripped up before,
    /// so that connections settle instead of taking turns (negotiated congestion's
    /// history cost); one ripped up too often may not be crossed again.
    fn rip_up_price(&self, obstruction: Obstruction) -> Option<i64> {
        match obstruction {
            Obstruction::None => Some(0),
            Obstruction::Fixed => None,
            Obstruction::Routed(connections) => {
                let rip_ups = self.rip_ups?;
                connections
                    .iter()
                    .map(|c| {
                        let times = rip_ups[c.index()];
                        (times < self.profile.max_rip_ups)
                            .then(|| self.profile.rip_up_cost * i64::from(times + 1))
                    })
                    .sum()
            }
        }
    }

    fn layer_node(&self, state: State) -> usize {
        state.layer as usize * self.grid.node_count() + self.grid.index(state.node)
    }

    fn slot(&self, state: State) -> u32 {
        u32::try_from(self.layer_node(state) * HEADINGS + usize::from(state.heading))
            .expect("search windows below 2³² states")
    }

    fn state(&self, slot: u32) -> State {
        let slot = slot as usize;
        let (layer_node, heading) = (slot / HEADINGS, slot % HEADINGS);
        let nodes = self.grid.node_count();
        State {
            node: self.grid.node(layer_node % nodes),
            layer: (layer_node / nodes) as u32,
            heading: heading as u8,
        }
    }

    /// The pad centres joined without a search, when they see each other on a
    /// layer both share: straight if they line up at a multiple of 45°, else one
    /// straight and one diagonal segment, as octilinear routing requires.
    fn direct_line(&self) -> Option<Vec<Step>> {
        let candidates = octilinear_paths(self.from.position, self.to.position);
        let fits = |layer: LayerId, path: &[Point]| {
            path.windows(2).all(|pair| {
                let segment = Segment::new(pair[0], pair[1]);
                self.layout
                    .is_track_clear(self.net, layer, segment, self.width)
            })
        };
        self.routable_layers(self.from)
            .filter(|&layer| self.to.is_on(layer))
            .find_map(|layer| {
                let path = candidates.iter().find(|path| fits(layer, path))?;
                Some(
                    path.iter()
                        .map(|&at| Step {
                            at,
                            layer,
                            via: None,
                        })
                        .collect(),
                )
            })
    }

    fn routable_layers<'p>(&'p self, pad: &'p Pad) -> impl Iterator<Item = LayerId> + 'p {
        pad.copper
            .iter()
            .map(|c| c.layer)
            .filter(|l| self.design.layers[l.index()].routable)
    }

    /// The grid states a track from the pad's centre can reach in a straight line,
    /// and what that track costs.
    fn access(&self, pad: &Pad) -> Vec<(State, i64)> {
        let mut states = Vec::new();
        for copper in pad
            .copper
            .iter()
            .filter(|c| self.design.layers[c.layer.index()].routable)
        {
            for node in self.grid.nodes_near(copper.shape.bounds(), pad.position) {
                let point = self.grid.point(node);
                let segment = Segment::new(pad.position, point);
                if let Some(extra) = self.track_extra(copper.layer, segment) {
                    let state = State {
                        node,
                        layer: copper.layer.0,
                        heading: UNSET,
                    };
                    states.push((state, pad.position.distance(point) + extra));
                }
            }
        }
        states
    }

    /// What is left to pay, as if nothing were in the way: the octilinear distance
    /// to the target and a via if the target has no copper on this layer. Paths
    /// on the grid are octilinear, so this is far tighter than the straight
    /// distance; only the short any-angle stub into the pad can cost less.
    fn estimate(&self, state: State) -> i64 {
        let here = self.grid.point(state.node);
        let dx = i64::from(here.x.abs_diff(self.to.position.x));
        let dy = i64::from(here.y.abs_diff(self.to.position.y));
        let (long, short) = (dx.max(dy), dx.min(dy));
        let octilinear = long - short + short * self.diagonal / i64::from(self.profile.grid_pitch);
        let via = if self.to.is_on(LayerId(state.layer)) {
            0
        } else {
            self.profile.via_cost
        };
        octilinear + via
    }

    /// The estimate as the search ranks states: weighted by the profile, so the
    /// search heads for the target more boldly than the cheapest path requires.
    fn ranked_estimate(&self, state: State) -> i64 {
        self.estimate(state) * self.weight_per_mille / 1000
    }

    fn steps(&self, last: u32, scratch: &Scratch) -> Vec<Step> {
        let last_layer = LayerId(self.state(last).layer);
        let mut reversed = vec![Step {
            at: self.to.position,
            layer: last_layer,
            via: None,
        }];
        let mut current = Some(last);
        while let Some(slot) = current {
            let state = self.state(slot);
            let came = Came::unpack(
                scratch
                    .came
                    .get(slot as usize)
                    .expect("every reached state knows its origin"),
            );
            reversed.push(Step {
                at: self.grid.point(state.node),
                layer: LayerId(state.layer),
                via: came.via,
            });
            current = came.from;
        }
        let first_layer = reversed.last().map_or(LayerId(0), |s| s.layer);
        reversed.push(Step {
            at: self.from.position,
            layer: first_layer,
            via: None,
        });
        reversed.reverse();
        reversed
    }
}

/// The octilinear ways from `from` to `to` with at most one bend: diagonal
/// first, then straight first.
fn octilinear_paths(from: Point, to: Point) -> Vec<Vec<Point>> {
    let (dx, dy) = (to.x - from.x, to.y - from.y);
    if dx == 0 || dy == 0 || dx.abs() == dy.abs() {
        return vec![vec![from, to]];
    }
    let diagonal = dx.abs().min(dy.abs());
    let step = Point::new(dx.signum() * diagonal, dy.signum() * diagonal);
    vec![vec![from, from + step, to], vec![from, to - step, to]]
}

/// How many 45° steps lie between a heading and the next direction.
fn turn(heading: u8, direction: usize) -> u8 {
    if heading == UNSET {
        return 0;
    }
    let difference = (i16::from(heading) - direction as i16).rem_euclid(8) as u8;
    difference.min(8 - difference)
}
