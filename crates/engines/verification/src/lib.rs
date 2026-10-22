//! Verification Engine: how correctness is judged (volatility V9 of
//! docs/Architecture/Volatilities.md).
//!
//! It shares no clearance code with routing, only the geometry utility, so that a
//! bug in the router's checks cannot hide itself.

mod pieces;
mod unions;

use pieces::Piece;
use std::collections::BTreeSet;
use traxide_contracts::{
    ConstraintSet, CopperItem, Design, KeepoutScope, NetId, PadId, Routes, Verdict, Violation,
};
use unions::Unions;

/// Checks `routes`, which are all the copper on the board besides the pads,
/// against `rules` compiled for verification.
pub fn verify(design: &Design, rules: &ConstraintSet, routes: &Routes) -> Verdict {
    let pieces = pieces::collect(design, routes);
    // A set: one violation per pair of items, however many of their segments conflict.
    let mut found = BTreeSet::new();
    let mut joined = Unions::new(pieces.len());
    for_each_pair_within(&pieces, rules.largest_clearance(), |i, j| {
        let (a, b) = (&pieces[i], &pieces[j]);
        if a.net.is_some() && a.net == b.net {
            if a.shape.is_closer_than(&b.shape, 1) {
                joined.join(i, j);
            }
        } else if !(a.is_pad() && b.is_pad()) {
            // Pads too close to each other are the footprint's business, not
            // routing's; pads_too_close names them.
            let required = rules.clearance(a.net, b.net);
            if a.shape.is_closer_than(&b.shape, required) {
                let (first, second) = if a.item <= b.item {
                    (a.item, b.item)
                } else {
                    (b.item, a.item)
                };
                found.insert(Violation::Clearance {
                    layer: a.layer,
                    a: first,
                    b: second,
                    required,
                });
            }
        }
    });
    let mut violations: Vec<Violation> = found.into_iter().collect();
    violations.extend(edge_and_keepout_violations(design, rules, &pieces));
    pieces::join_layers_of_the_same_item(&pieces, &mut joined);
    pieces::join_through_planes(design, &pieces, &mut joined);
    violations.extend(unconnected(design, &pieces, &mut joined));
    Verdict { violations }
}

/// The pairs of pads of different nets closer than their clearance, on any
/// layer, each in id order: connecting one of them breaks a rule, and where
/// they overlap, shorts the other. Pads of one part count only where they
/// overlap, as a footprint may set its pins closer by its own rules.
pub fn pads_too_close(design: &Design, rules: &ConstraintSet) -> Vec<[PadId; 2]> {
    let pads: Vec<Piece> = pieces::collect(design, &Routes::default());
    let mut found = BTreeSet::new();
    for_each_pair_within(&pads, rules.largest_clearance(), |i, j| {
        let (a, b) = (&pads[i], &pads[j]);
        let (CopperItem::Pad(first), CopperItem::Pad(second)) = (a.item, b.item) else {
            return;
        };
        let required = if design.pad(first).part() == design.pad(second).part() {
            1
        } else {
            rules.clearance(a.net, b.net)
        };
        if a.net.is_some()
            && b.net.is_some()
            && a.net != b.net
            && a.shape.is_closer_than(&b.shape, required)
        {
            found.insert([first.min(second), first.max(second)]);
        }
    });
    found.into_iter().collect()
}

/// Calls `visit` with each pair of pieces on one layer whose bounds lie
/// within `reach` of each other, in a sweep along x.
fn for_each_pair_within(pieces: &[Piece], reach: i32, mut visit: impl FnMut(usize, usize)) {
    let mut order: Vec<usize> = (0..pieces.len()).collect();
    order.sort_by_key(|&i| (pieces[i].layer, pieces[i].bounds.min.x, i));
    for (position, &i) in order.iter().enumerate() {
        let a = &pieces[i];
        let horizon = a.bounds.inflated(reach);
        for &j in order[position + 1..]
            .iter()
            .take_while(|&&j| pieces[j].layer == a.layer && pieces[j].bounds.min.x <= horizon.max.x)
        {
            if pieces[j].bounds.intersects(horizon) {
                visit(i, j);
            }
        }
    }
}

fn edge_and_keepout_violations(
    design: &Design,
    rules: &ConstraintSet,
    pieces: &[Piece],
) -> Vec<Violation> {
    let mut violations = BTreeSet::new();
    for piece in pieces.iter().filter(|p| !p.is_pad()) {
        if !design
            .outline
            .holds_apart(&piece.shape, rules.edge_clearance(), |hole| {
                rules.cutout_clearance(hole)
            })
        {
            violations.insert(Violation::Edge {
                layer: piece.layer,
                item: piece.item,
            });
        }
        let blocked_by = |scope: KeepoutScope| match (scope, piece.item) {
            (KeepoutScope::Everything, _) => true,
            (KeepoutScope::TracksOnly, item) => matches!(item, CopperItem::Track(_)),
            (KeepoutScope::ViasOnly, item) => matches!(item, CopperItem::Via(_)),
        };
        let hits_keepout = design.keepouts.iter().any(|k| {
            k.layer.is_none_or(|l| l == piece.layer)
                && blocked_by(k.scope)
                && piece.shape.is_closer_than(&k.shape, 0)
        });
        if hits_keepout {
            violations.insert(Violation::Keepout {
                layer: piece.layer,
                item: piece.item,
            });
        }
    }
    violations.into_iter().collect()
}

fn unconnected(design: &Design, pieces: &[Piece], joined: &mut Unions) -> Vec<Violation> {
    let piece_of_pad = |pad| pieces.iter().position(|p| p.item == CopperItem::Pad(pad));
    let mut violations = Vec::new();
    for (index, net) in design.nets.iter().enumerate() {
        let Some((&first, rest)) = net.pads.split_first() else {
            continue;
        };
        let Some(anchor) = piece_of_pad(first) else {
            continue;
        };
        let apart: Vec<_> = rest
            .iter()
            .copied()
            .filter(|&pad| piece_of_pad(pad).is_none_or(|p| !joined.same(anchor, p)))
            .collect();
        if !apart.is_empty() {
            violations.push(Violation::Unconnected {
                net: NetId::from_index(index),
                pads: apart,
            });
        }
    }
    violations
}
