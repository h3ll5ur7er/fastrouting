use crate::grid::{DIRECTIONS, Grid, Node};
use crate::path::Step;
use std::cmp::Reverse;
use std::collections::{BinaryHeap, HashMap};
use traxide_contracts::geometry::{Point, Rect, Segment};
use traxide_contracts::{
    Connection, ConstraintSet, Design, LayerId, LayoutAccess, NetId, Pad, Profile, Via, ViaKindId,
};

/// No heading yet: just left a pad or a via.
const UNSET: u8 = 8;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
struct State {
    node: Node,
    layer: u32,
    heading: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
enum Vertex {
    Grid(State),
    /// The centre of the target pad, reached from any node near it.
    Target,
}

#[derive(Debug, Clone, Copy)]
struct Came {
    from: Option<State>,
    via: Option<ViaKindId>,
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
}

impl<'a, L: LayoutAccess> Search<'a, L> {
    pub(crate) fn new(
        design: &'a Design,
        rules: &'a ConstraintSet,
        profile: &'a Profile,
        connection: Connection,
        layout: &'a L,
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
        }
    }

    pub(crate) fn run(self) -> Option<Vec<Step>> {
        if let Some(direct) = self.direct_line() {
            return Some(direct);
        }
        let target_access = self.access(self.to);
        let mut best: HashMap<Vertex, i64> = HashMap::new();
        let mut came: HashMap<State, Came> = HashMap::new();
        let mut target_came: Option<State> = None;
        let mut open = BinaryHeap::new();
        for (state, cost) in self.access(self.from) {
            if cost < *best.get(&Vertex::Grid(state)).unwrap_or(&i64::MAX) {
                best.insert(Vertex::Grid(state), cost);
                came.insert(
                    state,
                    Came {
                        from: None,
                        via: None,
                    },
                );
                open.push(Reverse((
                    cost + self.estimate(state),
                    cost,
                    Vertex::Grid(state),
                )));
            }
        }
        let mut edge_clear: HashMap<(Node, u32, usize), bool> = HashMap::new();
        let mut via_clear: HashMap<(Node, ViaKindId), bool> = HashMap::new();
        while let Some(Reverse((_, cost, vertex))) = open.pop() {
            if cost > *best.get(&vertex).unwrap_or(&i64::MAX) {
                continue;
            }
            let Vertex::Grid(state) = vertex else {
                return Some(self.steps(target_came?, &came));
            };
            let mut relax = |next: Vertex,
                             step_cost: i64,
                             origin: Came,
                             open: &mut BinaryHeap<_>| {
                let total = cost + step_cost;
                if total < *best.get(&next).unwrap_or(&i64::MAX) {
                    best.insert(next, total);
                    match next {
                        Vertex::Grid(next_state) => {
                            came.insert(next_state, origin);
                            open.push(Reverse((total + self.estimate(next_state), total, next)));
                        }
                        Vertex::Target => {
                            target_came = origin.from;
                            open.push(Reverse((total, total, next)));
                        }
                    }
                }
            };
            if let Some(&(_, finish)) = target_access
                .iter()
                .find(|(s, _)| s.node == state.node && s.layer == state.layer)
            {
                relax(
                    Vertex::Target,
                    finish,
                    Came {
                        from: Some(state),
                        via: None,
                    },
                    &mut open,
                );
            }
            for (index, (dx, dy)) in DIRECTIONS.iter().enumerate() {
                let turn = turn(state.heading, index);
                if turn > 2 {
                    continue;
                }
                let next = (state.node.0 + dx, state.node.1 + dy);
                if !self.grid.contains(next) {
                    continue;
                }
                let clear = *edge_clear
                    .entry((state.node, state.layer, index))
                    .or_insert_with(|| {
                        let segment =
                            Segment::new(self.grid.point(state.node), self.grid.point(next));
                        self.layout.is_track_clear(
                            self.net,
                            LayerId(state.layer),
                            segment,
                            self.width,
                        )
                    });
                if clear {
                    let length = if dx * dy == 0 {
                        i64::from(self.profile.grid_pitch)
                    } else {
                        self.diagonal
                    };
                    let step_cost = length + i64::from(turn) * self.profile.bend_cost;
                    let next = State {
                        node: next,
                        layer: state.layer,
                        heading: index as u8,
                    };
                    relax(
                        Vertex::Grid(next),
                        step_cost,
                        Came {
                            from: Some(state),
                            via: None,
                        },
                        &mut open,
                    );
                }
            }
            for &kind in self.via_kinds {
                let kind_of = self.design.via_kind(kind);
                for layer in self
                    .design
                    .layer_ids()
                    .filter(|&l| l.0 != state.layer && self.design.layers[l.index()].routable)
                {
                    if !kind_of.connects(LayerId(state.layer), layer) {
                        continue;
                    }
                    let clear = *via_clear.entry((state.node, kind)).or_insert_with(|| {
                        let via = Via {
                            net: self.net,
                            kind,
                            position: self.grid.point(state.node),
                        };
                        self.layout.is_via_clear(&via)
                    });
                    if clear {
                        let next = State {
                            node: state.node,
                            layer: layer.0,
                            heading: UNSET,
                        };
                        relax(
                            Vertex::Grid(next),
                            self.profile.via_cost,
                            Came {
                                from: Some(state),
                                via: Some(kind),
                            },
                            &mut open,
                        );
                    }
                }
            }
        }
        None
    }

    /// A straight track between the pad centres, when one fits on a layer both share.
    fn direct_line(&self) -> Option<Vec<Step>> {
        let segment = Segment::new(self.from.position, self.to.position);
        let layer = self.routable_layers(self.from).find(|&l| {
            self.to.is_on(l) && self.layout.is_track_clear(self.net, l, segment, self.width)
        })?;
        Some(vec![
            Step {
                at: self.from.position,
                layer,
                via: None,
            },
            Step {
                at: self.to.position,
                layer,
                via: None,
            },
        ])
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
                if self
                    .layout
                    .is_track_clear(self.net, copper.layer, segment, self.width)
                {
                    let state = State {
                        node,
                        layer: copper.layer.0,
                        heading: UNSET,
                    };
                    states.push((state, pad.position.distance(point)));
                }
            }
        }
        states
    }

    /// Never more than what is left to pay: the straight distance to the target,
    /// and a via if the target has no copper on this layer.
    fn estimate(&self, state: State) -> i64 {
        let distance = self.grid.point(state.node).distance(self.to.position);
        let via = if self.to.is_on(LayerId(state.layer)) {
            0
        } else {
            self.profile.via_cost
        };
        distance + via
    }

    fn steps(&self, last: State, came: &HashMap<State, Came>) -> Vec<Step> {
        let mut reversed = vec![Step {
            at: self.to.position,
            layer: LayerId(last.layer),
            via: None,
        }];
        let mut current = Some(last);
        while let Some(state) = current {
            let origin = came[&state];
            reversed.push(Step {
                at: self.grid.point(state.node),
                layer: LayerId(state.layer),
                via: origin.via,
            });
            current = origin.from;
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

/// How many 45° steps lie between a heading and the next direction.
fn turn(heading: u8, direction: usize) -> u8 {
    if heading == UNSET {
        return 0;
    }
    let difference = (i16::from(heading) - direction as i16).rem_euclid(8) as u8;
    difference.min(8 - difference)
}
