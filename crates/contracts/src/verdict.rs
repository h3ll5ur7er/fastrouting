use crate::{LayerId, NetId, PadId};

/// What verification found wrong with a routed design; empty when all is well.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Verdict {
    pub violations: Vec<Violation>,
}

impl Verdict {
    pub fn is_clean(&self) -> bool {
        self.violations.is_empty()
    }
}

/// Ordered by kind, then layer and items, so reports read the same every time.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum Violation {
    /// Copper of two nets closer than their clearance.
    Clearance {
        layer: LayerId,
        a: CopperItem,
        b: CopperItem,
        required: i32,
    },
    /// Copper outside the board or closer to its edge than allowed.
    Edge { layer: LayerId, item: CopperItem },
    /// Copper where a keepout forbids it.
    Keepout { layer: LayerId, item: CopperItem },
    /// Pads of a net that copper does not join to the net's first pad.
    Unconnected { net: NetId, pads: Vec<PadId> },
}

/// A piece of copper: a pad of the design, or a track or via of the verified
/// routes, by index.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum CopperItem {
    Pad(PadId),
    Track(usize),
    Via(usize),
}
