use crate::{NetId, PadId};

/// The connections to route, in the order to route them.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Plan {
    pub connections: Vec<Connection>,
}

/// Two pads of one net that routing joins with copper.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Connection {
    pub net: NetId,
    pub from: PadId,
    pub to: PadId,
}
