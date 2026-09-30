//! Versioned transport DTOs do not mutate legacy response shapes.
use std::sync::atomic::{AtomicU64, Ordering};

pub(crate) use crate::domain::transport::{CommandRequest, ConfigRequest, DeviceRequest};
use axum::{
    extract::{rejection::JsonRejection, Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    Extension, Json,
};
use serde_json::{json, Value};

use super::{
    security::{self, Identity},
    AppState,
};
use crate::application::{config, sessions};
use crate::domain::{
    command::{Capability, BUILTINS},
    error::{AppError, ErrorCode},
};

static NEXT_REQUEST: AtomicU64 = AtomicU64::new(1);
pub(crate) fn request_id() -> String {
    format!(
        "{}-{}",
        std::process::id(),
        NEXT_REQUEST.fetch_add(1, Ordering::Relaxed)
    )
}

pub(crate) fn error_response(error: AppError, id: Option<String>) -> Response {
    let status = match error.code {
        ErrorCode::InvalidInput | ErrorCode::UnsupportedSchema | ErrorCode::UnknownCommand => {
            StatusCode::BAD_REQUEST
        }
        ErrorCode::Unauthorized => StatusCode::UNAUTHORIZED,
        ErrorCode::Forbidden => StatusCode::FORBIDDEN,
        ErrorCode::Conflict => StatusCode::CONFLICT,
        ErrorCode::CapacityExhausted => StatusCode::TOO_MANY_REQUESTS,
        ErrorCode::ShuttingDown => StatusCode::SERVICE_UNAVAILABLE,
        ErrorCode::ExecutionFailed | ErrorCode::PersistenceFailed => {
            StatusCode::INTERNAL_SERVER_ERROR
        }
    };
    (status, Json(json!({"api_version":2,"request_id":id.unwrap_or_else(request_id),"state":"failed","code":error.code,"message":error.message}))).into_response()
}

pub(crate) fn valid_request_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 128
        && id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"-_:.".contains(&b))
}

pub(crate) async fn command(
    State(state): State<AppState>,
    Extension(identity): Extension<Identity>,
    request: Result<Json<CommandRequest>, JsonRejection>,
) -> Response {
    let request = match request {
        Ok(Json(request)) => request,
        Err(_) => {
            return error_response(
                AppError::new(ErrorCode::InvalidInput, "Invalid JSON request"),
                None,
            )
        }
    };
    let id = request.request_id.unwrap_or_else(request_id);
    if !valid_request_id(&id) {
        return error_response(
            AppError::new(ErrorCode::InvalidInput, "Invalid request identifier"),
            None,
        );
    }
    let parsed = match state.executor.parse(&request.message) {
        Ok(command) => command,
        Err(error) => return error_response(error, Some(id)),
    };
    match state
        .executor
        .execute(parsed, identity.capabilities, true)
        .await
    {
        Ok(result) => {
            let success = result.get("success").and_then(Value::as_bool) != Some(false);
            // Do not expose integration/native error strings that can contain
            // tokens, script bodies, private paths or upstream responses.
            if !success {
                return error_response(
                    AppError::new(ErrorCode::ExecutionFailed, "Command execution failed"),
                    Some(id),
                );
            }
            Json(json!({"api_version":2,"request_id":id,"state":"completed","result":result}))
                .into_response()
        }
        Err(error) => error_response(error, Some(id)),
    }
}

pub(crate) async fn catalog() -> Json<Value> {
    Json(json!({"api_version":2,"commands":BUILTINS}))
}

pub(crate) async fn get_config() -> Response {
    match config::shared(crate::app::utils::settings::get_config::get_config_path())
        .and_then(|s| s.snapshot())
    {
        Ok(snapshot) => {
            Json(json!({"api_version":2,"revision":snapshot.revision,"config":snapshot.config}))
                .into_response()
        }
        Err(error) => error_response(error, None),
    }
}

pub(crate) async fn save_config(
    State(state): State<AppState>,
    request: Result<Json<ConfigRequest>, JsonRejection>,
) -> Response {
    let request = match request {
        Ok(Json(request)) => request,
        Err(_) => {
            return error_response(
                AppError::new(ErrorCode::InvalidInput, "Invalid JSON request"),
                None,
            )
        }
    };
    match super::routes_config::transact(&state, Some(request.revision), |old| {
        super::routes_config::resize(request.config, &old)
    })
    .await
    {
        Ok(snapshot) => {
            crate::app::utils::global_variables::set_global_variable(
                "config",
                snapshot.config.clone(),
            );
            Json(json!({"api_version":2,"revision":snapshot.revision,"config":snapshot.config}))
                .into_response()
        }
        Err(error) => error_response(error, None),
    }
}

pub(crate) async fn approve_device(
    Extension(identity): Extension<Identity>,
    request: Result<Json<DeviceRequest>, JsonRejection>,
) -> Response {
    let request = match request {
        Ok(Json(request)) => request,
        Err(_) => {
            return error_response(
                AppError::new(ErrorCode::InvalidInput, "Invalid JSON request"),
                None,
            )
        }
    };
    if let Err(error) = security::require(&identity, Capability::Admin) {
        return error_response(error, None);
    }
    match sessions::shared()
        .and_then(|s| s.approve(request.name, request.capabilities, request.ttl_seconds))
    {
        Ok((device, token)) => {
            Json(json!({"api_version":2,"device":device,"token":token})).into_response()
        }
        Err(error) => error_response(error, None),
    }
}

pub(crate) async fn list_devices(Extension(identity): Extension<Identity>) -> Response {
    if let Err(error) = security::require(&identity, Capability::Admin) {
        return error_response(error, None);
    }
    match sessions::shared() {
        Ok(service) => Json(json!({"api_version":2,"devices":service.list()})).into_response(),
        Err(error) => error_response(error, None),
    }
}

pub(crate) async fn revoke_device(
    Extension(identity): Extension<Identity>,
    Path(id): Path<String>,
) -> Response {
    if let Err(error) = security::require(&identity, Capability::Admin) {
        return error_response(error, None);
    }
    match sessions::shared().and_then(|s| s.revoke(&id)) {
        Ok(()) => Json(json!({"api_version":2,"revoked":true})).into_response(),
        Err(error) => error_response(error, None),
    }
}
