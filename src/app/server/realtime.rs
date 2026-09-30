//! Versioned Socket.IO commands recheck identity at each invocation.
use super::{
    security::{self, Identity},
    v2, AppState,
};
use crate::adapters::platform::usage::get_usage;
use crate::domain::error::{AppError, ErrorCode};
use axum::{extract::ConnectInfo, response::Json};
use serde_json::{json, Value};
use socketioxide::extract::{Data, SocketRef};
use socketioxide::handler::ConnectHandler;
use std::net::SocketAddr;

pub(crate) async fn usage() -> Json<Value> {
    Json(get_usage(None, &[]))
}

fn socket_identity(socket: &SocketRef, token: Option<&str>) -> Result<Identity, AppError> {
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
    security::authorize(token, local)
}
fn token(auth: &Value) -> Option<String> {
    match auth.get("token") {
        None if auth.is_object() => None,
        Some(Value::String(token)) => Some(token.clone()),
        // A supplied malformed credential must not inherit loopback trust.
        _ => Some(String::new()),
    }
}

pub(crate) fn socketio_layer(
    _server_address: &str,
    _server_port: u16,
    state: AppState,
) -> socketioxide::layer::SocketIoLayer {
    let (layer, io) = socketioxide::SocketIo::builder()
        .max_payload(65536)
        .build_layer();
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
                        let identity = socket_identity(&socket, token.as_deref())?;
                        let request: v2::CommandRequest = serde_json::from_value(data)
                            .map_err(|_| AppError::new(ErrorCode::InvalidInput, "Invalid command request"))?;
                        if !v2::valid_request_id(&id) { return Err(AppError::new(ErrorCode::InvalidInput, "Invalid request identifier")); }
                        let command = state.executor.parse(&request.message)?;
                        state.executor.execute_with_admission(command, identity.capabilities, || {
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
        socket_identity(&socket, token(&auth).as_deref()).map(|_| ())
    }));
    layer
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn malformed_socket_credentials_are_not_absent_credentials() {
        assert_eq!(token(&json!({})), None);
        for auth in [json!({"token":null}), json!({"token":42}), json!([])] {
            assert_eq!(token(&auth), Some(String::new()));
        }
    }
}
