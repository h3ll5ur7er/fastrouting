use crate::{NetClassId, NetId, ViaKindId};

/// The rules compiled for routing: everything a router asks, answered in O(1).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConstraintSet {
    class_of_net: Vec<NetClassId>,
    default_class: NetClassId,
    track_width_of_class: Vec<i32>,
    via_kinds_of_class: Vec<Vec<ViaKindId>>,
    /// Row-major, classes × classes.
    clearance_between_classes: Vec<i32>,
    edge_clearance: i32,
}

impl ConstraintSet {
    /// Tables indexed by class; `clearance_between_classes` is a full square matrix.
    pub fn new(
        class_of_net: Vec<NetClassId>,
        default_class: NetClassId,
        track_width_of_class: Vec<i32>,
        via_kinds_of_class: Vec<Vec<ViaKindId>>,
        clearance_between_classes: Vec<i32>,
        edge_clearance: i32,
    ) -> Self {
        let classes = track_width_of_class.len();
        assert_eq!(via_kinds_of_class.len(), classes);
        assert_eq!(clearance_between_classes.len(), classes * classes);
        Self {
            class_of_net,
            default_class,
            track_width_of_class,
            via_kinds_of_class,
            clearance_between_classes,
            edge_clearance,
        }
    }

    fn class(&self, net: Option<NetId>) -> NetClassId {
        net.map_or(self.default_class, |net| self.class_of_net[net.index()])
    }

    pub fn track_width(&self, net: NetId) -> i32 {
        self.track_width_of_class[self.class(Some(net)).index()]
    }

    pub fn via_kinds(&self, net: NetId) -> &[ViaKindId] {
        &self.via_kinds_of_class[self.class(Some(net)).index()]
    }

    /// The clearance between copper of two nets; `None` is copper of no net.
    pub fn clearance(&self, a: Option<NetId>, b: Option<NetId>) -> i32 {
        let classes = self.track_width_of_class.len();
        self.clearance_between_classes[self.class(a).index() * classes + self.class(b).index()]
    }

    pub fn largest_clearance(&self) -> i32 {
        self.clearance_between_classes
            .iter()
            .copied()
            .max()
            .unwrap_or(0)
            .max(self.edge_clearance)
    }

    pub fn edge_clearance(&self) -> i32 {
        self.edge_clearance
    }
}
