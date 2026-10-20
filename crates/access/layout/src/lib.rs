//! Layout Access: how the routing state is stored and queried (volatility V11 of
//! docs/Architecture/Volatilities.md). For now a uniform grid of buckets per
//! layer, queried one item at a time.

mod buckets;

use buckets::Buckets;
use traxide_contracts::geometry::{Point, Rect, Segment, Shape};
use traxide_contracts::{
    ConstraintSet, Design, KeepoutScope, LayerId, LayoutAccess, NetId, Track, Via,
};

/// Buckets of about this size keep a clearance query to a handful of buckets for
/// the track widths and clearances of ordinary boards.
const BUCKET_SIZE: i32 = 1_000_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Owner {
    Net(Option<NetId>),
    /// Not copper, so no clearance applies: nothing may overlap it.
    Keepout(KeepoutScope),
}

#[derive(Debug, Clone)]
struct Obstacle {
    owner: Owner,
    shape: Shape,
    bounds: Rect,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Item {
    Track,
    Via,
}

/// The copper and keepouts of a design, and the routes added so far.
#[derive(Debug)]
pub struct Layout<'a> {
    design: &'a Design,
    rules: &'a ConstraintSet,
    obstacles: Vec<Obstacle>,
    layers: Vec<Buckets>,
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
            obstacles: Vec::new(),
            layers,
        };
        for pad in &design.pads {
            for copper in &pad.copper {
                layout.insert(copper.layer, Owner::Net(pad.net), copper.shape.clone());
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
        for track in &design.existing.tracks {
            layout.add_track(track);
        }
        for via in &design.existing.vias {
            layout.add_via(via);
        }
        layout
    }

    fn insert(&mut self, layer: LayerId, owner: Owner, shape: Shape) {
        let bounds = shape.bounds();
        let id = self.obstacles.len();
        self.obstacles.push(Obstacle {
            owner,
            shape,
            bounds,
        });
        self.layers[layer.index()].insert(bounds, id);
    }

    fn is_clear(&self, net: NetId, layer: LayerId, shape: &Shape, item: Item) -> bool {
        if !shape.is_inside(&self.design.outline, self.rules.edge_clearance()) {
            return false;
        }
        let reach = shape.bounds().inflated(self.rules.largest_clearance());
        self.layers[layer.index()].candidates(reach).all(|id| {
            let obstacle = &self.obstacles[id];
            if !obstacle.bounds.intersects(reach) {
                return true;
            }
            let clearance = match obstacle.owner {
                Owner::Net(Some(owner)) if owner == net => return true,
                Owner::Net(other) => self.rules.clearance(other, Some(net)),
                Owner::Keepout(KeepoutScope::TracksOnly) if item == Item::Via => return true,
                Owner::Keepout(KeepoutScope::ViasOnly) if item == Item::Track => return true,
                Owner::Keepout(_) => 0,
            };
            !shape.is_closer_than(&obstacle.shape, clearance)
        })
    }

    fn via_copper(&self, via: &Via) -> impl Iterator<Item = (LayerId, Shape)> + '_ {
        let offset = via.position;
        self.design.via_kind(via.kind).copper.iter().map(move |c| {
            (
                c.layer,
                c.shape.map(|p| Point::new(p.x + offset.x, p.y + offset.y)),
            )
        })
    }
}

impl LayoutAccess for Layout<'_> {
    fn is_track_clear(&self, net: NetId, layer: LayerId, segment: Segment, width: i32) -> bool {
        self.is_clear(net, layer, &Shape::track(segment, width), Item::Track)
    }

    fn is_via_clear(&self, via: &Via) -> bool {
        self.via_copper(via)
            .all(|(layer, shape)| self.is_clear(via.net, layer, &shape, Item::Via))
    }

    fn add_track(&mut self, track: &Track) {
        for pair in track.path.windows(2) {
            let shape = Shape::track(Segment::new(pair[0], pair[1]), track.width);
            self.insert(track.layer, Owner::Net(Some(track.net)), shape);
        }
    }

    fn add_via(&mut self, via: &Via) {
        let copper: Vec<_> = self.via_copper(via).collect();
        for (layer, shape) in copper {
            self.insert(layer, Owner::Net(Some(via.net)), shape);
        }
    }
}
