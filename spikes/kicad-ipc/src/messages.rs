//! The few messages of KiCad's API the spike sends and reads, defined here from the
//! protocol's field numbers and types rather than generated from KiCad's `.proto`
//! files (GPL-3.0), so that none of their text enters traxide. Fields we do not
//! need are left out; protobuf skips them when decoding.

use prost::{Message, Name};

macro_rules! kicad_name {
    ($type:ty, $package:literal, $name:literal) => {
        impl Name for $type {
            const NAME: &'static str = $name;
            const PACKAGE: &'static str = $package;
            fn type_url() -> String {
                format!("type.googleapis.com/{}.{}", $package, $name)
            }
        }
    };
}

#[derive(Clone, PartialEq, Message)]
pub struct ApiRequest {
    #[prost(message, optional, tag = "1")]
    pub header: Option<RequestHeader>,
    #[prost(message, optional, tag = "2")]
    pub message: Option<prost_types::Any>,
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
    pub message: Option<prost_types::Any>,
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

#[derive(Clone, PartialEq, Message)]
pub struct GetVersion {}
kicad_name!(GetVersion, "kiapi.common.commands", "GetVersion");

#[derive(Clone, PartialEq, Message)]
pub struct GetVersionResponse {
    #[prost(message, optional, tag = "1")]
    pub version: Option<Version>,
}
kicad_name!(
    GetVersionResponse,
    "kiapi.common.commands",
    "GetVersionResponse"
);

#[derive(Clone, PartialEq, Message)]
pub struct Version {
    #[prost(string, tag = "4")]
    pub full_version: String,
}

pub const DOCUMENT_PCB: i32 = 3;

#[derive(Clone, PartialEq, Message)]
pub struct GetOpenDocuments {
    #[prost(int32, tag = "1")]
    pub document_type: i32,
}
kicad_name!(
    GetOpenDocuments,
    "kiapi.common.commands",
    "GetOpenDocuments"
);

#[derive(Clone, PartialEq, Message)]
pub struct GetOpenDocumentsResponse {
    #[prost(message, repeated, tag = "1")]
    pub documents: Vec<Document>,
}
kicad_name!(
    GetOpenDocumentsResponse,
    "kiapi.common.commands",
    "GetOpenDocumentsResponse"
);

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
pub struct ItemHeader {
    #[prost(message, optional, tag = "1")]
    pub document: Option<Document>,
}

#[derive(Clone, PartialEq, Message)]
pub struct Kiid {
    #[prost(string, tag = "1")]
    pub value: String,
}

#[derive(Clone, PartialEq, Message)]
pub struct GetNets {
    #[prost(message, optional, tag = "1")]
    pub board: Option<Document>,
}
kicad_name!(GetNets, "kiapi.board.commands", "GetNets");

#[derive(Clone, PartialEq, Message)]
pub struct NetsResponse {
    #[prost(message, repeated, tag = "1")]
    pub nets: Vec<Net>,
}
kicad_name!(NetsResponse, "kiapi.board.commands", "NetsResponse");

#[derive(Clone, PartialEq, Message)]
pub struct Net {
    #[prost(string, tag = "2")]
    pub name: String,
}

#[derive(Clone, PartialEq, Message)]
pub struct GetBoardEnabledLayers {
    #[prost(message, optional, tag = "1")]
    pub board: Option<Document>,
}
kicad_name!(
    GetBoardEnabledLayers,
    "kiapi.board.commands",
    "GetBoardEnabledLayers"
);

#[derive(Clone, PartialEq, Message)]
pub struct BoardEnabledLayersResponse {
    #[prost(uint32, tag = "1")]
    pub copper_layer_count: u32,
}
kicad_name!(
    BoardEnabledLayersResponse,
    "kiapi.board.commands",
    "BoardEnabledLayersResponse"
);

pub const OBJECT_PAD: i32 = 2;
pub const OBJECT_TRACK: i32 = 11;

#[derive(Clone, PartialEq, Message)]
pub struct GetItems {
    #[prost(message, optional, tag = "1")]
    pub header: Option<ItemHeader>,
    #[prost(int32, repeated, tag = "2")]
    pub types: Vec<i32>,
}
kicad_name!(GetItems, "kiapi.common.commands", "GetItems");

#[derive(Clone, PartialEq, Message)]
pub struct GetItemsResponse {
    #[prost(int32, tag = "2")]
    pub status: i32,
    #[prost(message, repeated, tag = "3")]
    pub items: Vec<prost_types::Any>,
}
kicad_name!(
    GetItemsResponse,
    "kiapi.common.commands",
    "GetItemsResponse"
);

#[derive(Clone, PartialEq, Message)]
pub struct Point {
    #[prost(int64, tag = "1")]
    pub x_nm: i64,
    #[prost(int64, tag = "2")]
    pub y_nm: i64,
}

#[derive(Clone, PartialEq, Message)]
pub struct Distance {
    #[prost(int64, tag = "1")]
    pub value_nm: i64,
}

#[derive(Clone, PartialEq, Message)]
pub struct Pad {
    #[prost(message, optional, tag = "1")]
    pub id: Option<Kiid>,
    #[prost(string, tag = "3")]
    pub number: String,
    #[prost(message, optional, tag = "4")]
    pub net: Option<Net>,
    #[prost(message, optional, tag = "7")]
    pub position: Option<Point>,
}
kicad_name!(Pad, "kiapi.board.types", "Pad");

pub const LAYER_F_CU: i32 = 3;

#[derive(Clone, PartialEq, Message)]
pub struct Track {
    #[prost(message, optional, tag = "1")]
    pub id: Option<Kiid>,
    #[prost(message, optional, tag = "2")]
    pub start: Option<Point>,
    #[prost(message, optional, tag = "3")]
    pub end: Option<Point>,
    #[prost(message, optional, tag = "4")]
    pub width: Option<Distance>,
    #[prost(int32, tag = "6")]
    pub layer: i32,
    #[prost(message, optional, tag = "7")]
    pub net: Option<Net>,
}
kicad_name!(Track, "kiapi.board.types", "Track");

pub const OBJECT_VIA: i32 = 12;
pub const LAYER_B_CU: i32 = 34;
pub const VIA_THROUGH: i32 = 1;
pub const PADSTACK_SAME_ON_ALL_LAYERS: i32 = 1;
pub const SHAPE_CIRCLE: i32 = 1;

#[derive(Clone, PartialEq, Message)]
pub struct Via {
    #[prost(message, optional, tag = "1")]
    pub id: Option<Kiid>,
    #[prost(message, optional, tag = "2")]
    pub position: Option<Point>,
    #[prost(message, optional, tag = "3")]
    pub pad_stack: Option<PadStack>,
    #[prost(message, optional, tag = "5")]
    pub net: Option<Net>,
    #[prost(int32, tag = "6")]
    pub via_type: i32,
}
kicad_name!(Via, "kiapi.board.types", "Via");

#[derive(Clone, PartialEq, Message)]
pub struct PadStack {
    #[prost(int32, tag = "1")]
    pub stack_type: i32,
    #[prost(int32, repeated, tag = "2")]
    pub layers: Vec<i32>,
    #[prost(message, optional, tag = "3")]
    pub drill: Option<Drill>,
    #[prost(message, repeated, tag = "5")]
    pub copper_layers: Vec<CopperShape>,
}

#[derive(Clone, PartialEq, Message)]
pub struct Drill {
    #[prost(int32, tag = "1")]
    pub start_layer: i32,
    #[prost(int32, tag = "2")]
    pub end_layer: i32,
    #[prost(message, optional, tag = "3")]
    pub diameter: Option<Point>,
}

#[derive(Clone, PartialEq, Message)]
pub struct CopperShape {
    #[prost(int32, tag = "1")]
    pub layer: i32,
    #[prost(int32, tag = "2")]
    pub shape: i32,
    #[prost(message, optional, tag = "3")]
    pub size: Option<Point>,
}

#[derive(Clone, PartialEq, Message)]
pub struct GetPadShapeAsPolygon {
    #[prost(message, optional, tag = "1")]
    pub board: Option<Document>,
    #[prost(message, repeated, tag = "2")]
    pub pads: Vec<Kiid>,
    #[prost(int32, tag = "3")]
    pub layer: i32,
}
kicad_name!(
    GetPadShapeAsPolygon,
    "kiapi.board.commands",
    "GetPadShapeAsPolygon"
);

#[derive(Clone, PartialEq, Message)]
pub struct PadShapeAsPolygonResponse {
    #[prost(message, repeated, tag = "1")]
    pub pads: Vec<Kiid>,
    #[prost(message, repeated, tag = "2")]
    pub polygons: Vec<PolygonWithHoles>,
}
kicad_name!(
    PadShapeAsPolygonResponse,
    "kiapi.board.commands",
    "PadShapeAsPolygonResponse"
);

#[derive(Clone, PartialEq, Message)]
pub struct PolygonWithHoles {
    #[prost(message, optional, tag = "1")]
    pub outline: Option<PolyLine>,
}

#[derive(Clone, PartialEq, Message)]
pub struct PolyLine {
    #[prost(message, repeated, tag = "1")]
    pub nodes: Vec<PolyLineNode>,
}

#[derive(Clone, PartialEq, Message)]
pub struct PolyLineNode {
    #[prost(message, optional, tag = "1")]
    pub point: Option<Point>,
    #[prost(message, optional, tag = "2")]
    pub arc: Option<Arc>,
}

/// Only whether a node is an arc matters to the spike, not its points.
#[derive(Clone, PartialEq, Message)]
pub struct Arc {}

#[derive(Clone, PartialEq, Message)]
pub struct BeginCommit {
    #[prost(message, optional, tag = "1")]
    pub header: Option<ItemHeader>,
}
kicad_name!(BeginCommit, "kiapi.common.commands", "BeginCommit");

#[derive(Clone, PartialEq, Message)]
pub struct BeginCommitResponse {
    #[prost(message, optional, tag = "1")]
    pub id: Option<Kiid>,
}
kicad_name!(
    BeginCommitResponse,
    "kiapi.common.commands",
    "BeginCommitResponse"
);

pub const COMMIT_PUSH: i32 = 1;

#[derive(Clone, PartialEq, Message)]
pub struct EndCommit {
    #[prost(message, optional, tag = "1")]
    pub id: Option<Kiid>,
    #[prost(int32, tag = "2")]
    pub action: i32,
    #[prost(string, tag = "3")]
    pub message: String,
    #[prost(message, optional, tag = "4")]
    pub header: Option<ItemHeader>,
}
kicad_name!(EndCommit, "kiapi.common.commands", "EndCommit");

#[derive(Clone, PartialEq, Message)]
pub struct EndCommitResponse {}
kicad_name!(
    EndCommitResponse,
    "kiapi.common.commands",
    "EndCommitResponse"
);

#[derive(Clone, PartialEq, Message)]
pub struct CreateItems {
    #[prost(message, optional, tag = "1")]
    pub header: Option<ItemHeader>,
    #[prost(message, repeated, tag = "2")]
    pub items: Vec<prost_types::Any>,
}
kicad_name!(CreateItems, "kiapi.common.commands", "CreateItems");

#[derive(Clone, PartialEq, Message)]
pub struct CreateItemsResponse {
    #[prost(int32, tag = "2")]
    pub status: i32,
    #[prost(message, repeated, tag = "3")]
    pub created_items: Vec<ItemResult>,
}
kicad_name!(
    CreateItemsResponse,
    "kiapi.common.commands",
    "CreateItemsResponse"
);

pub const ITEM_OK: i32 = 1;

#[derive(Clone, PartialEq, Message)]
pub struct ItemResult {
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

#[derive(Clone, PartialEq, Message)]
pub struct RunAction {
    #[prost(string, tag = "1")]
    pub action: String,
}
kicad_name!(RunAction, "kiapi.common.commands", "RunAction");

pub const ACTION_SUBMITTED: i32 = 1;

#[derive(Clone, PartialEq, Message)]
pub struct RunActionResponse {
    #[prost(int32, tag = "1")]
    pub status: i32,
}
kicad_name!(
    RunActionResponse,
    "kiapi.common.commands",
    "RunActionResponse"
);

#[derive(Clone, PartialEq, Message)]
pub struct GetNetClassForNets {
    #[prost(message, repeated, tag = "1")]
    pub nets: Vec<Net>,
}
kicad_name!(
    GetNetClassForNets,
    "kiapi.board.commands",
    "GetNetClassForNets"
);

#[derive(Clone, PartialEq, Message)]
pub struct NetClassForNetsResponse {
    #[prost(map = "string, message", tag = "1")]
    pub classes: std::collections::HashMap<String, NetClass>,
}
kicad_name!(
    NetClassForNetsResponse,
    "kiapi.board.commands",
    "NetClassForNetsResponse"
);

#[derive(Clone, PartialEq, Message)]
pub struct NetClass {
    #[prost(string, tag = "1")]
    pub name: String,
    #[prost(message, optional, tag = "3")]
    pub board: Option<NetClassRules>,
}

#[derive(Clone, PartialEq, Message)]
pub struct NetClassRules {
    #[prost(message, optional, tag = "1")]
    pub clearance: Option<Distance>,
    #[prost(message, optional, tag = "2")]
    pub track_width: Option<Distance>,
    #[prost(message, optional, tag = "6")]
    pub via_stack: Option<PadStack>,
}
