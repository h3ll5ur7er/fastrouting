//! Global Routing Engine: the algorithm that decides roughly where each
//! connection goes before any exact copper is drawn (volatility V6 of
//! docs/Architecture/Volatilities.md).
//!
//! The board is divided into square cells a few tracks wide. Every connection
//! is routed from cell to cell, all of them together, with negotiated
//! congestion (PathFinder, McMurchie and Ebeling 1995): cells that more
//! connections want than they have room for grow more expensive with every
//! round, until the connections have shared them out. Each connection's cells,
//! widened by one, become its corridor.

mod capacity;
mod negotiation;

use capacity::Cells;
use traxide_contracts::{ConstraintSet, Corridors, Design, Plan, Profile};

/// Corridors for every connection of `plan`.
pub fn route(design: &Design, plan: &Plan, rules: &ConstraintSet, profile: &Profile) -> Corridors {
    let cells = Cells::of(design, rules);
    let paths = negotiation::negotiate(design, plan, rules, &cells, profile);
    let corridors = paths.iter().map(|path| cells.widened(path)).collect();
    Corridors::new(cells.origin(), cells.size(), cells.columns(), corridors)
}
