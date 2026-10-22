use crate::{Connection, LayerId, NetId, PadId, ViaKindId};
use std::collections::HashSet;
use traxide_geometry::Point;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Routes {
    pub tracks: Vec<Track>,
    pub vias: Vec<Via>,
}

impl Routes {
    pub fn extend(&mut self, other: Routes) {
        self.tracks.extend(other.tracks);
        self.vias.extend(other.vias);
    }

    /// These routes without the vias that repeat one of `kept`'s or an earlier
    /// one of their own: connections of a net may each place a via on the same
    /// spot, which is one hole.
    #[must_use]
    pub fn without_repeated_vias(mut self, kept: &Routes) -> Self {
        let mut seen: HashSet<(NetId, ViaKindId, Point)> = kept.vias.iter().map(Via::key).collect();
        self.vias.retain(|via| seen.insert(via.key()));
        self
    }
}

/// A polyline of copper on one layer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Track {
    pub net: NetId,
    pub layer: LayerId,
    pub width: i32,
    pub path: Vec<Point>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Via {
    pub net: NetId,
    pub kind: ViaKindId,
    pub position: Point,
}

impl Via {
    fn key(&self) -> (NetId, ViaKindId, Point) {
        (self.net, self.kind, self.position)
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RoutingResult {
    pub routes: Routes,
    pub unrouted: Vec<Connection>,
    /// Pads no track could leave at all, in id order: the reason some of the
    /// unrouted connections are.
    pub hemmed_in: Vec<PadId>,
    /// Pads every way out of which the routed copper of other connections took,
    /// in id order; routed in another order, they may have been left.
    pub exits_taken: Vec<PadId>,
    /// The unrouted connections between whose pads the board's own copper,
    /// keepouts and edge leave no way at all, in the order they failed.
    pub cut_off: Vec<Connection>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn via(net: usize, x: i32) -> Via {
        Via {
            net: NetId::from_index(net),
            kind: ViaKindId(0),
            position: Point::new(x, 0),
        }
    }

    #[test]
    fn a_via_repeating_one_kept_or_placed_before_is_left_out() {
        let kept = Routes {
            tracks: vec![],
            vias: vec![via(0, 1)],
        };
        let routes = Routes {
            tracks: vec![],
            vias: vec![via(0, 1), via(0, 2), via(1, 2), via(0, 2)],
        };
        let left = routes.without_repeated_vias(&kept);
        assert_eq!(left.vias, [via(0, 2), via(1, 2)]);
    }
}
