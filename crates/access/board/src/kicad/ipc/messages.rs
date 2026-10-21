//! The messages of KiCad's API that traxide sends and reads, defined here from
//! the protocol's field numbers and types rather than generated from KiCad's
//! `.proto` files (GPL-3.0), so that none of their text enters traxide. Fields
//! traxide does not need are left out; protobuf skips them when decoding.

use prost::{Message, Name};
use std::collections::HashMap;

macro_rules! named {
    ($type:ident, $package:literal) => {
        impl Name for $type {
            const NAME: &'static str = stringify!($type);
            const PACKAGE: &'static str = $package;
            fn type_url() -> String {
                format!("type.googleapis.com/{}.{}", $package, stringify!($type))
            }
        }
    };
}

/// `google.protobuf.Any`: a message of the type its URL names.
#[derive(Clone, PartialEq, Message)]
pub struct Any {
    #[prost(string, tag = "1")]
    pub type_url: String,
    #[prost(bytes = "vec", tag = "2")]
    pub value: Vec<u8>,
}

impl Any {
    pub fn of<M: Message + Name>(message: &M) -> Self {
        Self {
            type_url: M::type_url(),
            value: message.encode_to_vec(),
        }
    }

    /// The message inside, if it is an `M`.
    pub fn to<M: Message + Name + Default>(&self) -> Option<M> {
        (self.type_url == M::type_url())
            .then(|| M::decode(self.value.as_slice()).ok())
            .flatten()
    }
}

#[derive(Clone, PartialEq, Message)]
pub struct ApiRequest {
    #[prost(message, optional, tag = "1")]
    pub header: Option<RequestHeader>,
    #[prost(message, optional, tag = "2")]
    pub message: Option<Any>,
}

#[derive(Clone, PartialEq, Message)]
pub struct RequestHeader {
    #[prost(string, tag = "1")]
    pub kicad_token: String,
    #[prost(string, tag = "2")]
    pub client_name: String,
}

#[derive(Clone, PartialEq, Message)]
pub struct ApiResponse {
    #[prost(message, optional, tag = "2")]
    pub status: Option<ResponseStatus>,
    #[prost(message, optional, tag = "3")]
    pub message: Option<Any>,
}

#[derive(Clone, PartialEq, Message)]
pub struct ResponseStatus {
    #[prost(enumeration = "StatusCode", tag = "1")]
    pub status: i32,
    #[prost(string, tag = "2")]
    pub error_message: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, prost::Enumeration)]
#[repr(i32)]
pub enum StatusCode {
    Unknown = 0,
    Ok = 1,
    Timeout = 2,
    BadRequest = 3,
    NotReady = 4,
    Unhandled = 5,
    TokenMismatch = 6,
    Busy = 7,
    Unimplemented = 8,
}

/// `google.protobuf.Empty`, KiCad's reply to commands that return nothing.
#[cfg(feature = "testing")]
#[derive(Clone, PartialEq, Message)]
pub struct Empty {}
#[cfg(feature = "testing")]
named!(Empty, "google.protobuf");

pub const DOCUMENT_PCB: i32 = 3;

#[derive(Clone, PartialEq, Message)]
pub struct GetOpenDocuments {
    #[prost(int32, tag = "1")]
    pub document_type: i32,
}
named!(GetOpenDocuments, "kiapi.common.commands");

#[derive(Clone, PartialEq, Message)]
pub struct GetOpenDocumentsResponse {
    #[prost(message, repeated, tag = "1")]
    pub documents: Vec<Document>,
}
named!(GetOpenDocumentsResponse, "kiapi.common.commands");

/// KiCad's `DocumentSpecifier`.
#[derive(Clone, PartialEq, Message)]
pub struct Document {
    #[prost(int32, tag = "1")]
    pub document_type: i32,
    #[prost(string, optional, tag = "4")]
    pub board_filename: Option<String>,
    #[prost(message, optional, tag = "5")]
    pub project: Option<Project>,
}

#[derive(Clone, PartialEq, Message)]
pub struct Project {
    #[prost(string, tag = "1")]
    pub name: String,
    #[prost(string, tag = "2")]
    pub path: String,
}

#[derive(Clone, PartialEq, Message)]
pub struct SaveDocumentToString {
    #[prost(message, optional, tag = "1")]
    pub document: Option<Document>,
}
named!(SaveDocumentToString, "kiapi.common.commands");

#[derive(Clone, PartialEq, Message)]
pub struct SavedDocumentResponse {
    #[prost(string, tag = "2")]
    pub contents: String,
}
named!(SavedDocumentResponse, "kiapi.common.commands");

#[derive(Clone, PartialEq, Message)]
pub struct GetNets {
    #[prost(message, optional, tag = "1")]
    pub board: Option<Document>,
}
named!(GetNets, "kiapi.board.commands");

#[derive(Clone, PartialEq, Message)]
pub struct NetsResponse {
    #[prost(message, repeated, tag = "1")]
    pub nets: Vec<Net>,
}
named!(NetsResponse, "kiapi.board.commands");

#[derive(Clone, PartialEq, Message)]
pub struct Net {
    #[prost(string, tag = "2")]
    pub name: String,
}

#[derive(Clone, PartialEq, Message)]
pub struct GetNetClassForNets {
    #[prost(message, repeated, tag = "1")]
    pub nets: Vec<Net>,
}
named!(GetNetClassForNets, "kiapi.board.commands");

#[derive(Clone, PartialEq, Message)]
pub struct NetClassForNetsResponse {
    /// Each net's class by the net's name.
    #[prost(map = "string, message", tag = "1")]
    pub classes: HashMap<String, NetClass>,
}
named!(NetClassForNetsResponse, "kiapi.board.commands");

/// A class that only combines others, for a net that has several.
pub const NET_CLASS_IMPLICIT: i32 = 2;

#[derive(Clone, PartialEq, Message)]
pub struct NetClass {
    #[prost(string, tag = "1")]
    pub name: String,
    #[prost(message, optional, tag = "3")]
    pub board: Option<NetClassBoardSettings>,
    #[prost(int32, tag = "5")]
    pub class_type: i32,
    /// The classes an implicit class combines, first the one that counts most.
    #[prost(string, repeated, tag = "6")]
    pub constituents: Vec<String>,
}

#[derive(Clone, PartialEq, Message)]
pub struct NetClassBoardSettings {
    #[prost(message, optional, tag = "1")]
    pub clearance: Option<Distance>,
    #[prost(message, optional, tag = "2")]
    pub track_width: Option<Distance>,
    #[prost(message, optional, tag = "6")]
    pub via_stack: Option<PadStack>,
}

#[derive(Clone, Copy, PartialEq, Message)]
pub struct Vector2 {
    #[prost(int64, tag = "1")]
    pub x_nm: i64,
    #[prost(int64, tag = "2")]
    pub y_nm: i64,
}

#[derive(Clone, Copy, PartialEq, Message)]
pub struct Distance {
    #[prost(int64, tag = "1")]
    pub value_nm: i64,
}

#[derive(Clone, PartialEq, Message)]
pub struct Kiid {
    #[prost(string, tag = "1")]
    pub value: String,
}

#[derive(Clone, PartialEq, Message)]
pub struct ItemHeader {
    #[prost(message, optional, tag = "1")]
    pub document: Option<Document>,
}

/// KiCad's `BoardLayer`: the front, the inner layers in order, then the back.
pub const LAYER_FRONT: i32 = 3;
pub const LAYER_BACK: i32 = 34;

#[derive(Clone, PartialEq, Message)]
pub struct Track {
    #[prost(message, optional, tag = "2")]
    pub start: Option<Vector2>,
    #[prost(message, optional, tag = "3")]
    pub end: Option<Vector2>,
    #[prost(message, optional, tag = "4")]
    pub width: Option<Distance>,
    #[prost(int32, tag = "6")]
    pub layer: i32,
    #[prost(message, optional, tag = "7")]
    pub net: Option<Net>,
}
named!(Track, "kiapi.board.types");

pub const VIA_THROUGH: i32 = 1;
pub const VIA_BLIND_BURIED: i32 = 2;

#[derive(Clone, PartialEq, Message)]
pub struct Via {
    #[prost(message, optional, tag = "2")]
    pub position: Option<Vector2>,
    #[prost(message, optional, tag = "3")]
    pub pad_stack: Option<PadStack>,
    #[prost(message, optional, tag = "5")]
    pub net: Option<Net>,
    #[prost(int32, tag = "6")]
    pub via_type: i32,
}
named!(Via, "kiapi.board.types");

pub const PADSTACK_NORMAL: i32 = 1;
pub const SHAPE_CIRCLE: i32 = 1;

#[derive(Clone, PartialEq, Message)]
pub struct PadStack {
    #[prost(int32, tag = "1")]
    pub stack_type: i32,
    #[prost(int32, repeated, tag = "2")]
    pub layers: Vec<i32>,
    #[prost(message, optional, tag = "3")]
    pub drill: Option<Drill>,
    #[prost(message, repeated, tag = "5")]
    pub copper_layers: Vec<PadStackLayer>,
}

#[derive(Clone, PartialEq, Message)]
pub struct Drill {
    #[prost(int32, tag = "1")]
    pub start_layer: i32,
    #[prost(int32, tag = "2")]
    pub end_layer: i32,
    #[prost(message, optional, tag = "3")]
    pub diameter: Option<Vector2>,
}

#[derive(Clone, PartialEq, Message)]
pub struct PadStackLayer {
    #[prost(int32, tag = "1")]
    pub layer: i32,
    #[prost(int32, tag = "2")]
    pub shape: i32,
    #[prost(message, optional, tag = "3")]
    pub size: Option<Vector2>,
}

#[derive(Clone, PartialEq, Message)]
pub struct BeginCommit {}
named!(BeginCommit, "kiapi.common.commands");

#[derive(Clone, PartialEq, Message)]
pub struct BeginCommitResponse {
    #[prost(message, optional, tag = "1")]
    pub id: Option<Kiid>,
}
named!(BeginCommitResponse, "kiapi.common.commands");

pub const COMMIT_PUSH: i32 = 1;
pub const COMMIT_DROP: i32 = 2;

#[derive(Clone, PartialEq, Message)]
pub struct EndCommit {
    #[prost(message, optional, tag = "1")]
    pub id: Option<Kiid>,
    #[prost(int32, tag = "2")]
    pub action: i32,
    #[prost(string, tag = "3")]
    pub message: String,
}
named!(EndCommit, "kiapi.common.commands");

#[derive(Clone, PartialEq, Message)]
pub struct EndCommitResponse {}
named!(EndCommitResponse, "kiapi.common.commands");

#[derive(Clone, PartialEq, Message)]
pub struct CreateItems {
    #[prost(message, optional, tag = "1")]
    pub header: Option<ItemHeader>,
    #[prost(message, repeated, tag = "2")]
    pub items: Vec<Any>,
}
named!(CreateItems, "kiapi.common.commands");

pub const REQUEST_OK: i32 = 1;
pub const ITEM_OK: i32 = 1;

#[derive(Clone, PartialEq, Message)]
pub struct CreateItemsResponse {
    #[prost(int32, tag = "2")]
    pub status: i32,
    #[prost(message, repeated, tag = "3")]
    pub created_items: Vec<ItemCreationResult>,
}
named!(CreateItemsResponse, "kiapi.common.commands");

#[derive(Clone, PartialEq, Message)]
pub struct ItemCreationResult {
    #[prost(message, optional, tag = "1")]
    pub status: Option<ItemStatus>,
}

#[derive(Clone, PartialEq, Message)]
pub struct ItemStatus {
    #[prost(int32, tag = "1")]
    pub code: i32,
    #[prost(string, tag = "2")]
    pub error_message: String,
}

#[cfg(feature = "testing")]
#[derive(Clone, PartialEq, Message)]
pub struct SaveDocument {
    #[prost(message, optional, tag = "1")]
    pub document: Option<Document>,
}
#[cfg(feature = "testing")]
named!(SaveDocument, "kiapi.common.commands");

#[cfg(feature = "testing")]
#[derive(Clone, PartialEq, Message)]
pub struct RunAction {
    #[prost(string, tag = "1")]
    pub action: String,
}
#[cfg(feature = "testing")]
named!(RunAction, "kiapi.common.commands");

#[cfg(feature = "testing")]
pub const ACTION_OK: i32 = 1;

#[cfg(feature = "testing")]
#[derive(Clone, PartialEq, Message)]
pub struct RunActionResponse {
    #[prost(int32, tag = "1")]
    pub status: i32,
}
#[cfg(feature = "testing")]
named!(RunActionResponse, "kiapi.common.commands");
