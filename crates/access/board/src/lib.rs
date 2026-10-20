//! Board Access: where boards come from and where routes go (volatility V10 of
//! docs/Architecture/Volatilities.md). Callers see format-neutral contracts; the
//! file extension picks the format.

mod error;
pub mod specctra;

pub use error::BoardError;

use std::path::Path;
use traxide_contracts::{Design, Routes};

pub fn load_design(path: &Path) -> Result<Design, BoardError> {
    match extension(path) {
        Some("dsn") => specctra::parse_dsn(&read(path)?),
        _ => Err(BoardError::UnsupportedFormat {
            path: path.into(),
            expected: "a Specctra design (.dsn)",
        }),
    }
}

/// Writes `routes` for `design`, which must be the design they were routed on.
pub fn store_routes(path: &Path, design: &Design, routes: &Routes) -> Result<(), BoardError> {
    match extension(path) {
        Some("ses") => {
            let name = path
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default();
            let text = specctra::write_ses(&name, design, routes);
            std::fs::write(path, text).map_err(|source| BoardError::Io {
                path: path.into(),
                source,
            })
        }
        _ => Err(BoardError::UnsupportedFormat {
            path: path.into(),
            expected: "a Specctra session (.ses)",
        }),
    }
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
