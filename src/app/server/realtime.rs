//! Legacy adapters retain HTTP results versus Socket.IO original-message echo.
use super::{
    security::{self, Identity},
    v2, AppState,
};
use crate::app::buttons::usage::get_usage;
use crate::domain::error::{AppError, ErrorCode};
use axum::{
    extract::{ConnectInfo, State},
    response::{IntoResponse, Json, Response},
    Extension,
};
use serde_json::{json, Value};
use socketioxide::extract::{Data, SocketRef};
use socketioxide::handler::ConnectHandler;
use std::net::SocketAddr;

pub(crate) async fn usage() -> Json<Value> {
    Json(get_usage(None, &[]))
}

fn socket_identity(
    socket: &SocketRef,
    token: Option<&str>,
    legacy: bool,
) -> Result<Identity, AppError> {
    if !security::origin_allowed(&socket.req_parts().headers) {
        return Err(AppError::new(
            ErrorCode::Forbidden,
            "Cross-origin requests are denied",
        ));
    }
    let local = security::is_local(
        socket
            .req_parts()
            .extensions
            .get::<ConnectInfo<SocketAddr>>()
            .map(|c| c.0.ip()),
    );
    security::authorize(token, local, legacy)
}
fn token(auth: &Value) -> Option<String> {
    auth.get("token")
        .and_then(Value::as_str)
        .map(str::to_string)
}

pub(crate) fn socketio_layer(
    _server_address: &str,
    _server_port: u16,
    state: AppState,
) -> socketioxide::layer::SocketIoLayer {
    let (layer, io) = socketioxide::SocketIo::builder()
        .max_payload(65536)
        .build_layer();
    let legacy_state = state.clone();
    io.ns(
        "/",
        (move |socket: SocketRef, Data::<Value>(auth)| {
            let state = legacy_state.clone();
            let identity_token = token(&auth);
            async move {
                socket.on("send", {
                    let token = identity_token.clone();
                    move |socket: SocketRef, Data::<Value>(data)| {
                        let token = token.clone();
                        async move {
                            if socket_identity(&socket, token.as_deref(), true).is_ok() {
                                let _ = socket.broadcast().emit("message", &data).await;
                            } else {
                                let _ = socket.disconnect();
                            }
                        }
                    }
                });
                socket.on(
                    "message_from_socket",
                    move |socket: SocketRef, Data::<Value>(data)| {
                        let state = state.clone();
                        let token = identity_token.clone();
                        async move {
                            let identity = match socket_identity(&socket, token.as_deref(), true) {
                                Ok(identity) => identity,
                                Err(_) => {
                                    let _ = socket.disconnect();
                                    return;
                                }
                            };
                            let message = data
                                .as_str()
                                .map(str::to_string)
                                .unwrap_or_else(|| data.to_string());
                            let result = match state.executor.parse(&message) {
                                Ok(command) => {
                                    state
                                        .executor
                                        .execute(command, identity.capabilities, false)
                                        .await
                                }
                                Err(error) => Err(error),
                            };
                            if result.is_ok() {
                                let _ = socket.emit("json_data", &message);
                            } else {
                                let _ = socket.emit(
                                    "command_error",
                                    &json!({"success":false,"message":"Command rejected"}),
                                );
                            }
                        }
                    },
                );
            }
        })
        .with(|socket: SocketRef, Data::<Value>(auth)| async move {
            socket_identity(&socket, token(&auth).as_deref(), true).map(|_| ())
        }),
    );
    io.ns("/v2", (move |socket: SocketRef, Data::<Value>(auth)| {
        let state = state.clone();
        let identity_token = token(&auth);
        async move {
            socket.on("command", move |socket: SocketRef, Data::<Value>(data)| {
                let state = state.clone();
                let token = identity_token.clone();
                async move {
                    let id = data.get("request_id").and_then(Value::as_str).unwrap_or("");
                    let id = if id.is_empty() { v2::request_id() } else { id.to_string() };
                    let result = async {
                        let identity = socket_identity(&socket, token.as_deref(), false)?;
                        let request: v2::CommandRequest = serde_json::from_value(data)
                            .map_err(|_| AppError::new(ErrorCode::InvalidInput, "Invalid command request"))?;
                        if !v2::valid_request_id(&id) { return Err(AppError::new(ErrorCode::InvalidInput, "Invalid request identifier")); }
                        let command = state.executor.parse(&request.message)?;
                        state.executor.execute_with_admission(command, identity.capabilities, true, || {
                            let _ = socket.emit("command_result", &json!({"api_version":2,"request_id":id,"state":"accepted"}));
                        }).await
                    }.await;
                    let payload = match result {
                        Ok(result) if result.get("success").and_then(Value::as_bool) != Some(false) => json!({"api_version":2,"request_id":id,"state":"completed","result":result}),
                        Ok(_) => json!({"api_version":2,"request_id":id,"state":"failed","code":"execution_failed","message":"Command execution failed"}),
                        Err(error) => json!({"api_version":2,"request_id":id,"state":"failed","code":error.code,"message":error.message}),
                    };
                    let _ = socket.emit("command_result", &payload);
                }
            });
        }
    }).with(|socket: SocketRef, Data::<Value>(auth)| async move {
        socket_identity(&socket, token(&auth).as_deref(), false).map(|_| ())
    }));
    layer
}

pub(crate) async fn send_data_route(
    State(state): State<AppState>,
    Extension(identity): Extension<Identity>,
    Json(body): Json<Value>,
) -> Response {
    let message = body.get("message").and_then(Value::as_str).unwrap_or("");
    let result = match state.executor.parse(message) {
        Ok(command) => {
            state
                .executor
                .execute(command, identity.capabilities, false)
                .await
        }
        Err(error) => Err(error),
    };
    match result {
        Ok(result) => Json(result).into_response(),
        Err(error) => v2::error_response(error, None),
    }
}
