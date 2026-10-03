use crate::{contracts::ErrorCode, domain::Error};

pub(super) fn unavailable() -> Error {
    Error::new(
        ErrorCode::ExecutionFailed,
        "Application runtime unavailable",
    )
}
pub(super) fn failed() -> Error {
    Error::new(ErrorCode::ExecutionFailed, "Runtime command failed")
}
