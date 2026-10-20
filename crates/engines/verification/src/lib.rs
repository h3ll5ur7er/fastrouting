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
    ConstraintSet, CopperItem, Design, KeepoutScope, NetId, Routes, Verdict, Violation,
};
use unions::Unions;

/// Checks `routes`, which are all the copper on the board besides the pads,
/// against `rules` compiled for verification.
pub fn verify(design: &Design, rules: &ConstraintSet, routes: &Routes) -> Verdict {
    let pieces = pieces::collect(design, routes);
    // A set: one violation per pair of items, however many of their segments conflict.
    let mut found = BTreeSet::new();
    let mut joined = Unions::new(pieces.len());
    let reach = rules.largest_clearance();
    let mut order: Vec<usize> = (0..pieces.len()).collect();
    order.sort_by_key(|&i| (pieces[i].layer, pieces[i].bounds.min.x, i));
    // Sweep along x: only pieces whose spans overlap, plus the reach of a clearance, can conflict.
    for (position, &i) in order.iter().enumerate() {
        let a = &pieces[i];
        let horizon = a.bounds.inflated(reach);
        for &j in order[position + 1..]
            .iter()
            .take_while(|&&j| pieces[j].layer == a.layer && pieces[j].bounds.min.x <= horizon.max.x)
        {
            let b = &pieces[j];
            if !b.bounds.intersects(horizon) {
                continue;
            }
            if a.net.is_some() && a.net == b.net {
                if a.shape.is_closer_than(&b.shape, 1) {
                    joined.join(i, j);
                }
            } else if !(a.is_pad() && b.is_pad()) {
                // Pads too close to each other are the footprint's business, not routing's.
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
        }
    }
    let mut violations: Vec<Violation> = found.into_iter().collect();
    violations.extend(edge_and_keepout_violations(design, rules, &pieces));
    pieces::join_layers_of_the_same_item(&pieces, &mut joined);
    violations.extend(unconnected(design, &pieces, &mut joined));
    Verdict { violations }
}

fn edge_and_keepout_violations(
    design: &Design,
    rules: &ConstraintSet,
    pieces: &[Piece],
) -> Vec<Violation> {
    let mut violations = BTreeSet::new();
    for piece in pieces.iter().filter(|p| !p.is_pad()) {
        if !piece
            .shape
            .is_inside(&design.outline, rules.edge_clearance())
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
