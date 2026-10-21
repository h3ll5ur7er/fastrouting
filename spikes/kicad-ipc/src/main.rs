//! TRX-17: reads the board open in KiCad 10 through its IPC API and adds a track and a via in
//! one commit, then undoes it, from a native process without Python.
//!
//! `traxide-kicad-ipc-spike [--undo]`; the socket comes from `KICAD_API_SOCKET`.

mod client;
mod messages;
mod nng;

use anyhow::{Context, Result, ensure};
use client::Kicad;
use messages::*;
use prost::Message;

fn main() -> Result<()> {
    let undo = std::env::args().any(|a| a == "--undo");
    let mut kicad = Kicad::connect_from_environment()?;

    let version: GetVersionResponse = kicad.call(&GetVersion {})?;
    println!("KiCad {}", version.version.unwrap_or_default().full_version);

    let documents: GetOpenDocumentsResponse = kicad.call(&GetOpenDocuments {
        document_type: DOCUMENT_PCB,
    })?;
    let board = documents
        .documents
        .into_iter()
        .next()
        .context("no board is open")?;
    println!("board {}", board.board_filename.clone().unwrap_or_default());

    let layers: BoardEnabledLayersResponse = kicad.call(&GetBoardEnabledLayers {
        board: Some(board.clone()),
    })?;
    println!("copper layers {}", layers.copper_layer_count);

    let nets: NetsResponse = kicad.call(&GetNets {
        board: Some(board.clone()),
    })?;
    println!("nets {}", nets.nets.len());

    let pads: Vec<Pad> = items(&mut kicad, &board, OBJECT_PAD)?;
    println!("pads {}", pads.len());
    let pad = pads
        .iter()
        .find(|p| p.net.as_ref().is_some_and(|n| !n.name.is_empty()))
        .context("no pad on a net")?;
    let pad_net = pad.net.clone().unwrap_or_default();
    let pad_at = pad.position.clone().unwrap_or_default();
    println!(
        "pad {} of net {} at ({}, {}) nm",
        pad.number, pad_net.name, pad_at.x_nm, pad_at.y_nm
    );

    let classes: NetClassForNetsResponse = kicad.call(&GetNetClassForNets {
        nets: vec![pad_net.clone()],
    })?;
    for (net, class) in &classes.classes {
        let rules = class.board.clone().unwrap_or_default();
        let nm = |d: Option<Distance>| d.map(|d| d.value_nm);
        println!(
            "net {net}: class {} clearance {:?} nm, width {:?} nm, via {}",
            class.name,
            nm(rules.clearance),
            nm(rules.track_width),
            if rules.via_stack.is_some() {
                "given"
            } else {
                "missing"
            }
        );
    }

    let shape: PadShapeAsPolygonResponse = kicad.call(&GetPadShapeAsPolygon {
        board: Some(board.clone()),
        pads: pad.id.iter().cloned().collect(),
        layer: LAYER_F_CU,
    })?;
    for polygon in &shape.polygons {
        let nodes = &polygon.outline.clone().unwrap_or_default().nodes;
        let arcs = nodes.iter().filter(|n| n.arc.is_some()).count();
        println!("pad polygon: {} nodes, {arcs} arcs", nodes.len());
    }

    let copper_before = copper_count(&mut kicad, &board)?;
    let header = Some(ItemHeader {
        document: Some(board.clone()),
    });
    let commit: BeginCommitResponse = kicad.call(&BeginCommit {
        header: header.clone(),
    })?;
    let track_end = Point {
        x_nm: pad_at.x_nm + 2_540_000,
        y_nm: pad_at.y_nm,
    };
    let track = Track {
        start: Some(pad_at.clone()),
        end: Some(track_end.clone()),
        width: Some(Distance { value_nm: 250_000 }),
        layer: LAYER_F_CU,
        net: Some(pad_net.clone()),
        ..Track::default()
    };
    let created: CreateItemsResponse = kicad.call(&CreateItems {
        header: header.clone(),
        items: vec![
            prost_types::Any::from_msg(&track)?,
            prost_types::Any::from_msg(&through_via(track_end, pad_net))?,
        ],
    })?;
    for result in &created.created_items {
        let status = result.status.clone().unwrap_or_default();
        ensure!(
            status.code == ITEM_OK,
            "track refused: {}",
            status.error_message
        );
    }
    let _: EndCommitResponse = kicad.call(&EndCommit {
        id: commit.id,
        action: COMMIT_PUSH,
        message: "traxide: route".into(),
        header,
    })?;
    let copper_after = copper_count(&mut kicad, &board)?;
    println!("tracks and vias {copper_before:?} -> {copper_after:?} after one commit");
    ensure!(
        copper_after == (copper_before.0 + 1, copper_before.1 + 1),
        "the commit did not add exactly one track and one via"
    );

    if undo {
        // RunAction is documented as unstable and for prototyping only; the spike uses it to
        // play the user pressing Ctrl+Z.
        let undone: RunActionResponse = kicad.call(&RunAction {
            action: "common.Interactive.undo".into(),
        })?;
        ensure!(
            undone.status == ACTION_SUBMITTED,
            "undo was not accepted: {}",
            undone.status
        );
        std::thread::sleep(std::time::Duration::from_millis(500));
        let copper_undone = copper_count(&mut kicad, &board)?;
        println!("tracks and vias {copper_after:?} -> {copper_undone:?} after one undo");
        ensure!(
            copper_undone == copper_before,
            "one undo did not remove the commit"
        );
    }
    Ok(())
}

fn through_via(position: Point, net: Net) -> Via {
    let diameter = |nm| Some(Point { x_nm: nm, y_nm: nm });
    Via {
        position: Some(position),
        pad_stack: Some(PadStack {
            stack_type: PADSTACK_SAME_ON_ALL_LAYERS,
            layers: vec![LAYER_F_CU, LAYER_B_CU],
            drill: Some(Drill {
                start_layer: LAYER_F_CU,
                end_layer: LAYER_B_CU,
                diameter: diameter(300_000),
            }),
            copper_layers: vec![CopperShape {
                layer: LAYER_F_CU,
                shape: SHAPE_CIRCLE,
                size: diameter(600_000),
            }],
        }),
        net: Some(net),
        via_type: VIA_THROUGH,
        ..Via::default()
    }
}

fn copper_count(kicad: &mut Kicad, board: &Document) -> Result<(usize, usize)> {
    let tracks = items::<Track>(kicad, board, OBJECT_TRACK)?.len();
    let vias = items::<Via>(kicad, board, OBJECT_VIA)?.len();
    Ok((tracks, vias))
}

fn items<Item: Message + prost::Name + Default>(
    kicad: &mut Kicad,
    board: &Document,
    object_type: i32,
) -> Result<Vec<Item>> {
    let response: GetItemsResponse = kicad.call(&GetItems {
        header: Some(ItemHeader {
            document: Some(board.clone()),
        }),
        types: vec![object_type],
    })?;
    response
        .items
        .iter()
        .map(|any| Ok(any.to_msg::<Item>()?))
        .collect()
}
