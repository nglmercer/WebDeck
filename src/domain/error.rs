use serde::{Deserialize, Serialize};

/// Public codes deliberately exclude filesystem paths and command arguments.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ErrorCode {
    InvalidInput,
    UnsupportedSchema,
    Conflict,
    Unauthorized,
    Forbidden,
    CapacityExhausted,
    ShuttingDown,
    UnknownCommand,
    ExecutionFailed,
    PersistenceFailed,
}

#[derive(Debug, Clone)]
pub struct AppError {
    pub code: ErrorCode,
    pub message: &'static str,
}

impl AppError {
    pub fn new(code: ErrorCode, message: &'static str) -> Self {
        Self { code, message }
    }
}

impl std::fmt::Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.message)
    }
}

impl std::error::Error for AppError {}
