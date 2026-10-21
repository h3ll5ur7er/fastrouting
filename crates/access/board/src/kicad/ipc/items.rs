//! Routes as the items KiCad's API creates: a track per straight piece, a via
//! with its padstack.

use super::super::board::Board;
use super::messages::{
    Any, Distance, Drill, LAYER_BACK, LAYER_FRONT, Net, PADSTACK_NORMAL, PadStack, PadStackLayer,
    SHAPE_CIRCLE, Track, VIA_BLIND_BURIED, VIA_THROUGH, Vector2, Via,
};
use crate::BoardError;
use traxide_contracts::geometry::Point;
use traxide_contracts::{LayerId, NetId, Routes};

/// The tracks, then the vias, of `routes` on `board`.
pub(super) fn of(board: &Board, routes: &Routes) -> Result<Vec<Any>, BoardError> {
    let mut items = Vec::new();
    for track in &routes.tracks {
        let layer = layer_number(board, track.layer)?;
        for pair in track.path.windows(2) {
            items.push(Any::of(&Track {
                start: Some(vector(pair[0])),
                end: Some(vector(pair[1])),
                width: Some(distance(track.width)),
                layer,
                net: Some(net(board, track.net)),
            }));
        }
    }
    for via in &routes.vias {
        let kind = board.design.via_kind(via.kind);
        let size = board.via_sizes[via.kind.index()];
        let (top, bottom) = (
            kind.copper[0].layer,
            kind.copper[kind.copper.len() - 1].layer,
        );
        let through = top.index() == 0 && bottom.index() == board.design.layers.len() - 1;
        let (top, bottom) = (layer_number(board, top)?, layer_number(board, bottom)?);
        let round = |diameter| Some(vector(Point::new(diameter, diameter)));
        items.push(Any::of(&Via {
            position: Some(vector(via.position)),
            pad_stack: Some(PadStack {
                stack_type: PADSTACK_NORMAL,
                layers: vec![top, bottom],
                drill: Some(Drill {
                    start_layer: top,
                    end_layer: bottom,
                    diameter: round(size.drill),
                }),
                // A padstack the same on every layer keeps its shape under the front's.
                copper_layers: vec![PadStackLayer {
                    layer: LAYER_FRONT,
                    shape: SHAPE_CIRCLE,
                    size: round(size.diameter),
                }],
            }),
            net: Some(net(board, via.net)),
            via_type: if through {
                VIA_THROUGH
            } else {
                VIA_BLIND_BURIED
            },
        }));
    }
    Ok(items)
}

/// KiCad's number for the copper layer: the front, `In1.Cu` to `In30.Cu`, the back.
fn layer_number(board: &Board, layer: LayerId) -> Result<i32, BoardError> {
    let name = &board.design.layers[layer.index()].name;
    let inner = name
        .strip_prefix("In")
        .and_then(|n| n.strip_suffix(".Cu"))
        .and_then(|n| n.parse::<i32>().ok())
        .filter(|n| (1..LAYER_BACK - LAYER_FRONT).contains(n));
    match (name.as_str(), inner) {
        ("F.Cu", _) => Ok(LAYER_FRONT),
        ("B.Cu", _) => Ok(LAYER_BACK),
        (_, Some(inner)) => Ok(LAYER_FRONT + inner),
        _ => Err(BoardError::Kicad(format!(
            "KiCad has no copper layer named {name}"
        ))),
    }
}

fn net(board: &Board, net: NetId) -> Net {
    Net {
        name: board.design.net(net).name.clone(),
    }
}

fn vector(point: Point) -> Vector2 {
    Vector2 {
        x_nm: point.x.into(),
        y_nm: point.y.into(),
    }
}

fn distance(nanometres: i32) -> Distance {
    Distance {
        value_nm: nanometres.into(),
    }
}

#[cfg(test)]
mod tests {
    use super::super::super::board;
    use super::*;
    use traxide_contracts::{Track as Route, Via as RouteVia, ViaKindId};

    /// A 4-layer board with one net and the default class's through via.
    fn four_layers() -> Board {
        let text = r#"(kicad_pcb (version 20260206)
            (layers (0 "F.Cu" signal) (4 "In1.Cu" signal) (6 "In2.Cu" signal) (2 "B.Cu" signal))
            (gr_rect (start 0 0) (end 10 10) (layer "Edge.Cuts"))
            (footprint "R" (at 5 5) (property "Reference" "R1")
              (pad "1" smd rect (at 0 0) (size 1 1) (layers "F.Cu") (net "GND"))))"#;
        board::read(text, None, None).expect("a valid board")
    }

    fn decoded<M: prost::Message + prost::Name + Default>(item: &Any) -> M {
        item.to::<M>().expect("the item's type")
    }

    #[test]
    fn a_track_becomes_one_kicad_track_per_straight_piece() {
        let board = four_layers();
        let routes = Routes {
            tracks: vec![Route {
                net: NetId::from_index(0),
                layer: LayerId::from_index(2),
                width: 250_000,
                path: vec![
                    Point::new(0, 0),
                    Point::new(1_000, 0),
                    Point::new(1_000, -500),
                ],
            }],
            vias: Vec::new(),
        };
        let items = of(&board, &routes).expect("convertible");
        let tracks: Vec<Track> = items.iter().map(decoded).collect();
        assert_eq!(
            tracks[1],
            Track {
                start: Some(Vector2 {
                    x_nm: 1_000,
                    y_nm: 0
                }),
                end: Some(Vector2 {
                    x_nm: 1_000,
                    y_nm: -500
                }),
                width: Some(Distance { value_nm: 250_000 }),
                layer: LAYER_FRONT + 2,
                net: Some(Net { name: "GND".into() }),
            }
        );
        assert_eq!(tracks.len(), 2);
    }

    #[test]
    fn a_via_carries_its_classes_size_through_the_board() {
        let board = four_layers();
        let routes = Routes {
            tracks: Vec::new(),
            vias: vec![RouteVia {
                net: NetId::from_index(0),
                kind: ViaKindId::from_index(0),
                position: Point::new(2_000_000, 3_000_000),
            }],
        };
        let via: Via = decoded(&of(&board, &routes).expect("convertible")[0]);
        assert_eq!(via.via_type, VIA_THROUGH);
        let stack = via.pad_stack.expect("a padstack");
        assert_eq!(stack.layers, [LAYER_FRONT, LAYER_BACK]);
        let drill = stack.drill.expect("a drill").diameter.expect("a diameter");
        assert_eq!(drill.x_nm, 300_000, "KiCad's default via drill");
        let copper = stack.copper_layers[0].size.expect("a size");
        assert_eq!(copper.x_nm, 600_000, "KiCad's default via diameter");
    }

    #[test]
    fn layers_take_kicads_numbers() {
        let board = four_layers();
        let numbers: Vec<i32> = board
            .design
            .layer_ids()
            .map(|layer| layer_number(&board, layer).expect("a copper layer"))
            .collect();
        assert_eq!(numbers, [3, 4, 5, 34]);
    }
}
