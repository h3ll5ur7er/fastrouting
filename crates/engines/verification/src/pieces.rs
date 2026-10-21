use crate::unions::Unions;
use traxide_contracts::geometry::{Point, Rect, Segment, Shape};
use traxide_contracts::{CopperItem, Design, LayerId, NetId, PadId, Routes};

/// One shape of copper on one layer.
#[derive(Debug, Clone)]
pub(crate) struct Piece {
    pub(crate) layer: LayerId,
    pub(crate) net: Option<NetId>,
    pub(crate) shape: Shape,
    pub(crate) bounds: Rect,
    pub(crate) item: CopperItem,
}

impl Piece {
    fn new(layer: LayerId, net: Option<NetId>, shape: Shape, item: CopperItem) -> Self {
        Self {
            layer,
            net,
            bounds: shape.bounds(),
            shape,
            item,
        }
    }

    pub(crate) fn is_pad(&self) -> bool {
        matches!(self.item, CopperItem::Pad(_))
    }
}

pub(crate) fn collect(design: &Design, routes: &Routes) -> Vec<Piece> {
    let mut pieces = Vec::new();
    for (index, pad) in design.pads.iter().enumerate() {
        for copper in &pad.copper {
            pieces.push(Piece::new(
                copper.layer,
                pad.net,
                copper.shape.clone(),
                CopperItem::Pad(PadId::from_index(index)),
            ));
        }
    }
    for (index, track) in routes.tracks.iter().enumerate() {
        for pair in track.path.windows(2) {
            let shape = Shape::track(Segment::new(pair[0], pair[1]), track.width);
            pieces.push(Piece::new(
                track.layer,
                Some(track.net),
                shape,
                CopperItem::Track(index),
            ));
        }
    }
    for (index, via) in routes.vias.iter().enumerate() {
        let at = via.position;
        for copper in &design.via_kind(via.kind).copper {
            let shape = copper.shape.map(|p| Point::new(p.x + at.x, p.y + at.y));
            pieces.push(Piece::new(
                copper.layer,
                Some(via.net),
                shape,
                CopperItem::Via(index),
            ));
        }
    }
    pieces
}

/// The layers of one through-hole pad or via are connected by its barrel, and
/// the segments of one track by their shared points.
pub(crate) fn join_layers_of_the_same_item(pieces: &[Piece], joined: &mut Unions) {
    let mut by_item: Vec<usize> = (0..pieces.len()).collect();
    by_item.sort_by_key(|&i| pieces[i].item);
    for pair in by_item.windows(2) {
        if pieces[pair[0]].item == pieces[pair[1]].item {
            joined.join(pair[0], pair[1]);
        }
    }
}
