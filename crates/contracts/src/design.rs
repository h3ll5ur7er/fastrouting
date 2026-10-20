use crate::{LayerId, NetClassId, NetId, PadId, Routes, ViaKindId};
use traxide_geometry::{Point, Region, Shape};

/// A board to route, independent of the format it came from.
#[derive(Debug, Clone, PartialEq)]
pub struct Design {
    /// Copper layers from top to bottom.
    pub layers: Vec<Layer>,
    /// The board's outline: one polygon, or one per board of a panel.
    pub outline: Region,
    pub nets: Vec<Net>,
    pub pads: Vec<Pad>,
    pub via_kinds: Vec<ViaKind>,
    pub keepouts: Vec<Keepout>,
    /// Copper already on the board; routing keeps it as it is.
    pub existing: Routes,
    pub rules: DesignRules,
    /// The coordinate step of the board's format. Routes are rounded to it when
    /// stored, so routing keeps that much extra clearance.
    pub resolution: i32,
}

impl Design {
    pub fn net(&self, id: NetId) -> &Net {
        &self.nets[id.index()]
    }

    pub fn pad(&self, id: PadId) -> &Pad {
        &self.pads[id.index()]
    }

    pub fn via_kind(&self, id: ViaKindId) -> &ViaKind {
        &self.via_kinds[id.index()]
    }

    pub fn layer_ids(&self) -> impl Iterator<Item = LayerId> + use<> {
        (0..self.layers.len()).map(LayerId::from_index)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Layer {
    pub name: String,
    /// Plane layers carry copper but no tracks.
    pub routable: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Net {
    pub name: String,
    pub class: NetClassId,
    pub pads: Vec<PadId>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Pad {
    /// Unique on the board, such as `R1-2`.
    pub name: String,
    pub net: Option<NetId>,
    pub position: Point,
    /// Absolute shapes per layer; a through-hole pad has one on every layer.
    pub copper: Vec<LayerShape>,
}

impl Pad {
    pub fn is_on(&self, layer: LayerId) -> bool {
        self.copper.iter().any(|c| c.layer == layer)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LayerShape {
    pub layer: LayerId,
    pub shape: Shape,
}

/// A via padstack. Its copper is relative to the via's centre.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ViaKind {
    pub name: String,
    pub copper: Vec<LayerShape>,
}

impl ViaKind {
    pub fn connects(&self, from: LayerId, to: LayerId) -> bool {
        let on = |layer| self.copper.iter().any(|c| c.layer == layer);
        on(from) && on(to)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Keepout {
    /// `None` blocks every layer.
    pub layer: Option<LayerId>,
    pub shape: Shape,
    pub scope: KeepoutScope,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeepoutScope {
    Everything,
    TracksOnly,
    ViasOnly,
}

/// The rules as the board states them. Only the Rules Engine interprets them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DesignRules {
    pub classes: Vec<NetClass>,
    /// The class of copper that belongs to no net, such as a mounting hole's pad.
    pub default_class: NetClassId,
    pub edge_clearance: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NetClass {
    pub name: String,
    pub track_width: i32,
    pub clearance: i32,
    /// In order of preference.
    pub via_kinds: Vec<ViaKindId>,
}
