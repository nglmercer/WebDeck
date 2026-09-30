//! Configuration transactions and settings publication owned by the application.
use crate::app::utils::settings::{get_config::get_config_path, gridsize::update_gridsize};
use crate::application::config::{self, ConfigSnapshot};
use crate::domain::error::{AppError, ErrorCode};
use serde_json::{json, Value};
pub(crate) fn dimension(value: &Value) -> Result<usize, AppError> {
    value
        .as_u64()
        .filter(|n| (1..=128).contains(n))
        .map(|n| n as usize)
        .ok_or_else(|| AppError::new(ErrorCode::InvalidInput, "Invalid grid dimensions"))
}
pub fn resize(mut candidate: Value, old: &Value) -> Result<Value, AppError> {
    let height = dimension(&candidate["front"]["height"])?;
    let width = dimension(&candidate["front"]["width"])?;
    if let Some(front) = candidate.get_mut("front").and_then(Value::as_object_mut) {
        front.insert("height".into(), old["front"]["height"].clone());
        front.insert("width".into(), old["front"]["width"].clone());
    }
    let mut candidate = update_gridsize(candidate, height, width);
    candidate["front"]["height"] = json!(height);
    candidate["front"]["width"] = json!(width);
    Ok(candidate)
}

fn publish(snapshot: &ConfigSnapshot, old: &Value) {
    crate::app::utils::global_variables::set_global_variable("config", snapshot.config.clone());
    crate::adapters::platform::settings::apply_changes(old, &snapshot.config);
}

pub fn transact(
    expected: u64,
    transform: impl FnOnce(Value) -> Result<Value, AppError>,
) -> Result<ConfigSnapshot, AppError> {
    let service = config::shared(get_config_path())?;
    let mut old = Value::Null;
    let snapshot = service.update(expected, |value| {
        old = value.clone();
        transform(value)
    })?;
    publish(&snapshot, &old);
    Ok(snapshot)
}
