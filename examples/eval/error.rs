//! Errors of the eval tool. The text is fixed: it never carries a file name,
//! a path, a manifest line or the text of a lower-level error.

use std::fmt;

#[derive(Debug, PartialEq)]
pub enum EvalError {
    BadEntry { index: usize, reason: &'static str },
    BadToml { line: Option<usize> },
    EmptyManifest,
    CannotReadManifest,
    CannotReadImage { index: usize },
    DirInsideRepository,
    CannotReadDir,
}

impl fmt::Display for EvalError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EvalError::BadEntry { index, reason } => write!(f, "entry {index}: {reason}"),
            EvalError::BadToml { line: Some(line) } => {
                write!(f, "manifest: invalid TOML at line {line}")
            }
            EvalError::BadToml { line: None } => write!(f, "manifest: invalid TOML"),
            EvalError::EmptyManifest => write!(f, "manifest: no entries"),
            EvalError::CannotReadManifest => write!(f, "manifest: cannot be read"),
            EvalError::CannotReadImage { index } => {
                write!(f, "entry {index}: cannot read image")
            }
            EvalError::DirInsideRepository => write!(
                f,
                "DIR is inside the pixctl repository; keep private images outside it"
            ),
            EvalError::CannotReadDir => write!(f, "DIR cannot be read"),
        }
    }
}

impl std::error::Error for EvalError {}
