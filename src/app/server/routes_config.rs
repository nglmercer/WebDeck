//! Legacy configuration adapters. Transactions serialize read-modify-write;
//! optional revision headers opt new clients into conflict detection.
use super::{v2, AppState};
use crate::app::utils::{
    merge_dicts::merge_dicts,
    settings::{
        check_config_update::check_config_update, create_folders::create_folders,
        get_config::get_config_path, gridsize::update_gridsize,
    },
};
use crate::application::config::{self, ConfigSnapshot};
use crate::domain::error::{AppError, ErrorCode};
use axum::{
    extract::State,
    http::HeaderMap,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::{json, Value};

pub(crate) fn dimension(value: &Value) -> Result<usize, AppError> {
    value
        .as_u64()
        .or_else(|| value.as_str()?.parse().ok())
        .filter(|n| (1..=128).contains(n))
        .map(|n| n as usize)
        .ok_or_else(|| AppError::new(ErrorCode::InvalidInput, "Invalid grid dimensions"))
}
fn revision(headers: &HeaderMap) -> Result<Option<u64>, AppError> {
    headers
        .get("x-webdeck-revision")
        .map(|h| {
            h.to_str().ok().and_then(|s| s.parse().ok()).ok_or_else(|| {
                AppError::new(ErrorCode::InvalidInput, "Invalid configuration revision")
            })
        })
        .transpose()
}
fn failed(error: AppError) -> Response {
    v2::error_response(error, None)
}
fn saved(snapshot: ConfigSnapshot) -> Response {
    Json(json!({"success":true,"revision":snapshot.revision})).into_response()
}

pub(crate) fn resize(mut candidate: Value, old: &Value) -> Result<Value, AppError> {
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

pub(crate) async fn transact(
    state: &AppState,
    expected: Option<u64>,
    transform: impl FnOnce(Value) -> Result<Value, AppError>,
) -> Result<ConfigSnapshot, AppError> {
    let mut folders = state.folders_to_create.lock().await;
    let service = config::shared(get_config_path())?;
    let mut old = Value::Null;
    let snapshot = service.update(expected, |value| {
        old = value.clone();
        let value = transform(value)?;
        Ok(create_folders(value, &folders))
    })?;
    // Keep queued folders on any failed persistence/conflict.
    folders.clear();
    drop(folders);
    publish(&snapshot, &old);
    Ok(snapshot)
}

pub(crate) async fn saveconfig(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(patch): Json<Value>,
) -> Response {
    let expected = match revision(&headers) {
        Ok(v) => v,
        Err(e) => return failed(e),
    };
    let result = transact(&state, expected, |old| {
        let mut next = merge_dicts(old.clone(), &patch);
        if let Some(raw) = next.pointer("/front/background").and_then(Value::as_str) {
            if let Ok(parsed) = serde_json::from_str::<Value>(
                &raw.replace("['", "[\"")
                    .replace("']", "\"]")
                    .replace("', '", "\",\"")
                    .replace("','", "\",\""),
            ) {
                next["front"]["background"] = parsed;
            }
        }
        Ok(check_config_update(resize(next, &old)?))
    })
    .await;
    match result {
        Ok(s) => saved(s),
        Err(e) => failed(e),
    }
}

pub(crate) async fn complete_save_config(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(next): Json<Value>,
) -> Response {
    let expected = match revision(&headers) {
        Ok(v) => v,
        Err(e) => return failed(e),
    };
    match transact(&state, expected, |old| resize(next, &old)).await {
        Ok(s) => saved(s),
        Err(e) => failed(e),
    }
}

pub(crate) async fn save_single_button(headers: HeaderMap, Json(data): Json<Value>) -> Response {
    let expected = match revision(&headers) {
        Ok(v) => v,
        Err(e) => return failed(e),
    };
    let index = |key: &str| {
        data.get(key)
            .and_then(|v| v.as_u64().or_else(|| v.as_str()?.parse().ok()))
            .map(|n| n as usize)
    };
    let (Some(folder), Some(index)) = (index("location_Folder"), index("location_Id")) else {
        return failed(AppError::new(
            ErrorCode::InvalidInput,
            "Invalid button location",
        ));
    };
    let Some(content) = data.get("content").filter(|v| v.is_object()).cloned() else {
        return failed(AppError::new(
            ErrorCode::InvalidInput,
            "Invalid button content",
        ));
    };
    let result = config::shared(get_config_path()).and_then(|service| {
        service.update(expected, |mut old| {
            let buttons = old
                .pointer_mut("/front/buttons")
                .and_then(Value::as_object_mut)
                .ok_or_else(|| AppError::new(ErrorCode::InvalidInput, "Invalid folder"))?;
            let (_, list) = buttons
                .iter_mut()
                .nth(folder)
                .ok_or_else(|| AppError::new(ErrorCode::InvalidInput, "Invalid folder"))?;
            let slot = list
                .as_array_mut()
                .and_then(|list| list.get_mut(index))
                .ok_or_else(|| AppError::new(ErrorCode::InvalidInput, "Invalid button"))?;
            *slot = content;
            Ok(old)
        })
    });
    match result {
        Ok(s) => {
            crate::app::utils::global_variables::set_global_variable("config", s.config.clone());
            saved(s)
        }
        Err(e) => failed(e),
    }
}

pub(crate) async fn save_buttons_only(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(patch): Json<Value>,
) -> Response {
    let expected = match revision(&headers) {
        Ok(v) => v,
        Err(e) => return failed(e),
    };
    match transact(&state, expected, |mut old| {
        if let Some(buttons) = patch.pointer("/front/buttons") {
            old["front"]["buttons"] = buttons.clone();
        }
        Ok(old)
    })
    .await
    {
        Ok(s) => saved(s),
        Err(e) => failed(e),
    }
}

pub(crate) async fn get_config_route(State(state): State<AppState>) -> Response {
    // Retain the legacy queue-flush behavior; v2 GET is a pure snapshot read.
    match transact(&state, None, Ok).await {
        Ok(s) => (
            [("x-webdeck-revision", s.revision.to_string())],
            Json(s.config),
        )
            .into_response(),
        Err(e) => failed(e),
    }
}

pub(crate) async fn create_folder(
    State(state): State<AppState>,
    Json(data): Json<Value>,
) -> Response {
    let name = data.get("name").and_then(Value::as_str).unwrap_or("");
    let parent = data
        .get("parent_folder")
        .and_then(Value::as_str)
        .unwrap_or("");
    if name.is_empty() || name.len() > 128 {
        return failed(AppError::new(
            ErrorCode::InvalidInput,
            "Invalid folder name",
        ));
    }
    let config = match config::shared(get_config_path()).and_then(|s| s.snapshot()) {
        Ok(s) => s.config,
        Err(e) => return failed(e),
    };
    let mut queue = state.folders_to_create.lock().await;
    if queue.iter().any(|item| item["name"] == name)
        || config
            .pointer("/front/buttons")
            .and_then(Value::as_object)
            .is_some_and(|map| map.contains_key(name))
    {
        return Json(json!({"success":false,"message":"Folder already exists"})).into_response();
    }
    queue.push(json!({"name":name,"parent_folder":parent}));
    Json(json!({"success":true})).into_response()
}
