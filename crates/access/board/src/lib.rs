//! Board Access: where boards come from and where routes go (volatility V10 of
//! docs/Architecture/Volatilities.md). Callers see format-neutral contracts; the
//! file extension picks the format.

mod error;
pub mod kicad;
mod sexp;
pub mod specctra;

pub use error::BoardError;

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::SystemTime;
use traxide_contracts::{Design, Routes, Verdict};

pub fn load_design(path: &Path) -> Result<Design, BoardError> {
    let mut design = read_design(path)?;
    design.warnings.extend(parts_outside(&design));
    Ok(design)
}

/// Says which parts lie wholly outside the board's outline, whose pads no
/// track can reach: whole boards of the corpus were exported so.
fn parts_outside(design: &Design) -> Option<String> {
    let mut inside_or_not: BTreeMap<&str, bool> = BTreeMap::new();
    for pad in &design.pads {
        let part = pad
            .name
            .rsplit_once('-')
            .map_or(pad.name.as_str(), |(part, _)| part);
        *inside_or_not.entry(part).or_insert(false) |= design.outline.contains(pad.position);
    }
    let outside: Vec<&str> = inside_or_not
        .into_iter()
        .filter(|&(_, inside)| !inside)
        .map(|(part, _)| part)
        .collect();
    match outside.as_slice() {
        [] => None,
        [part] => Some(format!("the part {part} lies outside the board's outline")),
        parts => Some(format!(
            "the parts {} lie outside the board's outline",
            parts.join(", ")
        )),
    }
}

fn read_design(path: &Path) -> Result<Design, BoardError> {
    match extension(path) {
        Some("dsn") => specctra::parse_dsn(&read(path)?),
        Some("kicad_pcb") => {
            let beside = Beside::of(path)?;
            kicad::parse_kicad_pcb(&read(path)?, beside.rule_files())
        }
        _ => Err(BoardError::UnsupportedFormat {
            path: path.into(),
            expected: "a Specctra design (.dsn) or a KiCad board (.kicad_pcb)",
        }),
    }
}

/// Where the routes of the board at `board` go unless told otherwise: a session
/// beside a Specctra design, and a copy beside a KiCad board, which KiCad opens
/// with the board's project.
pub fn default_routes_path(board: &Path) -> PathBuf {
    match extension(board) {
        Some("kicad_pcb") => board.with_extension("routed.kicad_pcb"),
        _ => board.with_extension("ses"),
    }
}

/// Stores `routes`, the copper routing added to `design`, which was read from
/// `board`, into `path`.
pub fn store_routes(
    path: &Path,
    board: &Path,
    design: &Design,
    routes: &Routes,
) -> Result<(), BoardError> {
    let text = match (extension(path), extension(board)) {
        (Some("ses"), _) => {
            let name = path
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default();
            // KiCad's session import replaces every track of the board with the
            // session's: the session carries the board's own copper too, as
            // Freerouting's do.
            let mut everything = design.existing.clone();
            everything.extend(routes.clone());
            specctra::write_ses(&name, design, &everything)
        }
        (Some("kicad_pcb"), Some("kicad_pcb")) => {
            let beside = Beside::of(board)?;
            let text = kicad::add_routes(&read(board)?, beside.rule_files(), design, routes)?
                .ok_or_else(|| BoardError::Changed { path: board.into() })?;
            // KiCad finds a board's rules in the files of the same name.
            for (extension, rules) in [
                (PROJECT, &beside.project),
                (CUSTOM_RULES, &beside.custom_rules),
            ] {
                let copy = path.with_extension(extension);
                if let Some(rules) = rules
                    && copy != board.with_extension(extension)
                {
                    write(&copy, rules)?;
                }
            }
            text
        }
        _ => {
            return Err(BoardError::UnsupportedFormat {
                path: path.into(),
                expected: "a Specctra session (.ses), or a KiCad board (.kicad_pcb) for a KiCad board",
            });
        }
    };
    write(path, &text)
}

/// Stores `verdict`, found on `routes` of `design` in the file `source`, into
/// `path` as a DRC report in the JSON format of `kicad-cli pcb drc`.
pub fn store_drc_report(
    path: &Path,
    source: &Path,
    design: &Design,
    routes: &Routes,
    verdict: &Verdict,
) -> Result<(), BoardError> {
    let source = source
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    let report = kicad::drc::report(&source, SystemTime::now(), design, routes, verdict);
    write(path, &format!("{report:#}\n"))
}

fn write(path: &Path, text: &str) -> Result<(), BoardError> {
    std::fs::write(path, text).map_err(|source| BoardError::Io {
        path: path.into(),
        source,
    })
}

/// Reads routes that were stored for `design`.
pub fn load_routes(path: &Path, design: &Design) -> Result<Routes, BoardError> {
    match extension(path) {
        Some("ses") => specctra::parse_ses(&read(path)?, design),
        _ => Err(BoardError::UnsupportedFormat {
            path: path.into(),
            expected: "a Specctra session (.ses)",
        }),
    }
}

const PROJECT: &str = "kicad_pro";
const CUSTOM_RULES: &str = "kicad_dru";

/// The texts of the files next to a KiCad board that hold its rules.
struct Beside {
    project: Option<String>,
    custom_rules: Option<String>,
}

impl Beside {
    fn of(board: &Path) -> Result<Self, BoardError> {
        let text = |extension| {
            let path = board.with_extension(extension);
            path.is_file().then(|| read(&path)).transpose()
        };
        Ok(Self {
            project: text(PROJECT)?,
            custom_rules: text(CUSTOM_RULES)?,
        })
    }

    fn rule_files(&self) -> kicad::RuleFiles<'_> {
        kicad::RuleFiles {
            project: self.project.as_deref(),
            custom_rules: self.custom_rules.as_deref(),
        }
    }
}

fn extension(path: &Path) -> Option<&str> {
    path.extension().and_then(|e| e.to_str())
}

/// The file's text: UTF-8, as KiCad writes, or else Latin-1, as some older
/// tools do, which maps every byte to the character of the same number.
fn read(path: &Path) -> Result<String, BoardError> {
    let bytes = std::fs::read(path).map_err(|source| BoardError::Io {
        path: path.into(),
        source,
    })?;
    Ok(String::from_utf8(bytes)
        .unwrap_or_else(|error| error.into_bytes().iter().map(|&b| char::from(b)).collect()))
}
