//! Legacy configuration adapters. Transactions serialize read-modify-write;
//! Every mutation requires a revision; reads never publish changes.
use super::v2;
use crate::app::utils::{
    merge_dicts::merge_dicts,
    settings::{
        check_config_update::check_config_update, create_folders::create_folders,
        get_config::get_config_path,
    },
};
use crate::application::config::{self, ConfigSnapshot};
use crate::application::layout::{resize, transact};
use crate::domain::error::{AppError, ErrorCode};
use axum::{
    http::HeaderMap,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::{json, Value};

fn revision(headers: &HeaderMap) -> Result<u64, AppError> {
    headers
        .get("x-webdeck-revision")
        .and_then(|h| h.to_str().ok())
        .and_then(|s| s.parse::<u64>().ok())
        .filter(|revision| *revision <= 9_007_199_254_740_991)
        .ok_or_else(|| {
            AppError::new(
                ErrorCode::InvalidInput,
                "A valid configuration revision is required",
            )
        })
}
fn failed(error: AppError) -> Response {
    v2::error_response(error, None)
}
fn saved(snapshot: ConfigSnapshot) -> Response {
    Json(json!({"success":true,"revision":snapshot.revision,"config":snapshot.config}))
        .into_response()
}

pub(crate) async fn saveconfig(headers: HeaderMap, Json(patch): Json<Value>) -> Response {
    let expected = match revision(&headers) {
        Ok(v) => v,
        Err(e) => return failed(e),
    };
    let result = transact(expected, |old| {
        let next = merge_dicts(old.clone(), &patch);
        Ok(check_config_update(resize(next, &old)?))
    });
    match result {
        Ok(s) => saved(s),
        Err(e) => failed(e),
    }
}

pub(crate) async fn complete_save_config(headers: HeaderMap, Json(next): Json<Value>) -> Response {
    let expected = match revision(&headers) {
        Ok(v) => v,
        Err(e) => return failed(e),
    };
    match transact(expected, |old| resize(next, &old)) {
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

pub(crate) async fn save_buttons_only(headers: HeaderMap, Json(patch): Json<Value>) -> Response {
    let expected = match revision(&headers) {
        Ok(v) => v,
        Err(e) => return failed(e),
    };
    match transact(expected, |mut old| {
        if let Some(buttons) = patch.pointer("/front/buttons") {
            old["front"]["buttons"] = buttons.clone();
        }
        Ok(old)
    }) {
        Ok(s) => saved(s),
        Err(e) => failed(e),
    }
}

pub(crate) async fn get_config_route() -> Response {
    match config::shared(get_config_path()).and_then(|service| service.snapshot()) {
        Ok(s) => (
            [("x-webdeck-revision", s.revision.to_string())],
            Json(s.config),
        )
            .into_response(),
        Err(e) => failed(e),
    }
}

pub(crate) async fn create_folder(headers: HeaderMap, Json(data): Json<Value>) -> Response {
    let expected = match revision(&headers) {
        Ok(v) => v,
        Err(e) => return failed(e),
    };
    let name = data.get("name").and_then(Value::as_str).unwrap_or("");
    let parent = data
        .get("parent_folder")
        .and_then(Value::as_str)
        .unwrap_or("");
    if name.is_empty() || name.len() > 128 || name.contains(['/', '\0']) {
        return failed(AppError::new(
            ErrorCode::InvalidInput,
            "Invalid folder name",
        ));
    }
    match transact(expected, |config| {
        let folders = config
            .pointer("/front/buttons")
            .and_then(Value::as_object)
            .ok_or_else(|| AppError::new(ErrorCode::InvalidInput, "Invalid folders"))?;
        if folders.contains_key(name) {
            return Err(AppError::new(ErrorCode::Conflict, "Folder already exists"));
        }
        if !folders.contains_key(parent) {
            return Err(AppError::new(
                ErrorCode::InvalidInput,
                "Parent folder does not exist",
            ));
        }
        Ok(create_folders(
            config,
            &[json!({"name":name,"parent_folder":parent})],
        ))
    }) {
        Ok(s) => saved(s),
        Err(e) => failed(e),
    }
}
