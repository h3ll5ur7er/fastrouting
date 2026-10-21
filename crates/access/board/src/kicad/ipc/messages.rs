//! The messages of KiCad's IPC API, generated at build time from KiCad's own
//! definitions in `proto/kicad`, whose README names their source and licence.

#[allow(dead_code, clippy::all, clippy::pedantic)]
mod generated {
    include!(concat!(env!("OUT_DIR"), "/kiapi.rs"));
}

pub(crate) use generated::kiapi::{board, common};
pub(crate) use prost_types::Any;

use prost::{Message, Name};

/// Packing messages into `google.protobuf.Any` and out of it again.
pub(crate) trait Packed {
    fn of<M: Message + Name>(message: &M) -> Self;

    /// The message inside, if it is an `M`.
    fn to<M: Message + Name + Default>(&self) -> Option<M>;
}

impl Packed for Any {
    fn of<M: Message + Name>(message: &M) -> Self {
        Self {
            type_url: M::type_url(),
            value: message.encode_to_vec(),
        }
    }

    fn to<M: Message + Name + Default>(&self) -> Option<M> {
        self.to_msg().ok()
    }
}
