use traxide_contracts::geometry::{Point, Rect, Segment, Shape};
use traxide_contracts::{Design, LayerId, NetId};

/// For each pad, the smallest index of the pads that the board's own copper of
/// its net joins it to: pads with the same number need no connection between
/// them. Boards often come partly routed.
pub(crate) fn joined_pads(design: &Design) -> Vec<usize> {
    let mut pieces: Vec<Piece> = Vec::new();
    for (index, pad) in design.pads.iter().enumerate() {
        let Some(net) = pad.net else { continue };
        for copper in &pad.copper {
            pieces.push(Piece::new(
                Owner::Pad(index),
                net,
                copper.layer,
                copper.shape.clone(),
            ));
        }
    }
    let first_track = pieces.len();
    for (index, track) in design.existing.tracks.iter().enumerate() {
        for pair in track.path.windows(2) {
            let shape = Shape::track(Segment::new(pair[0], pair[1]), track.width);
            pieces.push(Piece::new(
                Owner::Track(index),
                track.net,
                track.layer,
                shape,
            ));
        }
    }
    for (index, via) in design.existing.vias.iter().enumerate() {
        let offset = via.position;
        for copper in &design.via_kind(via.kind).copper {
            let shape = copper
                .shape
                .map(|p| Point::new(p.x + offset.x, p.y + offset.y));
            pieces.push(Piece::new(Owner::Via(index), via.net, copper.layer, shape));
        }
    }
    let mut groups = Groups::new(pieces.len());
    // Pieces of one owner, such as the layers of a via, are joined already.
    for (a, b) in (1..pieces.len()).map(|i| (i - 1, i)) {
        if pieces[a].owner == pieces[b].owner {
            groups.join(a, b);
        }
    }
    // A sweep along x: only pieces whose spans overlap can touch. Only the
    // board's own copper joins pads, so each pair needs one piece of it.
    let mut by_left_edge: Vec<usize> = (0..pieces.len()).collect();
    by_left_edge.sort_by_key(|&i| pieces[i].bounds.min.x);
    for (k, &a) in by_left_edge.iter().enumerate() {
        for &b in &by_left_edge[k + 1..] {
            if pieces[b].bounds.min.x > pieces[a].bounds.max.x {
                break;
            }
            let is_existing = |i: usize| i >= first_track;
            if (is_existing(a) || is_existing(b)) && pieces[a].touches(&pieces[b]) {
                groups.join(a, b);
            }
        }
    }
    let mut representative = vec![usize::MAX; design.pads.len()];
    let mut first_pad_of_group = std::collections::HashMap::new();
    for (index, piece) in pieces.iter().enumerate() {
        if let Owner::Pad(pad) = piece.owner {
            let group = groups.root(index);
            let first = *first_pad_of_group.entry(group).or_insert(pad);
            representative[pad] = first;
        }
    }
    representative
        .into_iter()
        .enumerate()
        .map(|(pad, first)| if first == usize::MAX { pad } else { first })
        .collect()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Owner {
    Pad(usize),
    Track(usize),
    Via(usize),
}

struct Piece {
    owner: Owner,
    net: NetId,
    layer: LayerId,
    shape: Shape,
    bounds: Rect,
}

impl Piece {
    fn new(owner: Owner, net: NetId, layer: LayerId, shape: Shape) -> Self {
        Self {
            owner,
            net,
            layer,
            bounds: shape.bounds(),
            shape,
        }
    }

    fn touches(&self, other: &Self) -> bool {
        self.net == other.net
            && self.layer == other.layer
            && self.bounds.intersects(other.bounds)
            && self.shape.is_closer_than(&other.shape, 1)
    }
}

/// Union-find over pieces of copper.
struct Groups {
    parent: Vec<usize>,
}

impl Groups {
    fn new(count: usize) -> Self {
        Self {
            parent: (0..count).collect(),
        }
    }

    fn root(&mut self, mut item: usize) -> usize {
        while self.parent[item] != item {
            self.parent[item] = self.parent[self.parent[item]];
            item = self.parent[item];
        }
        item
    }

    fn join(&mut self, a: usize, b: usize) {
        let (a, b) = (self.root(a), self.root(b));
        self.parent[a.max(b)] = a.min(b);
    }
}
