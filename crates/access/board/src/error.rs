use std::path::PathBuf;

#[derive(Debug, thiserror::Error)]
pub enum BoardError {
    #[error("cannot access {path}: {source}")]
    Io {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("{path}: unsupported format, expected {expected}")]
    UnsupportedFormat {
        path: PathBuf,
        expected: &'static str,
    },
    #[error("line {line}: {message}")]
    Syntax { line: usize, message: String },
    #[error("line {line}: {message}")]
    Invalid { line: usize, message: String },
}
