//! KiCad's own files: the board (`.kicad_pcb`) and the rules of its project
//! (`.kicad_pro`), as KiCad 9 and 10 write them.

mod board;
pub(crate) mod drc;
mod project;
mod sexpr;
mod write;

use crate::BoardError;
use traxide_contracts::{Design, Routes};

/// The design of the board in `board`, with the rules of its project, the text
/// of its `.kicad_pro`, if there is one.
pub fn parse_kicad_pcb(board: &str, project: Option<&str>) -> Result<Design, BoardError> {
    Ok(board::read(board, project)?.design)
}

/// The board file `board` with `routes` added, which were routed on `design`;
/// `None` if `board` is no longer the board `design` was read from.
pub fn add_routes(
    board: &str,
    project: Option<&str>,
    design: &Design,
    routes: &Routes,
) -> Result<Option<String>, BoardError> {
    let read = board::read(board, project)?;
    Ok((read.design == *design).then(|| write::with_routes(board, &read, routes)))
}
