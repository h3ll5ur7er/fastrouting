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
    #[error("{path} changed since it was read; route it again")]
    Changed { path: PathBuf },
    #[error("the board's custom rules, line {line}: {message}")]
    CustomRules { line: usize, message: String },
    #[error("the board's project: {0}")]
    Project(String),
}
