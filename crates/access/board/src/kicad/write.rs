use super::board::Board;
use std::fmt::Write;
use traxide_contracts::{LayerId, NetId, Routes};

/// The board file `text`, read as `board`, with `routes` added as KiCad's
/// segments and vias before its closing parenthesis; every other byte stays.
pub(crate) fn with_routes(text: &str, board: &Board, routes: &Routes) -> String {
    let body = text.trim_end();
    let body = body.strip_suffix(')').unwrap_or(body);
    let mut added = String::new();
    let mut ids = Ids::new(text);
    for track in &routes.tracks {
        for pair in track.path.windows(2) {
            let _ = writeln!(
                added,
                "\t(segment (start {} {}) (end {} {}) (width {}) (layer {}) {} (uuid \"{}\"))",
                millimetres(pair[0].x),
                millimetres(pair[0].y),
                millimetres(pair[1].x),
                millimetres(pair[1].y),
                millimetres(track.width),
                quoted(&layer_name(board, track.layer)),
                net(board, track.net),
                ids.next(),
            );
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
        let _ = writeln!(
            added,
            "\t(via{} (at {} {}) (size {}) (drill {}) (layers {} {}) {} (uuid \"{}\"))",
            if through { "" } else { " blind" },
            millimetres(via.position.x),
            millimetres(via.position.y),
            millimetres(size.diameter),
            millimetres(size.drill),
            quoted(&layer_name(board, top)),
            quoted(&layer_name(board, bottom)),
            net(board, via.net),
            ids.next(),
        );
    }
    format!("{body}{added})\n")
}

fn layer_name(board: &Board, layer: LayerId) -> String {
    board.design.layers[layer.index()].name.clone()
}

/// `(net "name")` as KiCad 10 writes it, or `(net 3)` as KiCad 9 does.
fn net(board: &Board, net: NetId) -> String {
    let name = &board.design.net(net).name;
    match board
        .net_numbers
        .as_ref()
        .and_then(|numbers| numbers.get(name))
    {
        Some(number) => format!("(net {number})"),
        None => format!("(net {})", quoted(name)),
    }
}

fn quoted(text: &str) -> String {
    format!("\"{}\"", text.replace('\\', "\\\\").replace('"', "\\\""))
}

/// Nanometres as millimetres, exactly and without trailing zeros.
fn millimetres(nanometres: i32) -> String {
    let sign = if nanometres < 0 { "-" } else { "" };
    let magnitude = nanometres.unsigned_abs();
    let fraction = format!("{:06}", magnitude % 1_000_000);
    let fraction = fraction.trim_end_matches('0');
    if fraction.is_empty() {
        format!("{sign}{}", magnitude / 1_000_000)
    } else {
        format!("{sign}{}.{fraction}", magnitude / 1_000_000)
    }
}

/// UUIDs for the added items, the same for the same board each time, so
/// that routing a board twice writes the same file.
struct Ids {
    state: u64,
}

impl Ids {
    fn new(text: &str) -> Self {
        // FNV-1a of the board.
        let state = text.bytes().fold(0xcbf2_9ce4_8422_2325, |hash: u64, byte| {
            (hash ^ u64::from(byte)).wrapping_mul(0x0100_0000_01b3)
        });
        Self { state }
    }

    /// A version 4 UUID from the next 128 bits of a SplitMix64 sequence.
    fn next(&mut self) -> String {
        let mut half = || {
            self.state = self.state.wrapping_add(0x9e37_79b9_7f4a_7c15);
            let mut z = self.state;
            z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
            z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
            z ^ (z >> 31)
        };
        let (high, low) = (half(), half());
        let bits = (u128::from(high) << 64 | u128::from(low)) & !(0xf000 << 64) | (0x4000 << 64);
        let bits = bits & !(0xc000_0000_0000_0000) | 0x8000_0000_0000_0000;
        let hex = format!("{bits:032x}");
        format!(
            "{}-{}-{}-{}-{}",
            &hex[0..8],
            &hex[8..12],
            &hex[12..16],
            &hex[16..20],
            &hex[20..32]
        )
    }
}

#[cfg(test)]
mod tests {
    use super::super::board::read;
    use super::*;
    use traxide_contracts::geometry::Point;
    use traxide_contracts::{Track, Via, ViaKindId};

    #[test]
    fn millimetres_are_exact_and_short() {
        assert_eq!(millimetres(1_270_001), "1.270001");
        assert_eq!(millimetres(-500_000), "-0.5");
        assert_eq!(millimetres(0), "0");
        assert_eq!(millimetres(30_000_000), "30");
    }

    #[test]
    fn names_are_quoted_with_their_quotes_escaped() {
        assert_eq!(quoted(r#"a"b\c"#), r#""a\"b\\c""#);
    }

    #[test]
    fn uuids_are_version_4_and_the_same_for_the_same_board() {
        let mut ids = Ids::new("board");
        let first = ids.next();
        assert_ne!(first, ids.next());
        assert_eq!(first, Ids::new("board").next());
        assert_eq!(first.len(), 36);
        assert_eq!(&first[14..15], "4");
        assert!(matches!(&first[19..20], "8" | "9" | "a" | "b"), "{first}");
    }

    #[test]
    fn kicad_9_boards_get_their_nets_by_number() {
        let text = r#"(kicad_pcb (version 20241229)
            (layers (0 "F.Cu" signal) (2 "B.Cu" signal))
            (net 0 "") (net 1 "GND")
            (gr_rect (start 0 0) (end 10 10) (layer "Edge.Cuts"))
            (segment (start 1 1) (end 2 1) (width 0.2) (layer "F.Cu") (net 1)))"#;
        let board = read(text, None).expect("valid");
        let routes = Routes {
            tracks: vec![Track {
                net: NetId::from_index(0),
                layer: LayerId::from_index(1),
                width: 250_000,
                path: vec![Point::new(0, 0), Point::new(1_000_000, 0)],
            }],
            vias: vec![Via {
                net: NetId::from_index(0),
                kind: ViaKindId::from_index(0),
                position: Point::new(1_000_000, 0),
            }],
        };
        let written = with_routes(text, &board, &routes);
        assert!(
            written
                .contains(r#"(segment (start 0 0) (end 1 0) (width 0.25) (layer "B.Cu") (net 1)"#),
            "{written}"
        );
        assert!(
            written
                .contains(r#"(via (at 1 0) (size 0.6) (drill 0.3) (layers "F.Cu" "B.Cu") (net 1)"#),
            "{written}"
        );
        assert_eq!(
            read(&written, None)
                .expect("valid")
                .design
                .existing
                .tracks
                .len(),
            2
        );
    }
}
