use std::fmt;
use std::io;
use std::path::PathBuf;
use mlua::Error;

#[derive(Debug)]
pub enum ExplorerError {
    Io(std::io::Error),
    Config(String),
    OperationFailed(String),
    LuaError(mlua::Error),
    Other(String),
}

impl fmt::Display for ExplorerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ExplorerError::Io(err)                => write!(f, "I/O error: {}", err),
            ExplorerError::Config(msg)            => write!(f, "Configuration error: {}", msg),
            ExplorerError::OperationFailed(msg)    => write!(f, "Operation failed: {}", msg),
            ExplorerError::LuaError(err)          => write!(f, "Lua error: {}", err),
            ExplorerError::Other(msg)             => write!(f, "{}", msg),
        }
    }
}

impl From<io::Error> for ExplorerError {
    fn from(err: io::Error) -> Self {
        ExplorerError::Io(err)
    }
}

pub type Result<T> = std::result::Result<T, ExplorerError>;
