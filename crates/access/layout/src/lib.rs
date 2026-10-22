//! Layout Access: how the routing state is stored and queried (volatility V11 of
//! docs/Architecture/Volatilities.md). For now a uniform grid of buckets per
//! layer, queried one item at a time.

mod buckets;
mod outline;

use buckets::Buckets;
use outline::Outline;
use traxide_contracts::geometry::{Point, Rect, Segment, Shape};
use traxide_contracts::{
    ConnectionId, ConstraintSet, Design, KeepoutScope, LayerId, LayoutAccess, NetId, Obstruction,
    Routes, Via,
};

/// Buckets of about this size keep a clearance query to a handful of buckets for
/// the track widths and clearances of ordinary boards.
const BUCKET_SIZE: i32 = 1_000_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Owner {
    /// Copper that stays: pads and locked tracks.
    Fixed(Option<NetId>),
    /// Copper routed for a connection, which rip-up may take away again.
    Routed {
        net: NetId,
        connection: ConnectionId,
    },
    /// Not copper, so no clearance applies: nothing may overlap it.
    Keepout(KeepoutScope),
}

#[derive(Debug, Clone)]
struct Obstacle {
    owner: Owner,
    shape: Shape,
    bounds: Rect,
    /// Removed obstacles stay in the buckets, ignored, rather than being searched for.
    removed: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Item {
    Track,
    Via,
}

/// Whether a query needs every obstruction or only whether there is one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Wanted {
    All,
    First,
}

/// The copper and keepouts of a design, and the routes added so far.
#[derive(Debug)]
pub struct Layout<'a> {
    design: &'a Design,
    rules: &'a ConstraintSet,
    outline: Outline,
    obstacles: Vec<Obstacle>,
    layers: Vec<Buckets>,
    /// The obstacles of each routed connection, by connection.
    routed: Vec<Vec<usize>>,
}

impl<'a> Layout<'a> {
    /// Everything routing must keep clear of: pads, keepouts and existing copper.
    pub fn build(design: &'a Design, rules: &'a ConstraintSet) -> Self {
        let bounds = design.outline.bounds();
        let layers = design
            .layers
            .iter()
            .map(|_| Buckets::new(bounds, BUCKET_SIZE))
            .collect();
        let mut layout = Self {
            design,
            rules,
            outline: Outline::new(&design.outline, rules, BUCKET_SIZE),
            obstacles: Vec::new(),
            layers,
            routed: Vec::new(),
        };
        for pad in &design.pads {
            for copper in &pad.copper {
                layout.insert(copper.layer, Owner::Fixed(pad.net), copper.shape.clone());
            }
        }
        for keepout in &design.keepouts {
            let layers: Vec<LayerId> = match keepout.layer {
                Some(layer) => vec![layer],
                None => design.layer_ids().collect(),
            };
            for layer in layers {
                layout.insert(layer, Owner::Keepout(keepout.scope), keepout.shape.clone());
            }
        }
        for (layer, net, shape) in copper_of(design, &design.existing) {
            layout.insert(layer, Owner::Fixed(Some(net)), shape);
        }
        layout
    }

    fn insert(&mut self, layer: LayerId, owner: Owner, shape: Shape) -> usize {
        let bounds = shape.bounds();
        let id = self.obstacles.len();
        self.obstacles.push(Obstacle {
            owner,
            shape,
            bounds,
            removed: false,
        });
        self.layers[layer.index()].insert(bounds, id);
        id
    }

    fn obstruction(
        &self,
        net: NetId,
        layer: LayerId,
        shape: &Shape,
        item: Item,
        wanted: Wanted,
    ) -> Obstruction {
        if !self.outline.holds(shape) {
            return Obstruction::Fixed;
        }
        let reach = shape.bounds().inflated(self.rules.largest_clearance());
        let mut routed = Vec::new();
        for id in self.layers[layer.index()].candidates(reach) {
            let obstacle = &self.obstacles[id];
            if obstacle.removed || !obstacle.bounds.intersects(reach) {
                continue;
            }
            let (connection, clearance) = match obstacle.owner {
                Owner::Fixed(Some(owner)) | Owner::Routed { net: owner, .. } if owner == net => {
                    continue;
                }
                Owner::Fixed(other) => (None, self.rules.clearance(other, Some(net))),
                Owner::Routed {
                    net: other,
                    connection,
                } => (
                    Some(connection),
                    self.rules.clearance(Some(other), Some(net)),
                ),
                Owner::Keepout(KeepoutScope::TracksOnly) if item == Item::Via => continue,
                Owner::Keepout(KeepoutScope::ViasOnly) if item == Item::Track => continue,
                Owner::Keepout(_) => (None, 0),
            };
            if !shape.is_closer_than(&obstacle.shape, clearance) {
                continue;
            }
            match connection {
                None => return Obstruction::Fixed,
                Some(connection) if wanted == Wanted::First => {
                    return Obstruction::Routed(vec![connection]);
                }
                Some(connection) => routed.push(connection),
            }
        }
        obstruction_by(routed)
    }

    fn via_copper(&self, via: &Via) -> impl Iterator<Item = (LayerId, Shape)> + '_ {
        via_copper(self.design, via)
    }

    fn via_obstruction_of(&self, via: &Via, wanted: Wanted) -> Obstruction {
        let mut routed = Vec::new();
        for (layer, shape) in self.via_copper(via) {
            match self.obstruction(via.net, layer, &shape, Item::Via, wanted) {
                Obstruction::None => {}
                Obstruction::Fixed => return Obstruction::Fixed,
                Obstruction::Routed(connections) if wanted == Wanted::First => {
                    return Obstruction::Routed(connections);
                }
                Obstruction::Routed(connections) => routed.extend(connections),
            }
        }
        obstruction_by(routed)
    }
}

fn obstruction_by(mut routed: Vec<ConnectionId>) -> Obstruction {
    if routed.is_empty() {
        return Obstruction::None;
    }
    routed.sort_unstable();
    routed.dedup();
    Obstruction::Routed(routed)
}

fn via_copper<'d>(design: &'d Design, via: &Via) -> impl Iterator<Item = (LayerId, Shape)> + 'd {
    let offset = via.position;
    design.via_kind(via.kind).copper.iter().map(move |c| {
        (
            c.layer,
            c.shape.map(|p| Point::new(p.x + offset.x, p.y + offset.y)),
        )
    })
}

/// Every piece of copper of `routes`, with its layer and net.
fn copper_of<'r>(
    design: &'r Design,
    routes: &'r Routes,
) -> impl Iterator<Item = (LayerId, NetId, Shape)> + 'r {
    let tracks = routes.tracks.iter().flat_map(|track| {
        track.path.windows(2).map(move |pair| {
            (
                track.layer,
                track.net,
                Shape::track(Segment::new(pair[0], pair[1]), track.width),
            )
        })
    });
    let vias = routes.vias.iter().flat_map(move |via| {
        via_copper(design, via).map(move |(layer, shape)| (layer, via.net, shape))
    });
    tracks.chain(vias)
}

impl LayoutAccess for Layout<'_> {
    fn track_obstruction(
        &self,
        net: NetId,
        layer: LayerId,
        segment: Segment,
        width: i32,
    ) -> Obstruction {
        self.obstruction(
            net,
            layer,
            &Shape::track(segment, width),
            Item::Track,
            Wanted::All,
        )
    }

    fn via_obstruction(&self, via: &Via) -> Obstruction {
        self.via_obstruction_of(via, Wanted::All)
    }

    fn is_track_clear(&self, net: NetId, layer: LayerId, segment: Segment, width: i32) -> bool {
        self.obstruction(
            net,
            layer,
            &Shape::track(segment, width),
            Item::Track,
            Wanted::First,
        ) == Obstruction::None
    }

    fn is_via_clear(&self, via: &Via) -> bool {
        self.via_obstruction_of(via, Wanted::First) == Obstruction::None
    }

    fn add_route(&mut self, connection: ConnectionId, routes: &Routes) {
        let copper: Vec<_> = copper_of(self.design, routes).collect();
        let ids: Vec<usize> = copper
            .into_iter()
            .map(|(layer, net, shape)| self.insert(layer, Owner::Routed { net, connection }, shape))
            .collect();
        if self.routed.len() <= connection.index() {
            self.routed.resize(connection.index() + 1, Vec::new());
        }
        self.routed[connection.index()].extend(ids);
    }

    fn remove_route(&mut self, connection: ConnectionId) {
        if let Some(ids) = self.routed.get_mut(connection.index()) {
            for id in ids.drain(..) {
                self.obstacles[id].removed = true;
            }
        }
    }
}
