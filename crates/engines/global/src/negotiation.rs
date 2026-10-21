use crate::capacity::{Axis, Cells};
use std::cmp::Reverse;
use std::collections::BinaryHeap;
use traxide_contracts::{ConstraintSet, Design, Pad, Plan, Profile};
use traxide_scratch::Stamped;

/// Rounds of negotiation before giving up on a plan without crowding; a few
/// usually suffice, and detailed routing resolves what remains.
const ROUNDS: i32 = 8;

const MOVES: [((i32, i32), Axis); 4] = [
    ((1, 0), Axis::Horizontal),
    ((-1, 0), Axis::Horizontal),
    ((0, 1), Axis::Vertical),
    ((0, -1), Axis::Vertical),
];

/// The cells each connection of `plan` passes, in order; empty for a connection
/// that found no way, which detailed routing then searches without a corridor.
pub(crate) fn negotiate(
    design: &Design,
    plan: &Plan,
    rules: &ConstraintSet,
    cells: &Cells,
    profile: &Profile,
) -> Vec<Vec<(i32, i32)>> {
    let mut usage = Usage::new(cells);
    let mut paths: Vec<Vec<State>> = vec![Vec::new(); plan.connections.len()];
    let mut search = Search {
        design,
        cells,
        profile,
        best: Stamped::default(),
        came: Stamped::default(),
    };
    for round in 0..ROUNDS {
        // Crowding grows more expensive each round, so that connections that
        // could go elsewhere eventually do.
        let crowding = 0.5 * 1.6_f64.powi(round);
        for (index, connection) in plan.connections.iter().enumerate() {
            usage.change(cells, &paths[index], -1);
            let may_change_layers = !rules.via_kinds(connection.net).is_empty();
            let (from, to) = (design.pad(connection.from), design.pad(connection.to));
            paths[index] = search.run(&usage, crowding, may_change_layers, from, to);
            usage.change(cells, &paths[index], 1);
        }
        if !usage.settle(cells) {
            break;
        }
    }
    paths
        .into_iter()
        .map(|path| path.into_iter().map(|state| state.cell).collect())
        .collect()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct State {
    cell: (i32, i32),
    layer: usize,
}

/// How many connections run through each cell along each axis, and how crowded
/// each cell has been in earlier rounds (PathFinder's history cost).
struct Usage {
    horizontal: Vec<u16>,
    vertical: Vec<u16>,
    history: Vec<f64>,
}

impl Usage {
    fn new(cells: &Cells) -> Self {
        let count = cells.count() * cells.layers();
        Self {
            horizontal: vec![0; count],
            vertical: vec![0; count],
            history: vec![0.0; count],
        }
    }

    fn of(&self, index: usize, axis: Axis) -> u16 {
        match axis {
            Axis::Horizontal => self.horizontal[index],
            Axis::Vertical => self.vertical[index],
        }
    }

    /// Adds or removes one use of every cell the path runs through, once per
    /// axis, however many steps of the path touch it.
    fn change(&mut self, cells: &Cells, path: &[State], by: i32) {
        let mut touched: Vec<(usize, Axis)> = path
            .windows(2)
            .filter(|pair| pair[0].layer == pair[1].layer)
            .flat_map(|pair| {
                let axis = if pair[0].cell.1 == pair[1].cell.1 {
                    Axis::Horizontal
                } else {
                    Axis::Vertical
                };
                pair.iter()
                    .map(move |state| (cells.index(state.cell, state.layer), axis))
            })
            .collect();
        touched.sort_unstable_by_key(|&(index, axis)| (index, axis == Axis::Vertical));
        touched.dedup();
        for (index, axis) in touched {
            let counter = match axis {
                Axis::Horizontal => &mut self.horizontal[index],
                Axis::Vertical => &mut self.vertical[index],
            };
            *counter = counter.saturating_add_signed(by as i16);
        }
    }

    /// Adds this round's crowding to the history; whether any cell is crowded.
    fn settle(&mut self, cells: &Cells) -> bool {
        let mut crowded = false;
        for layer in 0..cells.layers() {
            for row in 0..cells.rows() {
                for column in 0..cells.columns() {
                    let index = cells.index((column, row), layer);
                    for axis in [Axis::Horizontal, Axis::Vertical] {
                        let excess = self
                            .of(index, axis)
                            .saturating_sub(u16::from(cells.capacity((column, row), layer, axis)));
                        if excess > 0 {
                            crowded = true;
                            self.history[index] += f64::from(excess);
                        }
                    }
                }
            }
        }
        crowded
    }
}

struct Search<'a> {
    design: &'a Design,
    cells: &'a Cells,
    profile: &'a Profile,
    best: Stamped<i64>,
    /// The state each state was reached from, plus one; zero for a start.
    came: Stamped<u32>,
}

impl Search<'_> {
    fn run(
        &mut self,
        usage: &Usage,
        crowding: f64,
        may_change_layers: bool,
        from: &Pad,
        to: &Pad,
    ) -> Vec<State> {
        let cells = self.cells;
        let (Some(start), Some(target)) =
            (cells.cell_at(from.position), cells.cell_at(to.position))
        else {
            return Vec::new();
        };
        let design = self.design;
        let via_cost = self.profile.via_cost;
        let routable = |layer: usize| design.layers[layer].routable;
        let target_layers: Vec<usize> = to
            .copper
            .iter()
            .map(|c| c.layer.index())
            .filter(|&l| routable(l))
            .collect();
        let own_cells: Vec<(i32, i32)> = [from, to]
            .iter()
            .flat_map(|pad| {
                pad.copper
                    .iter()
                    .flat_map(|c| cells.covered(c.shape.bounds()))
            })
            .collect();
        self.best.reset(cells.count() * cells.layers());
        self.came.reset(cells.count() * cells.layers());
        let mut open = BinaryHeap::new();
        for layer in from
            .copper
            .iter()
            .map(|c| c.layer.index())
            .filter(|&l| routable(l))
        {
            let index = cells.index(start, layer);
            self.best.set(index, 0);
            self.came.set(index, 0);
            open.push(Reverse((
                self.estimate(start, layer, target, &target_layers),
                0_i64,
                index,
            )));
        }
        while let Some(Reverse((_, cost, index))) = open.pop() {
            if cost > self.best.get(index).unwrap_or(i64::MAX) {
                continue;
            }
            let state = self.state(index);
            if state.cell == target && target_layers.contains(&state.layer) {
                return self.path(index);
            }
            let mut relax = |next: State, step: i64, open: &mut BinaryHeap<_>| {
                let next_index = cells.index(next.cell, next.layer);
                let total = cost + step;
                if total < self.best.get(next_index).unwrap_or(i64::MAX) {
                    self.best.set(next_index, total);
                    self.came.set(next_index, index as u32 + 1);
                    let estimate = self.estimate(next.cell, next.layer, target, &target_layers);
                    open.push(Reverse((total + estimate, total, next_index)));
                }
            };
            for ((dx, dy), axis) in MOVES {
                let cell = (state.cell.0 + dx, state.cell.1 + dy);
                if !cells.contains(cell) {
                    continue;
                }
                let next = State {
                    cell,
                    layer: state.layer,
                };
                let next_index = cells.index(cell, state.layer);
                let capacity = cells.capacity(cell, state.layer, axis);
                // A cell with no lane left is a wall, except under the connection's
                // own pads: they fill the cells it starts and ends in.
                if capacity == 0 && !own_cells.contains(&cell) {
                    continue;
                }
                let excess = (usage.of(next_index, axis) + 1).saturating_sub(u16::from(capacity));
                let price =
                    (1.0 + usage.history[next_index]) * (1.0 + crowding * f64::from(excess));
                relax(next, (f64::from(cells.size()) * price) as i64, &mut open);
            }
            if may_change_layers {
                for layer in (0..cells.layers()).filter(|&l| l != state.layer && routable(l)) {
                    relax(
                        State {
                            cell: state.cell,
                            layer,
                        },
                        via_cost,
                        &mut open,
                    );
                }
            }
        }
        Vec::new()
    }

    /// Never more than what is left: a cell's width per step, and a via if the
    /// target is not on this layer.
    fn estimate(
        &self,
        cell: (i32, i32),
        layer: usize,
        target: (i32, i32),
        target_layers: &[usize],
    ) -> i64 {
        let steps = i64::from((cell.0 - target.0).abs() + (cell.1 - target.1).abs());
        let via = if target_layers.contains(&layer) {
            0
        } else {
            self.profile.via_cost
        };
        steps * i64::from(self.cells.size()) + via
    }

    fn state(&self, index: usize) -> State {
        let per_layer = self.cells.count();
        let (layer, cell) = (index / per_layer, index % per_layer);
        let columns = self.cells.columns() as usize;
        State {
            cell: ((cell % columns) as i32, (cell / columns) as i32),
            layer,
        }
    }

    fn path(&self, last: usize) -> Vec<State> {
        let mut reversed = Vec::new();
        let mut current = Some(last);
        while let Some(index) = current {
            reversed.push(self.state(index));
            current = self
                .came
                .get(index)
                .and_then(|came| (came as usize).checked_sub(1));
        }
        reversed.reverse();
        reversed
    }
}
