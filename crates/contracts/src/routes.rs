use crate::{Connection, LayerId, NetId, PadId, ViaKindId};
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

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RoutingResult {
    pub routes: Routes,
    pub unrouted: Vec<Connection>,
    /// Pads no track could leave at all, in id order: the reason some of the
    /// unrouted connections are.
    pub hemmed_in: Vec<PadId>,
}
