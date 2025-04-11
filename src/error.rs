use std::fmt;
use std::io;
use std::path::PathBuf;

#[derive(Debug)]
pub enum ExplorerError {
    IoError(io::Error),
    InvalidPath(PathBuf),
    OperationFailed(String),
}

impl fmt::Display for ExplorerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ExplorerError::IoError(err) => write!(f, "IO Error: {}", err),
            ExplorerError::InvalidPath(path) => write!(f, "Invalid path: {:?}", path),
            ExplorerError::OperationFailed(msg) => write!(f, "Operation failed: {}", msg),
        }
    }
}

impl From<io::Error> for ExplorerError {
    fn from(err: io::Error) -> Self {
        ExplorerError::IoError(err)
    }
}

pub type Result<T> = std::result::Result<T, ExplorerError>;
