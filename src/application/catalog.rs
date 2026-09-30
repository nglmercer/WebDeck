//! Startup publishes plugin metadata; transport reads never initialize plugins.
use crate::domain::error::{AppError, ErrorCode};
use serde_json::Value;
use std::sync::{OnceLock, RwLock};

fn state() -> &'static RwLock<Option<Value>> {
    static CATALOG: OnceLock<RwLock<Option<Value>>> = OnceLock::new();
    CATALOG.get_or_init(|| RwLock::new(None))
}

pub fn publish(commands: Value) {
    *state().write().unwrap_or_else(|p| p.into_inner()) = Some(commands);
}

pub fn snapshot() -> Result<Value, AppError> {
    if let Some(commands) = state().read().unwrap_or_else(|p| p.into_inner()).as_ref() {
        return Ok(commands.clone());
    }
    // Standalone router tests may not run desktop startup. Read only the
    // shipped metadata; never discover/evaluate plugins from a request.
    let bytes = std::fs::read("webdeck/commands.json")
        .map_err(|_| AppError::new(ErrorCode::PersistenceFailed, "Cannot read command catalog"))?;
    serde_json::from_slice(&bytes)
        .map_err(|_| AppError::new(ErrorCode::InvalidInput, "Invalid command catalog"))
}
